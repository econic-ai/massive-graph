//! Optimised Index scaffold: two-tier facade API (snapshot + delta + reserved slots)

use core::marker::PhantomData;
use std::sync::Arc;
use arc_swap::ArcSwap;
use crate::structures::optimised_index::radix_delta::RadixDelta;
use crate::structures::segmented_stream::{SegmentedStream as SegStream, Cursor as SegCursor};

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
pub struct OptimisedIndexGen<K: Clone, V, I: MphIndexer<K>> {
    /// Current published snapshot (reserved + base arrays and MPH meta).
    snapshot: ArcSwap<SnapshotGen<K, V, I>>, 
    /// Append-only delta stream carrying upserts and deletes.
    #[allow(dead_code)]
    delta_stream: Arc<DeltaStream<K, V>>, 
    /// In-memory delta overlay (tombstone=None) — placeholder RadixDelta
    delta: RadixDelta<K, V>,
    /// Bloom filter to skip delta probes on likely misses (lock-free)
    bloom: Arc<super::optimised_index::bloom::DeltaBloom>, 
    _pd: PhantomData<(K, V)>,
}

// Implementations live in index_impl.rs

/// MPH indexer trait to map keys to base array indices.
/// Implementations should be pure and fast; called on every base lookup.
pub trait MphIndexer<K>: Send + Sync {
    /// Evaluate the MPH index for the provided key.
    fn eval(&self, key: &K) -> usize;
}

/// Adapter to allow dynamic indexers while using a monomorphized field type.
pub struct ArcIndexer<K>(pub Arc<dyn MphIndexer<K>>);
impl<K> MphIndexer<K> for ArcIndexer<K> {
    #[inline]
    fn eval(&self, key: &K) -> usize { self.0.eval(key) }
}

/// Snapshot holds reserved and base arrays and MPH meta.
pub struct SnapshotGen<K, V, I: MphIndexer<K>> {
    /// Monotonic snapshot version.
    pub version: u64,
    /// Reserved schema keys in slot order.
    pub reserved_keys: Arc<[K]>,
    /// Reserved schema values aligned to reserved_keys.
    pub reserved_vals: Arc<[Arc<V>]>,
    /// Base MPH values array.
    pub mph_vals: Arc<[Arc<V>]>,
    /// MPH indexer for base lookups.
    pub mph_indexer: I, 
}

/// Backward-compatible aliases using a dyn-backed indexer wrapper.
pub type OptimisedIndex<K, V> = OptimisedIndexGen<K, V, ArcIndexer<K>>;
pub type Snapshot<K, V> = SnapshotGen<K, V, ArcIndexer<K>>;

/// Delta operation types carried in the delta stream.
pub enum DeltaOp<K, V> {
    /// Insert or update the value associated with a key.
    Upsert(K, V),
    /// Remove a key by writing a tombstone.
    Delete(K),
}

/// Type alias for the delta stream.
pub type DeltaStream<K, V> = SegStream<DeltaOp<K, V>>;

/// Type alias for a cursor over the delta stream.
pub type DeltaCursor<K, V> = SegCursor<DeltaOp<K, V>>;

mod optimised_index;
mod radix_delta;
mod arena;
mod bloom;

#[cfg(test)]
mod tests;
