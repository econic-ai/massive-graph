use crate::debug_log;

use super::mph_indexer::MphIndexer;
use super::util::hash64;
use core::marker::PhantomData;

/// Immutable MPH index slot containing key and value.
/// Packed for cache efficiency (no alignment padding).
pub struct Slot<K, V> {
    /// Fingerprint: middle 16 bits of key hash for quick rejection (0 when empty).
    pub tag16: u16,
    /// Fingerprint: full 64-bit key hash for validation (0 when empty).
    pub hash64: u64,
    /// The key stored in this slot.
    pub key: K,
    /// The value stored in this slot.
    pub value: V,
}

impl<K, V> Slot<K, V> {
    /// Create a new slot with key and value.
    #[inline]
    pub fn new(tag16: u16, hash64: u64, key: K, value: V) -> Self {
        Self { tag16, hash64, key, value }
    }
}

/// Immutable MPH index: array of slots with indexer for O(1) lookups.
/// All mutations happen in the radix index until publish time.
pub struct MPHIndex<K: Clone, V, I: MphIndexer<K>> {
    /// Contiguous slots; length equals MPH slot count.
    pub slots: Vec<Slot<K, V>>,
    /// MPH indexer for evaluating key -> slot index.
    pub indexer: I,
    /// Phantom data for key type.
    _pd: PhantomData<K>,
}

impl<K, V, I: MphIndexer<K>> MPHIndex<K, V, I>
where
    K: Clone + Eq + std::hash::Hash + std::fmt::Debug,
    V: Clone + std::fmt::Debug,
{
    /// Create an empty MPH index with the given indexer.
    pub fn empty(indexer: I) -> Self {
        Self {
            slots: Vec::new(),
            indexer,
            _pd: PhantomData,
        }
    }
    
    /// Create MPH index from slots and indexer.
    pub fn from_slots(slots: Vec<Slot<K, V>>, indexer: I) -> Self {
        Self { slots, indexer, _pd: PhantomData }
    }

    /// Get value by key.
    #[inline]
    pub fn get<'a>(&'a self, key: &K) -> Option<&'a V> {
        let h = hash64(key);
        self.get_with_hash(key, h)
    }

    /// Get value by key with pre-computed hash (avoids redundant hashing).
    #[inline]
    pub fn get_with_hash<'a>(&'a self, key: &K, hash: u64) -> Option<&'a V> {
        let idx = self.indexer.eval(key);
        if idx >= self.slots.len() {
            return None;
        }
        
        let slot = &self.slots[idx];
        
        // Quick rejection using fingerprint
        if slot.hash64 != hash {
            debug_log!("....Slot hash64({:?}) != hash({:?})...recalculating hash geives us {}", slot.hash64, hash, hash64(key));
            return None;
        }
        
        // Verify key match (optional, can be disabled for performance)
        // if slot.key != *key {
        //     return None;
        // }
        
        Some(&slot.value)
    }


    /// Iterate all values.
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &'a V> + 'a {
        self.slots.iter().map(|slot| &slot.value)
    }

    /// Get number of slots in the index.
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Check if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Get reference to the indexer.
    pub fn indexer(&self) -> &I {
        &self.indexer
    }

    /// Get reference to slots (for migration/compatibility).
    pub fn slots(&self) -> &[Slot<K, V>] {
        &self.slots
    }
}
