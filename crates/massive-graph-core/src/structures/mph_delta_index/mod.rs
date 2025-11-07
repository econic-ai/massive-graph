//! MPH + Delta index scaffold: two-tier facade API (snapshot + delta)
use core::marker::PhantomData;
use std::sync::Mutex;
use crossbeam_epoch as epoch;
use crate::structures::segmented_stream::{segmented_stream::StreamIndex, SegmentedStream};
use crate::structures::mph_delta_index::mph_index::MPHIndex;
use crate::structures::mph_delta_index::radix_index::RadixIndex;
use crate::structures::mph_delta_index::mph_indexer::{BBHashIndexer, MphIndexer};

/// Minimal stats structure for the index.
pub struct OptimisedIndexStats {
    /// Number of entries in the MPH overlay (base slots).
    pub len_base: usize,
    /// Number of entries in the delta tier (overrides present).
    pub len_delta: usize,
}

impl Default for OptimisedIndexStats {
    fn default() -> Self {
        Self { len_base: 0, len_delta: 0 }
    }
}

/// Minimal facade for the index (generic over indexer type for monomorphization).
pub struct OptimisedIndexGen<K: Clone,  V, I: MphIndexer<K>> {
    /// Immutable MPH index containing keys and StreamIndex values (epoch::Atomic for RCU updates).
    /// Replaced entirely on publish. Contains the indexer internally.
    pub(crate) mph_index: epoch::Atomic<MPHIndex<K, StreamIndex<V>, I>>,
    /// STEP 2: Restored real SegmentedStream (with epoch::Atomic for active_page)
    pub(crate) stream: SegmentedStream<V>,
    /// STEP 1: Restored real RadixIndex with proper capacity
    pub(crate) radix_index: RadixIndex<K, StreamIndex<V>>,
    /// Bloom filter to skip probes on unlikely keys (interior mutability via AtomicU64).
    pub(crate) bloom: super::mph_delta_index::bloom::DeltaBloom,
    /// Serialize cutover actions (e.g., overlay rebuild) without blocking readers.
    pub(crate) consolidate_lock: Mutex<()>,
    pub(crate) _pd: PhantomData<(K, V, I)>,
}

// Manual Debug implementation since Atomic and Mutex don't derive Debug easily
impl<K: Clone, V, I: MphIndexer<K>> std::fmt::Debug for OptimisedIndexGen<K, V, I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OptimisedIndexGen")
            .field("mph_index", &"<Atomic<MPHIndex>>")
            .field("stream", &"<SegmentedStream>")
            .field("radix_index", &"<RadixIndex>")
            .field("bloom", &"<DeltaBloom>")
            .finish()
    }
}

/// MPH indexer
pub mod mph_indexer;

/// Default optimised index using BBHashIndexer (monomorphized at compile time).
/// For custom indexers, use OptimisedIndexGen<K, V, YourIndexer> directly.
/// All indexers implement MphIndexer trait and are compiled with zero abstraction overhead.
pub type OptimisedIndex<K, V> = OptimisedIndexGen<K, V, BBHashIndexer<K>>;

/// Probabilistic delta membership filter used to skip unlikely delta probes.
pub mod bloom;
/// MPH overlay array with per-slot publish protocol.
pub mod mph_index;
/// Index facade wiring snapshot + delta + stream.
pub mod optimised_index; // implementation
/// Radix-hash delta overlay supporting upserts and deletes.
// pub mod radix_delta; // deprecated
/// Radix-hash delta overlay supporting upserts and deletes.
pub mod radix_index;
/// Next-state radix index with fixed-capacity buckets and mask-driven visibility.
pub mod radix_index_v2;
/// Re-export of the tiny open-addressed map used by the radix index buckets.
pub mod tiny_map;
/// Utility functions used by the radix index buckets.
pub mod util;
/// Statistics and diagnostics for radix index.
pub mod radix_stats;
/// Arena allocator for colocated Buffer+Recs+TinyMap allocations.
pub mod arena;
/// Debug logging macros (zero-overhead when disabled).
#[macro_use]
pub mod debug_macros;
/// Epoch defer tracking for diagnostics.
pub mod epoch_tracker;

// Consolidation summary removed in overlay-only model.

// Transitional module: re-export current index while we migrate naming and policy.

/// Test module for debugging MPH index building (in module root for easy testing).
mod mph_build_test;

#[cfg(test)]
mod tests {
    mod tests;
    mod mph_indexer_tests;
    mod mph_indexer_nondeterminism_test;
    mod diagnostic_tests;
    mod radix_index_tests;
    mod memory_footprint_tests;
    mod benchmark_replication_tests;
}
