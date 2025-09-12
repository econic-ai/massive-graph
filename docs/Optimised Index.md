# OptimisedIndex: Dual‑Tier Index (Radix Delta + MPH Base)

**Module target:** `core::structures::optimised_index`

**Goal:** Implement a reusable, ultra‑low‑latency index for 16‑byte IDs (e.g., `u128`) that supports high‑speed lookups and practical writes via a two‑tier design:

* **Base (immutable):** Minimal Perfect Hash (MPH) snapshot for collision‑free O(1) reads.
* **Delta (mutable):** Two‑level radix hash (directory by top *k* bits → tiny open‑address buckets) for fast inserts/updates/deletes (tombstones).
* **Routing:** Bloom‑guarded + EWMA‑driven adaptive order (MPH‑first when delta is cold; delta‑first when hot).
* **Hot‑swap:** Rebuild MPH in background and publish via `ArcSwap`.

> **Out of scope:** persistence, wire mapping, metrics beyond a stub (leave hook points). Focus on structure, concurrency, and API.

---

## 1) Design Overview

### 1.1 Architecture

* **`Snapshot` (Base):** Holds MPH function metadata + value array + optional 16‑bit fingerprint array for membership verification.
* **`RadixDelta` (Delta):** Sharded two‑level radix table with per‑bucket `RwLock` and Robin‑Hood open addressing (OA). Supports **tombstones**.
* **`OptimisedIndex<K,V>`:** Facade that routes `get/put/delete`, tracks EWMA hit‑rate, owns Bloom filter over delta keys, and coordinates consolidation.

### 1.2 Lookup Routing (fast path)

* Maintain a Bloom filter for the **delta**. On `get(id)`:

  1. **Bloom miss** → skip delta; evaluate **MPH** directly.
  2. **Bloom maybe** → probe **delta**; if miss, fall through to **MPH**.
* Maintain an EWMA of delta hit rate; flip a single atomic **routing mode** flag with hysteresis (e.g., flip to delta‑first when hit‑rate > 20%, flip back when < 10%). Even in delta‑first mode, keep Bloom guard to avoid pointless probes.

### 1.3 Consolidation (background)

* Trigger when `delta_len ≥ 5% of base_len` (configurable). Use a global `seqno` to select a **cut** so concurrent writes after the cut stay in delta.
* Build next MPH from `(base − tombstones) ∪ delta_adds_updates (≤ cut)`.
* Publish new `Snapshot` with `ArcSwap`; prune merged delta entries (≤ cut).

---

## 2) Key Concepts to Apply

### 2.1 Robin‑Hood Open Addressing (OA)

* OA with linear probing but on insert we **prefer moving entries with shorter probe length out of the way** ("steal from the rich, give to the poor"). This caps the maximum probe distance and tightens tail latency.

### 2.2 Bucket‑Local Split (Extendible Hashing)

* If a bucket gets full/hot, **split only that bucket** by increasing its local depth by 1 and redistributing entries based on the next bit. This avoids global rehashes.

### 2.3 Fingerprints

* Store a compact per‑slot fingerprint (e.g., 8 bits for delta buckets, 16 bits for base). Compare fingerprint before doing the full `u128` equality. Cuts memory traffic and branches.

---

## 3) Public API (generic, reusable)

```rust
pub trait OptimisedIndex<K, V>: Send + Sync {
    fn get(&self, key: &K) -> Option<V>;
    fn upsert(&self, key: K, value: V);
    fn delete(&self, key: &K);

    /// Manual consolidation hint (optional; background worker may call automatically)
    fn maybe_consolidate(&self);

    /// Minimal stats stub (extend later)
    fn stats(&self) -> OptimisedIndexStats;
}

pub struct OptimisedIndexStats {
    pub base_version: u64,
    pub len_base: usize,
    pub len_delta: usize,
    pub routing_mode_delta_first: bool,
}
```

> **Notes**
>
> * `V` is generic; callers should pass `Arc<T>` if they want shared immutable values. The index does not impose `Arc` internally.
> * `K` initially targets `u128`. Keep the trait generic but provide a default impl for `u128`.

---

## 4) Module Layout

```
massive-graph-core/src/
  structures/
    optimised_index/
      mod.rs
      radix_delta.rs
      snapshot_mph.rs
      bloom.rs
      ewma.rs
      routing.rs
      builder.rs        // consolidation pipeline
      fingerprints.rs
      types.rs          // common types, errors, config
      tests/
```

---

## 5) Types & Config

```rust
// types.rs
pub type SeqNo = u64;

#[derive(Clone)]
pub struct Config {
    /// Directory bits for two‑level radix (2^k buckets)
    pub radix_k_bits: u8,        // default: 16

    /// Bucket capacity bounds (Robin‑Hood OA array length)
    pub bucket_cap_min: usize,   // default: 16
    pub bucket_cap_max: usize,   // default: 32

    /// Bloom filter target false‑positive rate
    pub bloom_fpr: f64,          // default: 0.005 (0.5%)

    /// Consolidation trigger: delta_len >= trigger_percent of base_len
    pub consolidate_trigger_percent: f64, // default: 5.0

    /// EWMA smoothing factor and hysteresis thresholds
    pub ewma_alpha: f64,         // default: 0.2
    pub ewma_hi: f64,            // default: 0.20
    pub ewma_lo: f64,            // default: 0.10
}

pub enum RoutingMode { DeltaFirst, BaseFirst }
```

Provide a `Config::default()` with the values above, and allow overrides at construction.

---

## 6) Snapshot (MPH Base)

```rust
// snapshot_mph.rs
pub struct Snapshot<K, V> {
    pub version: u64,
    pub mph: Box<dyn MphFunction<K> + Send + Sync>,
    pub values: Arc<[V]>,
    pub fp16: Option<Arc<[u16]>>, // optional 16‑bit fingerprints
}

pub trait MphFunction<K>: Send + Sync {
    fn eval(&self, key: &K) -> usize; // 0..n-1
}

pub struct SnapshotHandle<K, V> {
    inner: arc_swap::ArcSwap<Snapshot<K, V>>, // hot‑swap base
}
```

**Builder contract:**

* `builder::build_snapshot(base_iter, delta_iter, tombstones, version_next) -> Snapshot<K,V>`
* Implementation may call an external MPH builder or an in‑house one. Keep the interface generic.

**Membership check:**

* If `fp16` is present, verify `fp16[idx] == fp16_of(key)` after `eval`. If mismatch → treat as miss (fall back to delta if routing chose base first).

---

## 7) RadixDelta (Two‑Level Radix)

```rust
// radix_delta.rs
pub struct RadixDelta<K, V> {
    cfg: Config,
    dir: Vec<Bucket<K, V>>,      // length = 1 << radix_k_bits
    // optional sharding can be added later; per‑bucket lock suffices for now
}

struct Bucket<K, V> {
    lock: parking_lot::RwLock<BucketInner<K, V>>,
}

struct BucketInner<K, V> {
    // Robin‑Hood OA arrays (SoA)
    keys: Vec<K>,
    vals: Vec<DeltaEntry<V>>, // Val(value, seq) | Tombstone(seq)
    ctrl: Vec<u8>,            // 0 = empty, else 8‑bit fingerprint
    cap: usize,
    len: usize,
    local_depth: u8,          // for bucket‑local split
}

pub enum DeltaEntry<V> { Val(V, SeqNo), Tombstone(SeqNo) }
```

**Operations:**

* `get(&self, key) -> Option<&DeltaEntry<V>>` (read lock; short probe using fingerprint → full compare)
* `upsert(&self, key, value, seq)` (write lock; Robin‑Hood insert; split on full)
* `tombstone(&self, key, seq)`
* `iter_cut(&self, cut_seq) -> iterator` over entries with `seq <= cut` (used by builder)
* `prune_cut(&self, cut_seq)` removes merged entries

**Fingerprints:**

* `fp8 = hash8(key)` for delta; used in `ctrl` to skip most unequal slots before loading the key.

**Bucket‑Local Split:**

* When full, allocate a sibling bucket, increment `local_depth`, redistribute entries by inspecting the next bit beyond the directory depth for that bucket. Update directory pointers for the affected range only.

---

## 8) Bloom + EWMA Routing

```rust
// bloom.rs
pub struct DeltaBloom { /* bitvec + k hash fns */ }
impl DeltaBloom {
    pub fn new(expected_n: usize, fpr: f64) -> Self { /* m,k by standard formulas */ }
    pub fn insert(&self, key: &u128);
    pub fn might_contain(&self, key: &u128) -> bool;
}

// ewma.rs
pub struct Ewma { alpha: f64, value: f64 }
impl Ewma { pub fn update(&mut self, sample: f64) { /* v = alpha*s + (1-alpha)*v */ } }

// routing.rs
pub struct Router { mode: AtomicBool /* delta_first? */, ewma: Mutex<Ewma>, hi: f64, lo: f64 }
impl Router { /* flip with hysteresis */ }
```

Routing decision in `get()`:

* If `delta_first`:

  * Bloom check → delta probe on maybe; on miss, go base.
* If `base_first`:

  * Bloom check → if miss, go base directly; else delta probe first.
* Update EWMA with hit=1.0 when delta served, else 0.0; router flips when thresholds crossed.

---

## 9) Facade: `OptimisedIndex` Implementation

```rust
// mod.rs
pub struct OptimisedIndexImpl<K, V> {
    cfg: Config,
    router: Router,
    bloom: DeltaBloom,
    delta: RadixDelta<K, V>,
    base: SnapshotHandle<K, V>,
    seqno: AtomicU64,
}

impl<K, V> OptimisedIndex<K, V>
where
    K: Copy + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new(cfg: Config, base: Snapshot<K, V>) -> Self { /* init bloom from delta scan */ }
    pub fn get(&self, key: &K) -> Option<V> { /* routing logic (see §8) */ }
    pub fn upsert(&self, key: K, value: V) { let s = self.seqno.fetch_add(1, SeqCst); /* delta.upsert + bloom.insert */ }
    pub fn delete(&self, key: &K) { let s = self.seqno.fetch_add(1, SeqCst); /* delta.tombstone + bloom.insert */ }
    pub fn maybe_consolidate(&self) { /* check 5% rule and spawn builder */ }
}
```

**Consolidation worker (builder.rs):**

* Read `base` snapshot and `delta.iter_cut(cut)`.
* Build next MPH & arrays; attach `fp16` if configured.
* `base.store(new_snapshot)` via `ArcSwap`.
* `delta.prune_cut(cut)`.

---

## 10) Configuration Constants (make overridable)

* `RADIX_K_BITS_DEFAULT = 16`
* `BUCKET_CAP_MIN_DEFAULT = 16`
* `BUCKET_CAP_MAX_DEFAULT = 32`
* `BLOOM_FPR_DEFAULT = 0.005`
* `CONSOLIDATE_TRIGGER_PERCENT_DEFAULT = 5.0`
* `EWMA_ALPHA_DEFAULT = 0.2`
* `EWMA_HI_DEFAULT = 0.20`
* `EWMA_LO_DEFAULT = 0.10`

Expose via `Config` and sensible `Default`.

---

## 11) Error Handling & Edge Cases

* **Duplicates across tiers:** delta always wins. MPH membership check via `fp16` prevents accidental wrong hits.
* **Tombstones:** delta masks base immediately; removed at prune after publish.
* **Resizes:** bucket‑local split only; keep probe distance bounded (Robin‑Hood).
* **Flapping routing:** use hysteresis thresholds (hi/lo) to prevent rapid flipping.

---

## 12) Tests (outline)

* **Correctness:** insert/get/delete across tiers; tombstones; consolidation keeps semantics.
* **Routing:** force delta hot → ensure mode flips; then cold → flips back.
* **Concurrency:** multi‑threaded upserts/gets; no lost updates around consolidation cut.
* **Capacity:** buckets split correctly; probe distance bounded.
* **Membership:** unknown keys never return base values when `fp16` enabled.

Provide Criterion benches placeholders (opt‑in) for `get()` latency under different delta/base ratios.

---

## 13) Implementation Notes

* Prefer **SoA** layout for buckets: fewer cachelines touched.
* Align directory & buckets to cacheline (64B) boundaries; consider prefetch on directory→bucket.
* Keep MPH metadata compact; place `values` and `fp16` in contiguous `Arc<[T]>`.
* Use `parking_lot` locks for low overhead.
* Use `ArcSwap` for snapshot hot‑swap; never mutate a published snapshot.

---

## 14) TODO / Hooks (future work)

* Persistence (mmap snapshot + delta; recover on startup).
* Metrics surface (prometheus): hit‑rates, false‑positive rate, bucket split counts, consolidation durations.
* NUMA‑aware sharding when cardinality grows.
* Optional cuckoo variant for delta if write tails become an issue (likely not needed).

---

## 15) Acceptance Criteria

* `OptimisedIndexImpl<u128, V>` compiles and passes unit tests.
* Lookup routes via Bloom + EWMA and returns correct values with duplicates across tiers.
* Consolidation builds a fresh MPH snapshot and swaps without blocking reads; delta is pruned up to cut.
* Bucket splits are local; no global rehash.
* Configurable constants honored via `Config`.

---

### Notes to Cursor

* Keep the code **generic** and **reusable**. Where external crates are obvious (e.g., `arc_swap`, `parking_lot`), use them; otherwise prefer internal minimal implementations (Bloom, EWMA) to keep dependencies light.
* Avoid copy‑pasting caller‑specific types. The caller will pass `Arc<T>` values when sharing immutable documents; your API should not enforce `Arc`.
* Leave clear TODOs where persistence/metrics would slot in.
