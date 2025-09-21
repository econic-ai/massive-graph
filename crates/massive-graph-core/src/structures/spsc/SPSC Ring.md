### SPSC Ring (Single-Producer Single-Consumer)

A bounded, lock-free SPSC ring buffer with power-of-two capacity. Designed for one producer thread and one consumer thread with cache-friendly layout and minimal synchronization.

#### Guarantees
- Exactly one producer and one consumer; behavior is undefined with multiple producers/consumers.
- Bounded capacity; non-power-of-two inputs are rounded up to the next power of two.
- No blocking; push returns Err(item) if full, pop returns None if empty.
- Publication: producer uses Release; consumer uses Acquire to see initialized items.

#### API
```rust
let ring = SpscRing::<T>::with_capacity_pow2(1024);

// Producer
for item in items { let _ = ring.push(item); }

// Consumer
while let Some(item) = ring.pop() { /* use item */ }

// State helpers
let full = ring.is_full();
let empty = ring.is_empty();
```

#### Architecture
- Buffer: `Box<[UnsafeCell<MaybeUninit<T>>]>` of length `capacity`.
- Indices: `head` (next write), `tail` (next read), both `AtomicUsize` with padding to avoid false sharing.
- Memory ordering:
  - Producer: read `tail` (Acquire) for full check; write slot; store `head` (Release).
  - Consumer: read `head` (Acquire) for empty check; read slot; store `tail` (Release).
- Wraparound: `idx = counter & (capacity - 1)` using `mask`.

#### Tests
- [x] Basic push/pop single-thread order
- [x] Full ring behavior (Err on push when full, then succeed after pop)
- [x] Empty ring behavior (None on pop)
- [x] Wraparound with small capacities (2, 4, 8)
- [x] SPSC concurrency (1 prod, 1 cons) with N items
- [x] is_full/is_empty correctness across transitions
- [x] Capacity rounding to power-of-two

#### Benchmarks
- Producer-only push until full (throughput)
- Consumer-only pop until empty (throughput)
- Steady-state SPSC throughput (1P/1C) for capacities: 64, 256, 4096
- Push-pop latency per pair in steady-state
- Burst patterns: producer/consumer bursts (B = 4, 32, 256)

## Benchmarks

The following iterations pair each code change with its measured performance. Time/op figures are derived from total time and operation count.

#### Iteration 0 — Original (baseline)
No optimizations.

- Producer-only (fill from empty to full; ops = capacity)

| Capacity | Median total time | Time/op | Ops/s |
|---:|---:|---:|---:|
| 64 | 92.179 ns | 1.44 ns | 0.70 G/s |
| 256 | 217.40 ns | 0.85 ns | 1.18 G/s |
| 4096 | 6.4375 µs | 1.57 ns | 0.64 G/s |

- Consumer-only (drain from full; ops = capacity)

| Capacity | Median total time | Time/op | Ops/s |
|---:|---:|---:|---:|
| 64 | 64.517 ns | 1.01 ns | 0.99 G/s |
| 256 | 134.51 ns | 0.53 ns | 1.90 G/s |
| 4096 | 1.5136 µs | 0.37 ns | 2.71 G/s |

- Steady-state (1P/1C; ops = capacity × 100)

| Capacity | Median total time | Time/transfer | Transfers/s |
|---:|---:|---:|---:|
| 64 | 89.100 µs | 13.95 ns | 71.7 M/s |
| 256 | 249.15 µs | 9.73 ns | 102.7 M/s |
| 4096 | 9.3313 ms | 22.79 ns | 43.9 M/s |

Summary: Solid baseline; large ring is best here.

#### Iteration 1 — Inlining + unchecked indexing + 64B padding
Changes (detailed):
- Added `#[inline(always)]` to hot methods: `with_capacity_pow2`, `push`, `pop`, `is_full`, `is_empty` (reduces call overhead, improves codegen).
- Replaced indexed buffer access with `get_unchecked` inside `push`/`pop` (removes bounds checks; safe under SPSC invariants and mask indexing).
- Introduced real padding (64 bytes) and `#[repr(C)]` to separate `head` and `tail` onto distinct cache lines on 64B machines.

Rationale: Lower per-op overhead and reduce false sharing between producer/consumer index atomics.

- Steady-state

| Capacity | Median total time | Time/transfer | Transfers/s |
|---:|---:|---:|---:|
| 64 | 57.035 µs | 8.90 ns | 112.4 M/s |
| 256 | 139.49 µs | 5.45 ns | 183.6 M/s |
| 4096 | 20.870 ms | 50.41 ns | 19.8 M/s |

Summary: 64/256 faster; 4096 regressed due to 128B cacheline on Apple Silicon (64B spacing insufficient).

#### Iteration 2 — 128B padding (platform-aware)
Changes (detailed):
- Made cacheline size platform-aware: 128B on macOS aarch64, 64B elsewhere.
- Ensured `head` and `tail` reside on separate 128B-aligned lines on Apple Silicon (prevents same-line sharing).

Rationale: Avoid cacheline ping‑pong on platforms with 128B L1 lines.

- Steady-state

| Capacity | Median total time | Time/transfer | Transfers/s |
|---:|---:|---:|---:|
| 64 | 53.244 µs | 8.32 ns | 120.1 M/s |
| 256 | 136.31 µs | 5.32 ns | 187.9 M/s |
| 4096 | 20.427 ms | 49.89 ns | 20.0 M/s |

Summary: Small/medium improve slightly; 4096 ~flat (coherence remains dominant).

#### Iteration 3 — Batched publication (B = 32)
Changes (detailed):
- Added batched producer/consumer handles `split_batched_owned::<B>()` returning `SpscProducer<T, B>` / `SpscConsumer<T, B>`.
- Producer writes up to B items, then issues a single `head.store(Release)` (reduces invalidations by B×).
- Consumer reads up to B items, then issues a single `tail.store(Release)`.
- Provided `flush()` on both ends and RAII flushing on drop to publish remaining items.

Rationale: Amortize index publication and minimize coherence traffic in steady-state, especially with larger capacities.

- Producer-only

| Capacity | Median total time | Time/op | Ops/s |
|---:|---:|---:|---:|
| 64 | 87.343 ns | 1.36 ns | 0.73 G/s |
| 256 | 215.27 ns | 0.84 ns | 1.19 G/s |
| 4096 | 6.9376 µs | 1.69 ns | 0.59 G/s |

- Consumer-only

| Capacity | Median total time | Time/op | Ops/s |
|---:|---:|---:|---:|
| 64 | 62.317 ns | 0.97 ns | 1.03 G/s |
| 256 | 129.13 ns | 0.50 ns | 2.00 G/s |
| 4096 | 1.5390 µs | 0.38 ns | 2.66 G/s |

- Steady-state (B = 32)

| Capacity | Median total time | Time/transfer | Transfers/s |
|---:|---:|---:|---:|
| 64 | 43.476 µs | 6.79 ns | 147.3 M/s |
| 256 | 85.384 µs | 3.33 ns | 300.0 M/s |
| 4096 | 1.0147 ms | 2.48 ns | 403.6 M/s |

Summary: Batched publication eliminates index ping‑pong; 4096 becomes fastest. 64/256 also improve further.
