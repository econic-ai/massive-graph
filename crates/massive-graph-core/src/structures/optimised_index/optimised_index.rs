use super::bloom::DeltaBloom;
use super::radix_delta::RadixDelta;
use super::*;
use std::hash::Hasher;

impl<K, V> OptimisedIndex<K, V>
where
    K: Clone + Eq + std::hash::Hash + 'static,
    V: Clone + 'static,
{
    /// Create a new index from a snapshot and a delta stream.
    pub fn new(snapshot: Snapshot<K, V>, delta_stream: Arc<DeltaStream<K, V>>) -> Self {
        Self {
            snapshot: ArcSwap::from_pointee(snapshot),
            delta_stream,
            delta: RadixDelta::new(),
            bloom: Arc::new(DeltaBloom::with_capacity(1024, 0.01)),
            _pd: PhantomData,
        }
    }

    /// Get a value by key with delta overlay; placeholder returns base only.
    pub fn get(&self, key: &K) -> Option<Arc<V>> {
        // Fused hash: compute once for Bloom + RadixDelta
        let mut hasher = ahash::AHasher::default();
        key.hash(&mut hasher);
        let seed = hasher.finish();

        // Bloom guard first (prehashed)
        if self.bloom.might_contain_prehashed(seed) {
            if let Some(v) = self.delta.get_hashed(key, seed) { return v; }
        }
        // Bloom miss: skip delta entirely
        let snap = self.snapshot.load();
        let idx = snap.mph_indexer.eval(key);
        snap.mph_vals.get(idx).cloned()
    }

    /// Load the current snapshot once for reuse (hoist outside tight loops).
    #[inline]
    pub fn load_snapshot_arc(&self) -> Arc<Snapshot<K, V>> { self.snapshot.load_full() }

    /// Get using a provided snapshot reference (hoisted); does Bloom→delta, else base via `snap`.
    #[inline]
    pub fn get_with_snapshot(&self, snap: &Snapshot<K, V>, key: &K) -> Option<Arc<V>> {
        // Fused hash once
        let mut hasher = ahash::AHasher::default();
        key.hash(&mut hasher);
        let seed = hasher.finish();
        if self.bloom.might_contain_prehashed(seed) {
            if let Some(v) = self.delta.get_hashed(key, seed) { return v; }
        }
        let idx = snap.mph_indexer.eval(key);
        snap.mph_vals.get(idx).cloned()
    }

    /// Reserved get using a provided snapshot (hoisted); base-only fast path.
    #[inline]
    pub fn get_reserved_with_snapshot(&self, snap: &Snapshot<K, V>, slot: usize) -> Option<Arc<V>> {
        snap.reserved_vals.get(slot).cloned()
    }

    /// Upsert directly into overlay and update Bloom for immediate visibility.
    pub fn upsert(&self, key: K, value: V) {
        // Delta stores V directly
        self.delta.upsert(key.clone(), value);
        // let mut h = ahash::AHasher::default();
        // key.hash(&mut h);
        // self.bloom.insert_prehashed(h.finish());
    }

    /// Base length.
    pub fn len(&self) -> usize {
        self.snapshot.load().mph_vals.len()
    }

    /// Iterate over base entries (placeholder without keys).
    pub fn iter(&self) -> impl Iterator<Item = (&K, Arc<V>)> {
        std::iter::empty()
    }

    /// Stats (placeholder zeros except base len).
    pub fn stats(&self) -> OptimisedIndexStats {
        let s = self.snapshot.load();
        OptimisedIndexStats {
            base_version: s.version,
            len_base: s.mph_vals.len(),
            len_delta: 0,
            routing_mode_delta_first: false,
        }
    }

    /// Remove a key by writing a tombstone into overlay and mark in Bloom.
    pub fn remove(&self, key: &K) {
        self.delta.delete(key);
        let mut h = ahash::AHasher::default();
        key.hash(&mut h);
        self.bloom.insert_prehashed(h.finish());
    }

    /// Existence (placeholder false).
    pub fn contains_key(&self, key: &K) -> bool {
        if let Some(v) = self.delta.get(key) { return v.is_some(); }
        let snap = self.snapshot.load();
        let idx = snap.mph_indexer.eval(key);
        snap.mph_vals.get(idx).is_some()
    }

    /// Base-only by MPH index.
    pub fn get_by_index(&self, idx: usize) -> Option<Arc<V>> {
        self.snapshot.load().mph_vals.get(idx).cloned()
    }

    /// Reserved slot fast-path: base-only vector access by default.
    /// If you need overlay for reserved, call get_reserved_slot_with_overlay.
    pub fn get_reserved_slot(&self, slot: usize) -> Option<Arc<V>> {
        self.snapshot.load().reserved_vals.get(slot).cloned()
    }

    /// Reserved slot with delta overlay using reserved_keys[slot]. Slower than base-only.
    pub fn get_reserved_slot_with_overlay(&self, slot: usize) -> Option<Arc<V>> {
        let snap = self.snapshot.load();
        if let Some(k) = snap.reserved_keys.get(slot) {
            if let Some(v) = self.delta.get(k) {
                return v;
            }
        }
        snap.reserved_vals.get(slot).cloned()
    }

    /// Create delta cursor (placeholder unimplemented).
    pub fn create_delta_cursor(&self) -> DeltaCursor<K, V> {
        SegCursor::new_at_head(&self.delta_stream)
    }
}

impl<K, V> OptimisedIndex<K, V>
where
    K: Clone + Eq + std::hash::Hash + 'static,
    V: Clone + 'static,
{
    /// Publish a new snapshot atomically (testing/helper API).
    pub fn publish_snapshot(&self, snapshot: Snapshot<K, V>) {
        self.snapshot.store(Arc::new(snapshot));
    }

    /// Append a delta upsert into the stream (applier will materialize into delta overlay).
    pub fn append_delta_upsert(&self, key: K, value: V) {
        let _ = self
            .delta_stream
            .append(DeltaOp::Upsert(key.clone(), value));
        let mut h = ahash::AHasher::default();
        key.hash(&mut h);
        self.bloom.insert_prehashed(h.finish());
    }

    /// Append a delta delete into the stream (applier will materialize into delta overlay).
    pub fn append_delta_delete(&self, key: K) {
        let _ = self.delta_stream.append(DeltaOp::Delete(key.clone()));
        let mut h = ahash::AHasher::default();
        key.hash(&mut h);
        self.bloom.insert_prehashed(h.finish());
    }

    /// Apply up to `max_ops` from the delta cursor into the in-memory delta overlay.
    pub fn apply_delta_once(&self, cursor: &mut DeltaCursor<K, V>, max_ops: usize) -> usize {
        let mut applied = 0usize;
        while applied < max_ops {
            if let Some(op) = cursor.next() {
                match op {
                    DeltaOp::Upsert(ref k, ref v) => self.delta.upsert(k.clone(), v.clone()),
                    DeltaOp::Delete(ref k) => self.delta.delete(k),
                }
                applied += 1;
            } else {
                break;
            }
        }
        applied
    }
}
