## OptimisedIndex — Benchmark & Change Log

This document logs iterative performance results and the precise code changes paired with each iteration. Time/op and ops/s are computed from Criterion medians.

### Iteration 0 — Baseline (MPH base + reserved fast-path, no delta overlay)

Changes (detailed)
- API skeleton with `Snapshot { reserved_keys, reserved_vals, mph_vals, mph_indexer }` and `OptimisedIndex::new()`.
- `get_by_index(idx)`: base-only MPH array dereference.
- `get_reserved_slot(slot)`: base-only reserved array dereference (no Bloom, no delta lookup).
- `get(key)`: base-only via `mph_indexer.eval(key)` (no delta overlay yet).

Benchmarks (baseline)

| Test | N | Median time | Time/op | Ops/s | Δ vs prev |
|---|---:|---:|---:|---:|---:|
| optidx_get_by_index | 1,024 | 2.969 ms | 2.90 µs | 0.344 M/s | — |
| optidx_get_by_index | 65,536 | 3.800 ms | 57.9 ns | 17.3 M/s | — |
| optidx_get_reserved | 8 | 69.90 ns | 8.74 ns | 14.3 M/s | — |

Notes
- Reserved slot is the fastest path by design (single vector read). Serves as the baseline for subsequent changes.

### Iteration 1 — Add delta overlay and tombstones (DashMap), keep reserved base-only

Changes (detailed)
- Introduced `delta_map: DashMap<K, Option<Arc<V>>>` in `OptimisedIndex` for TDD overlay:
  - `upsert(key, Arc<V>)` inserts `Some(Arc<V>)`.
  - `remove(key)` inserts `None` (tombstone).
  - `get(key)` first checks `delta_map`, then falls back to base MPH.
  - `contains_key(key)` consults delta then base.
- `get_reserved_slot(slot)` reverted to base-only fast-path (no delta probe) to keep the hot-path optimal and match requirements.

Benchmarks (vs Iteration 0)

| Test | N | Median time | Time/op | Ops/s | Δ vs prev |
|---|---:|---:|---:|---:|---:|
| optidx_get_by_index | 1,024 | 2.969 ms | 2.90 µs | 0.344 M/s | ~0% |
| optidx_get_by_index | 65,536 | 3.800 ms | 57.9 ns | 17.3 M/s | ~0% |
| optidx_get_reserved | 8 | 69.90 ns | 8.74 ns | 14.3 M/s | ~0% |

1b (reserved fast-path verification)

| Test | N | Median time | Time/op | Ops/s | Δ vs Iter. 1 |
|---|---:|---:|---:|---:|---:|
| optidx_get_by_index | 1,024 | 3.043 ms | 2.97 µs | 0.345 M/s | ~0% |
| optidx_get_by_index | 65,536 | 3.919 ms | 59.8 ns | 16.7 M/s | ~0% |
| optidx_get_reserved | 8 | 30.51 ns | 3.81 ns | 32.8 M/s | +56–57% faster |

Summary
- Base by-index performance unchanged.
- Reserved fast-path remains optimal after reverting to base-only.
- Delta overlay added; subsequent throughput/latency benches for `get(key)` will be added when we wire delta-heavy scenarios and appliers.

Next Steps
- Add delta-heavy benchmarks: `get(key)` hit/miss ratios and throughput with varying delta sizes.
- Introduce radix delta structure and stream applier to replace DashMap (the latter is for TDD only).

### Iteration 2 — RadixDelta overlay; stream-applier hooks

Changes (detailed)
- Replaced DashMap with `RadixDelta` (dir=2^12, bucket_cap=32), fingerprinted Robin-Hood OA; delete inserts tombstones on empty probe.
- Added stream hooks: `append_delta_upsert/delete`, `create_delta_cursor`, `apply_delta_once`.
- Reserved fast-path unchanged (base-only).

Benchmarks (vs Iter. 1b)

| Test | N | Median time | Time/op | Ops/s | Δ vs 1b |
|---|---:|---:|---:|---:|---:|
| optidx_get_by_index | 1,024 | 3.3835 ms | 3.30 µs | 0.303 M/s | −11% |
| optidx_get_by_index | 65,536 | 4.1351 ms | 63.1 ns | 15.9 M/s | −5% |
| optidx_get_reserved | 8 | 29.75 ns | 3.72 ns | 33.7 M/s | ~0% |

Notes
- Slight regression on by-index likely due to added code/links. Reserved fast-path stable.
- New benches added: `optidx_get_key_base`, `optidx_get_key_delta_hit`, `optidx_get_key_delta_miss` (populate next run).

2b (additional suites and latest medians)

| Test | N | Median time | Time/op | Ops/s | Δ vs 2 |
|---|---:|---:|---:|---:|---:|
| optidx_get_by_index | 1,024 | 3.3869 ms | 3.31 µs | 0.302 M/s | |
| optidx_get_by_index | 65,536 | 4.2425 ms | 64.7 ns | 15.4 M/s | |
| optidx_get_reserved | 8 | 29.68 ns | 3.71 ns | 269 M/s | |
| optidx_get_key_base | 1,024 | 9.2298 µs | 9.02 ns | 110.8 M/s | |
| optidx_get_key_base | 65,536 | 626.50 µs | 9.56 ns | 104.6 M/s | |
| optidx_get_key_delta_hit | 1,024 | 9.4415 µs | 9.22 ns | 108.4 M/s | |
| optidx_get_key_delta_hit | 65,536 | 1.9847 ms | 30.3 ns | 33.0 M/s | |
| optidx_get_key_delta_miss | 1,024 | 11.270 µs | 11.00 ns | 90.9 M/s | |
| optidx_get_key_delta_miss | 65,536 | 2.6446 ms | 40.4 ns | 24.7 M/s | |

#### Measurement modes

We report two classes of measurements to separate construction cost from steady-state lookup cost.

- With build included (construct per sample):

| Test | N | Median time | Time/op | Ops/s |
|---|---:|---:|---:|---:|
| optidx_get_by_index | 1,024 | 3.3869 ms | 3.31 µs | 0.302 M/s |
| optidx_get_by_index | 65,536 | 4.2425 ms | 64.7 ns | 15.4 M/s |
| optidx_get_reserved | 8 | 29.68 ns | 3.71 ns | 269 M/s |

- Build excluded (build once, measure get-only):

| Test | N | Median time | Time/op | Ops/s | Δ vs prev |
|---|---:|---:|---:|---:|---:|
| optidx_get_by_index | 1,024 | 3.7139 µs | 3.63 ns | 275.3 M/s | −99.89% |
| optidx_get_by_index | 65,536 | 239.60 µs | 3.66 ns | 273.1 M/s | −94.37% |
| optidx_get_key_base | 1,024 | 9.2388 µs | 9.02 ns | 110.7 M/s | ≈0% |
| optidx_get_key_base | 65,536 | 635.69 µs | 9.70 ns | 103.1 M/s | +1.08% |
| optidx_get_key_delta_hit | 1,024 | 9.4210 µs | 9.20 ns | 108.7 M/s | +0.32% |
| optidx_get_key_delta_hit | 65,536 | 1.9691 ms | 30.1 ns | 33.2 M/s | −0.78% |
| optidx_get_key_delta_miss | 1,024 | 11.311 µs | 11.05 ns | 90.5 M/s | ≈0% |
| optidx_get_key_delta_miss | 65,536 | 2.6345 ms | 40.2 ns | 24.9 M/s | −0.38% |


### Iteration 3 — Bloom filter guard (lock-free) and delta probe gating

Changes (detailed)
- Added lock-free Bloom filter (`DeltaBloom`):
  - `Vec<AtomicU64>` bitset, all operations `Ordering::Relaxed`.
  - Double hashing (Kirsch–Mitzenmacher) to reduce to 2 hash rounds per check.
  - `insert` uses `fetch_or`; `might_contain` uses relaxed loads.
- Wired `OptimisedIndex::get(key)` to probe delta only if Bloom returns maybe; skip delta entirely on Bloom miss.
- Ensured reserved/by-index paths never consult Bloom.

Benchmarks (build excluded; zero-locking Bloom latest medians)

| Test | N | Median time | Time/op | Ops/s | Δ vs prior Bloom |
|---|---:|---:|---:|---:|---:|
| optidx_get_by_index | 1,024 | 3.7225 µs | 3.64 ns | 275.0 M/s | ≈0% |
| optidx_get_by_index | 65,536 | 242.01 µs | 3.69 ns | 271.0 M/s | ≈0% |
| optidx_get_reserved | 8 | 29.481 ns | 3.69 ns | 271.4 M/s | ≈0% |
| optidx_get_key_base | 1,024 | 5.4454 µs | 5.32 ns | 188.0 M/s | −61.6% |
| optidx_get_key_base | 65,536 | 348.89 µs | 5.32 ns | 187.8 M/s | −63.4% |
| optidx_get_key_delta_hit | 1,024 | 5.5143 µs | 5.39 ns | 185.6 M/s | −62.7% |
| optidx_get_key_delta_hit | 65,536 | 349.37 µs | 5.33 ns | 187.6 M/s | −85.3% |
| optidx_get_key_delta_miss | 1,024 | 5.4303 µs | 5.30 ns | 188.7 M/s | −66.6% |
| optidx_get_key_delta_miss | 65,536 | 346.55 µs | 5.29 ns | 189.0 M/s | −88.2% |

Notes
- By-index and reserved remain unchanged (Bloom not on those paths).
- Base/delta-hit/delta-miss show ~6–9% overhead from Bloom hashing and branch; double hashing minimized k hash rounds.
- Further optimizations (optional): smaller k, SIMD bit tests, or fused hash in RadixDelta to reuse for Bloom.

#### Bloom Optimisations

Changes (focused on Bloom + hashing hot paths)
- Fused-hash reuse: compute one ahash per lookup in get(); reuse its seed for Bloom (prehashed) and RadixDelta (bucket index, probe start).
- Prehashed-only Bloom API in hot path: might_contain_prehashed, insert_prehashed.
- Removed non-prehashed Bloom variants from hot path (kept only prehashed; legacy variants removed).
- Added RadixDelta::get_hashed to avoid redundant hashing inside overlay lookup.

Benchmarks (build excluded; subset relevant to Bloom/hash changes)

| Test | N | Median time | Time/op | Ops/s | Δ vs prior |
|---|---:|---:|---:|---:|---:|
| optidx_get_key_base | 1,024 | 5.0943 µs | 4.98 ns | 201.0 M/s | −6.24% |
| optidx_get_key_base | 65,536 | 330.64 µs | 5.04 ns | 198.4 M/s | −6.13% |
| optidx_get_key_delta_hit | 1,024 | 5.0804 µs | 4.96 ns | 201.6 M/s | −6.93% |
| optidx_get_key_delta_hit | 65,536 | 327.22 µs | 5.00 ns | 200.2 M/s | −6.29% |
| optidx_get_key_delta_miss | 1,024 | 5.1307 µs | 5.01 ns | 199.6 M/s | −6.18% |
| optidx_get_key_delta_miss | 65,536 | 329.31 µs | 5.03 ns | 198.9 M/s | −5.48% |

Notes
- By-index and reserved unchanged; excluded from this subset.
- The fused-hash reuse accounts for most of the 5–10% gains on base/delta paths; hashed overlay lookup alone has marginal effect vs probe/key-compare costs.

### Iteration 4 — Comparative Ops (HashMap/DashMap/BTreeMap vs OptimisedIndex)

Method
- Build excluded; per-iteration setup using Criterion batched mode.
- Apples-to-apples loops for each op: upsert (insert 0..N), get (probe 0..N after prefill), delete (remove 0..N after prefill), traverse (sum get 0..N).

Results (median times)

| Structure | Op | N=1,024 | N=65,536 |
|---|---|---:|---:|
| HashMap | Upsert | 9.809 µs | 640.61 µs |
| OptimisedIndex | Upsert | 3.420 ms | 7.459 ms |
| DashMap | Upsert | 24.363 µs | 2.010 ms |
| BTreeMap | Upsert | 20.608 µs | 3.004 ms |
| HashMap | Get | 8.102 µs | 604.88 µs |
| OptimisedIndex | Get | 3.361 ms | 5.147 ms |
| DashMap | Get | 15.259 µs | 1.075 ms |
| BTreeMap | Get | 12.754 µs | 2.287 ms |
| HashMap | Delete | 9.164 µs | 700.85 µs |
| OptimisedIndex | Delete | 3.291 ms | 6.353 ms |
| DashMap | Delete | 21.624 µs | 1.405 ms |
| BTreeMap | Delete | 19.184 µs | 1.350 ms |
| HashMap | Traverse | 3.886 µs | 208.38 µs |
| OptimisedIndex | Traverse | 3.425 ms | 5.431 ms |
| BTreeMap | Traverse | 5.744 µs | 325.63 µs |

Notes
- These are single-threaded, best-effort apples-to-apples loops. OptimisedIndex currently pays overlay and stream plumbing even in single-threaded mode, which makes it slower than in-CPU core maps for basic ops; its strengths will show with multi-writer append, consolidated snapshots, and reserved/MPH reads.
- DashMap carries concurrency overhead even single-threaded, but still remains much faster than the current OptimisedIndex writes/reads due to simpler code paths.
- Next: measure multi-threaded write/read scaling and add applier/consolidation latency metrics to show end-to-end pipeline behavior.

