//! Optimised Index scaffold: two-tier facade (placeholder)

use core::marker::PhantomData;

/// Minimal stats structure for the optimised index.
pub struct OptimisedIndexStats {
    /// Version of the base (snapshot) index.
    pub base_version: u64,
    /// Number of entries in the base tier.
    pub len_base: usize,
    /// Number of entries in the delta tier.
    pub len_delta: usize,
    /// Whether routing prefers delta first.
    pub routing_mode_delta_first: bool,
}

impl Default for OptimisedIndexStats {
    fn default() -> Self {
        Self { base_version: 0, len_base: 0, len_delta: 0, routing_mode_delta_first: true }
    }
}

/// Minimal facade for the optimised index (placeholder implementation).
pub struct OptimisedIndex<K, V> {
    _pd: PhantomData<(K, V)>,
}

impl<K, V> OptimisedIndex<K, V>
where
    V: Clone,
{
    /// Create a new empty index.
    pub fn new() -> Self { Self { _pd: PhantomData } }

    /// Get a value by key (placeholder: always None).
    pub fn get(&self, _key: &K) -> Option<V> { None }

    /// Upsert a key/value (placeholder: no-op).
    pub fn upsert(&self, _key: K, _value: V) {}

    /// Get the number of entries in the index.
    pub fn len(&self) -> usize { 0 }

    /// Iterate over the entries in the index.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> { std::iter::empty() }

    /// Retrieve simple stats.
    pub fn stats(&self) -> OptimisedIndexStats { OptimisedIndexStats::default() }

    /// Remove a key/value (placeholder: no-op).
    pub fn remove(&self, _key: &K) {}

    /// Check if a key exists (placeholder: always false).
    pub fn contains_key(&self, _key: &K) -> bool { false }

}
