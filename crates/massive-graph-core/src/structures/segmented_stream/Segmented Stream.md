### Segmented Stream (Append-only, Lock-free-ish)

This structure provides an append-only segmented array ("stream") composed of fixed-size pages. It supports many-writer append and many-cursor reads without locks on the hot path. Reads observe only fully published entries.

#### Key properties
- Append-only pages of `entries: [MaybeUninit<T>; ENTRIES_PER_PAGE]` (no drops of `T` on reuse).
- Multi-writer: reservation via `claimed.fetch_add(1, Relaxed)`; publication via `committed.fetch_add(1, Release)`.
- Single-link guarantee: page linking uses CAS on `next` (Release); readers hop with Acquire.
- `active_page` is a hint: writers and cursors always follow `next` if present. Stores are Relaxed.
- Optional page pool, recycler, and prefiller to trade memory vs latency.

#### Minimal API
```rust
// Create
let stream: Stream<MyType> = Stream::new();

// Append (multi-writer safe)
stream.append(item)?;

// Iterate from head
let mut c = Cursor::new_at_head(&stream);
while let Some(v) = c.next() {
    // use *v
}

// Batch read from current page (contiguous slice of committed items)
let slice = c.next_batch();
```

#### Internal Architecture
- Page layout (hot fields first): `claimed, committed, next, readers, entries[..]` (64B aligned).
- Append path:
  1) Reserve slot `i = claimed.fetch_add(1, Relaxed)`.
  2) Write `entries[i]`.
  3) Publish `committed.fetch_add(1, Release)`.
  4) If page is full, link a new page via CAS on `next` (Release); winner updates `active_page` (hint).
  5) Early linking: first writer at half capacity attempts to pre-link next to reduce boundary contention.
- Read path:
  - Load `committed` with Acquire; read only `0..committed`.
  - If `committed == ENTRIES_PER_PAGE`, h∏op to `next` with Acquire.
  - Cursor index is a plain `u32` (no concurrent cursor mutation expected).

#### Memory Ordering
- Reservation: `claimed` with Relaxed (index only).
- Publication: `committed` with Release; readers load with Acquire.
- Page linking: CAS `next` with Release; readers/hoppers load `next` with Acquire.
- `active_page`: Relaxed loads/stores (hint only).

#### Pooling Options
- Simple fixed pool: `StreamPagePool` returns pre-initialized pages (round-robin).
- Recycler thread (optional): moves full, reader-free pages to a ready ring (off hot path).
- Prefiller thread (optional): allocates and pre-resets pages to keep a ready ring topped up.

### Performance Evolution (high level)
Changes applied over time and their impact (kept when net-positive):

- Hot-field reordering in `Page<T>` (claimed/committed/next/readers before entries): improves cache locality (kept).
- Read-path Arc reductions (avoid extra clones, hop via raw + strong-count inc): large read speedup (kept).
- `active_page` treated as hint; Relaxed store: reduces contention without affecting correctness (kept).
- Early linking at half page: reduces boundary CAS spikes on rollover (kept, neutral-to-small gain in most cases).
- Fixed-size pool (12 pages): improves single-writer by reducing allocations (kept as option).
- Pre-reset pool on hot path: regressed (dropped in favor of prefiller/recycler options).
- Recycler thread: helpful for memory reuse, not faster than hot-path reset (optional, not for latency wins).
- Prefiller thread: faster than recycler/pre-reset pool, still below baseline when rollovers are rare (optional).

### Current Benchmarks (production page size)
Latest median times and throughputs (ops/s). Throughput = N / time.

| Benchmark | N | Median time | Time/op | Ops/s |
|---|---:|---:|---:|---:|
| stream_append_single | 10,000 | 1.2163 ms | 121.6 ns | 8.22 M/s |
| stream_append_single | 100,000 | 1.8125 ms | 18.1 ns | 55.2 M/s |
| stream_append_multi (4×25k total 100k) | 100,000 | 11.563 ms | 115.6 ns | 8.65 M/s |
| stream_iter_read | 100,000 | 34.186 µs | 0.342 ns | 2.93 G/s |
| stream_iter_read | 1,000,000 | 345.27 µs | 0.345 ns | 2.90 G/s |
| stream_append_single_prereset_pool | 100,000 | 21.771 ms | 217.7 ns | 4.59 M/s |
| stream_append_with_recycler | 100,000 | 22.130 ms | 221.3 ns | 4.52 M/s |
| stream_append_with_prefiller | 100,000 | 8.372 ms | 83.7 ns | 11.95 M/s |

Notes:
- Tests use a small page size under `cfg(test)`. Benches run with production page size (`ENTRIES_PER_PAGE = PAGE_SIZE`).
- Single-writer append benefits from pooling and linking-hints; multi-writer is dominated by boundary CAS contention.
- The read path is extremely fast due to contiguous per-page slices and minimized synchronization.

### Rationale Summary
- Strict Release/Acquire points guarantee readers never see uninitialized entries.
- Treating `active_page` as advisory avoids unnecessary contention; correctness flows from the `next` chain.
- Early linking amortizes rollover work and smooths multi-writer spikes.
- Optional background threads trade memory for latency by removing allocation/reset from the hot path.


