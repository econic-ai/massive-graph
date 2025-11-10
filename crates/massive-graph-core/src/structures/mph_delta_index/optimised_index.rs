use super::bloom::DeltaBloom;
use super::mph_index::{MPHIndex, Slot};
use super::mph_indexer::BBHashIndexer;
use super::*;
use crossbeam_epoch as epoch;
use crossbeam_epoch::Owned;
use super::radix_index::RadixIndex;
use super::util::{hash64, tag16_from_hash};
use crate::structures::segmented_stream::{segmented_stream::StreamIndex, SegmentedStream};
use std::marker::PhantomData;

// Import debug macros
#[allow(unused_imports)]
use crate::{debug_log, debug_log_labeled, debug_block, debug_eval};

// Unified generic implementation for all indexer types (monomorphized at compile time).
// No dynamic dispatch, no vtables - pure compile-time polymorphism via trait bounds.
impl<K, V, I> OptimisedIndexGen<K, V, I>
where
    K: Clone + Eq + std::hash::Hash + std::fmt::Debug + 'static,
    V: Clone + std::fmt::Debug + 'static,
    I: MphIndexer<K>,
{
    /// Create an empty index with a custom indexer and specific radix capacities.
    /// The indexer must be pre-built with at least a dummy key set.
    pub fn new_with_indexer_and_capacity(
        indexer: I,
        radix_target_capacity: usize,
        radix_max_capacity: usize,
    ) -> Self {
        let stream = SegmentedStream::new();
        let radix_index = RadixIndex::with_capacity(radix_target_capacity, radix_max_capacity);
        let mph_index = epoch::Atomic::new(MPHIndex::empty(indexer));
        let bloom = DeltaBloom::with_capacity(radix_target_capacity, 0.01);
        
        Self {
            mph_index,
            stream,
            radix_index,
            bloom,
            consolidate_lock: Mutex::new(()),
            _pd: PhantomData,
        }
    }

    /// Create index with base keys and values.
    /// 
    /// # Deprecated
    /// This constructor is deprecated. Use `new_with_indexer_and_capacity` followed by
    /// `upsert` calls and then `publish` to build the MPH index.
    #[deprecated(since = "0.1.0", note = "Use new_with_indexer_and_capacity + upsert + publish instead")]
    pub fn new_with_base_keys_and_capacity(
        base_keys: &[K],
        base_vals: Vec<V>,
        indexer: I,
        radix_capacity: usize,
    ) -> Self {
        let stream = SegmentedStream::new(); // STEP 2: Restored real SegmentedStream
        
        let len = base_keys.len();
        
        // Append values to stream and create slots at correct indices
        // Pre-allocate vector with None to allow random-access placement by MPH indexer
        let mut slots: Vec<Option<Slot<K, StreamIndex<V>>>> = Vec::with_capacity(len);
        slots.resize_with(len, || None);
        for (key, val) in base_keys.iter().zip(base_vals.into_iter()) {
            let sidx = stream.append_with_index(val).expect("Failed to append to stream");
            let h = hash64(key);
            let tag16 = tag16_from_hash(h);
            let slot = Slot::new(tag16, h, key.clone(), sidx);
            
            // Place slot at the index the indexer expects
            let idx = indexer.eval(key);
            if idx >= slots.len() {
                panic!("Indexer returned out-of-bounds index {} for {} keys", idx, len);
            }
            slots[idx] = Some(slot);
        }
        
        // Convert Vec<Option<Slot>> to Vec<Slot> and verify all slots are filled
        let slots: Vec<Slot<K, StreamIndex<V>>> = slots.into_iter().enumerate().map(|(i, opt)| {
            opt.unwrap_or_else(|| panic!("Slot {} was not filled by indexer!", i))
        }).collect();

        let radix_max_capacity = radix_capacity * 4;
        let radix_index = RadixIndex::with_capacity(radix_capacity, radix_max_capacity);
        Self {
            mph_index: epoch::Atomic::new(MPHIndex::from_slots(slots, indexer)),
            stream,
            radix_index,
            bloom: DeltaBloom::with_capacity(radix_capacity, 0.01),
            consolidate_lock: Mutex::new(()),
            _pd: PhantomData,
        }
    }

    /// Publish: rebuild MPH index by folding in radix delta.
    /// Uses compile-time dispatch via I::build(&keys) - no vtable overhead.
    pub fn publish(&self) {
        if let Ok(_g) = self.consolidate_lock.try_lock() {
            let guard = epoch::pin();
            
            // Collect all keys and stream indices
            let mut entries: Vec<(K, StreamIndex<V>)> = Vec::new();
            
            // Add entries from current MPH (if any)
            let old_mph_ptr = self.mph_index.load(std::sync::atomic::Ordering::Acquire, &guard);
            if let Some(old_mph) = unsafe { old_mph_ptr.as_ref() } {
                for slot in old_mph.slots() {
                    if slot.hash64 == 0 { continue; }
                    let sidx = StreamIndex { page: slot.value.page, idx: slot.value.idx };
                    entries.push((slot.key.clone(), sidx));
                }
            }
            
            // Add/override with radix entries
            debug_log!("Publishing radix - starting iter {}", self.stats().len_delta);
            self.radix_index.consolidate_snapshots_only(&guard);         
            for (key, sidx) in self.radix_index.iter_with_keys(&guard) {
                entries.retain(|(k, _)| k != key);
                entries.push((key.clone(), *sidx));
            }
            debug_log!("Publishing radix - finished iter with entries {}", entries.len());
            
            // Build new indexer from merged keys (compile-time dispatch - monomorphized!)
            let keys: Vec<K> = entries.iter().map(|(k, _)| k.clone()).collect();
            let new_indexer = if !keys.is_empty() {
                I::build(&keys)
            } else {
                // No keys - keep old indexer (shouldn't normally happen)
                let old_mph_ptr = self.mph_index.load(std::sync::atomic::Ordering::Acquire, &guard);
                if let Some(old_mph) = unsafe { old_mph_ptr.as_ref() } {
                    old_mph.indexer().clone()
                } else {
                    // Fallback: build with empty key set (indexer-dependent behavior)
                    I::build(&keys)
                }
            };
            
            // Build new MPH slots - CRITICAL: place each slot at the index the indexer expects!
            // The indexer.eval(key) returns the slot index where this key should be stored.
            // Pre-allocate vector with None to allow random-access placement by MPH indexer
            let mut slots: Vec<Option<Slot<K, StreamIndex<V>>>> = Vec::with_capacity(entries.len());
            slots.resize_with(entries.len(), || None);
            for (key, sidx) in entries {
                let h = hash64(&key);
                let tag16 = tag16_from_hash(h);
                let slot = Slot::new(tag16, h, key.clone(), sidx);
                
                // Get the index where this key should be placed (from the indexer)
                let idx = new_indexer.eval(&key);
                if idx >= slots.len() {
                    panic!("Indexer returned out-of-bounds index {} for {} keys", idx, slots.len());
                }
                slots[idx] = Some(slot);
            }
            
            // Convert Vec<Option<Slot>> to Vec<Slot> and verify all slots are filled
            let slots: Vec<Slot<K, StreamIndex<V>>> = slots.into_iter().enumerate().map(|(i, opt)| {
                opt.unwrap_or_else(|| panic!("Slot {} was not filled by indexer!", i))
            }).collect();
            
            // Publish new MPH with the REBUILT indexer
            let new_mph = MPHIndex::from_slots(slots, new_indexer);
            let old = self.mph_index.swap(Owned::new(new_mph), std::sync::atomic::Ordering::AcqRel, &guard);
            if !old.is_null() {
                unsafe { guard.defer_unchecked(move || drop(old.into_owned())); }
            }
            
            // Clear radix
            self.radix_index.clear_all(&guard);
            
            // Reset bloom filter
            self.bloom.clear();
        }
    }

    /// Add a key to the bloom filter (used by upsert and delete).
    fn add_to_bloom(&self, hash: u64, _guard: &epoch::Guard) {
        self.bloom.insert_prehashed(hash);
    }

    /// Get value by key with explicit guard (returns reference tied to guard lifetime).
    /// Uses bloom filter to optimize radix lookup: checks radix first if bloom indicates key might be present.
    /// Computes hash once and reuses it across bloom/radix/MPH checks (saves ~12-30 CPU cycles).
    pub fn get<'g>(&'g self, key: &K, guard: &'g epoch::Guard) -> Option<&'g V> {
        let hash = hash64(key);
        
        // Check bloom filter - if key might be in radix, check radix first
        if self.bloom.might_contain_prehashed(hash) {
            // Bloom says key might be in radix - check radix (pass hash to avoid recomputation)
            if let Some(sidx) = self.radix_index.get_with_hash(key, hash, guard) {
                return Some(self.stream.resolve_ref_unchecked(sidx));
            }
            // Bloom false positive or key was deleted - fall through to MPH
        }
        
        // Check MPH index (base data) - pass hash to avoid recomputation
        self.get_mph_with_hash(key, hash, guard)
    }

    /// Get from radix
    pub fn get_radix<'g>(&'g self, key: &K, guard: &'g epoch::Guard) -> Option<&'g V> {
        if let Some(sidx) = self.radix_index.get(key, guard) {
            return Some(self.stream.resolve_ref_unchecked(&sidx));
        }
        None
    }
    

    /// Get from MPH index only.
    pub fn get_mph<'g>(&'g self, key: &K, guard: &'g epoch::Guard) -> Option<&'g V> {
        let hash = hash64(key);
        self.get_mph_with_hash(key, hash, guard)
    }

    /// Get from MPH index only with pre-computed hash (avoids redundant hashing).
    pub fn get_mph_with_hash<'g>(&'g self, key: &K, hash: u64, guard: &'g epoch::Guard) -> Option<&'g V> {
        let mph_ptr = self.mph_index.load(std::sync::atomic::Ordering::Acquire, guard);
        let mph = unsafe { mph_ptr.as_ref()? };
        
        // Delegate to MPHIndex.get_with_hash()
        let sidx = mph.get_with_hash(key, hash)?;
        
        // Resolve value from stream
        Some(self.stream.resolve_ref_unchecked(sidx))
    }

    /// Get from MPH index only.
    pub fn get_mph_index<'g>(&'g self, key: &K, guard: &'g epoch::Guard) -> Option<&'g StreamIndex<V>> {
        let mph_ptr = self.mph_index.load(std::sync::atomic::Ordering::Acquire, guard);
        let mph = unsafe { mph_ptr.as_ref()? };
        
        // Delegate to MPHIndex.get()
        mph.get(key)
    }    

   /// Get value by key using snapshot MPH index (no atomic load).
   pub fn get_mph_from_snapshot<'g>(&'g self, snapshot: &'g MPHIndex<K, StreamIndex<V>, I>, key: &K) -> Option<&'g V> {
        // Delegate to MPHIndex.get() which returns &StreamIndex<V>
        if let Some(sidx) = snapshot.get(key) {
            return Some(self.stream.resolve_ref_unchecked(sidx));
        }
        None
    }

    /// Get snapshot mph index slot.
    /// 
    /// Returns a reference to the slot that remains valid for the guard's lifetime.
    pub fn get_mph_index_from_snapshot<'g>(&'g self, snapshot: &'g MPHIndex<K, StreamIndex<V>, I>, key: &K) -> Option<&'g StreamIndex<V>> {
        // if let Some(sidx) = snapshot.get(key) {
        //     eprintln!("....StreamIndex Slot idx({:?})", sidx.idx);
        //     return Some(sidx);
        // } else {
        //     eprintln!("....StreamIndex Slot not found for key({:?})", key);
        // }
        // None
        snapshot.get(key)
    }

    /// Insert or update a key-value pair (goes to radix index).
    /// Also adds the key to the bloom filter for fast negative lookups.
    pub fn upsert(&self, key: K, val: V) {
        let guard = epoch::pin();
        let hash = hash64(&key);
        let sidx = self.stream.append_with_index(val).expect("Failed to append to stream");
        self.radix_index.upsert(&key, &sidx, &guard);  // Pass reference to StreamIndex<V>
        // debug_log!("Upserted to radix - size is {}", self.len());
        self.add_to_bloom(hash, &guard);
    }

    /// Remove a key (marks as deleted in radix index).
    /// Also adds the key to the bloom filter so we check radix for the tombstone.
    pub fn remove(&self, key: &K) {
        let guard = epoch::pin();
        let hash = hash64(key);
        self.radix_index.delete(key, &guard);
        self.add_to_bloom(hash, &guard);
    }

    /// Check if key exists.
    pub fn contains_key(&self, key: &K) -> bool {
        let guard = epoch::pin();
        
        // Check MPH first
        let mph_ptr = self.mph_index.load(std::sync::atomic::Ordering::Acquire, &guard);
        if let Some(mph) = unsafe { mph_ptr.as_ref() } {
            if mph.get(key).is_some() {
                return true;
            }
        }
        
        // Check radix - if we can get it, it exists
        self.radix_index.get(key, &guard).is_some()
    }

    /// Iterate all entries from MPH index (returns values only).
    pub fn iter_mph<'g>(&'g self, guard: &'g epoch::Guard) -> impl Iterator<Item = &'g V> + 'g {
        let ov_ptr = self.mph_index.load(std::sync::atomic::Ordering::Acquire, guard);
        let ov_ref = unsafe { ov_ptr.as_ref().unwrap() };
        // Delegate to MPHIndex.iter() and resolve from stream
        ov_ref.iter().map(move |sidx| self.stream.resolve_ref_unchecked(sidx))
    }

    /// Iterate all entries from MPH index (returns handles only).
    pub fn iter_mph_index<'g>(&'g self, guard: &'g epoch::Guard) -> impl Iterator<Item = &'g StreamIndex<V>> + 'g {
        let ov_ptr = self.mph_index.load(std::sync::atomic::Ordering::Acquire, guard);
        let ov_ref = unsafe { ov_ptr.as_ref().unwrap() };
        // Delegate to MPHIndex.iter()
        ov_ref.iter()
    }    

    /// Iterate all entries from radix index.
    pub fn iter_radix<'g>(&'g self, guard: &'g epoch::Guard) -> impl Iterator<Item = &'g V> + 'g {
        self.radix_index.iter(guard)
            .map(move |sidx| self.stream.resolve_ref_unchecked(sidx))  // sidx is &StreamIndex<V>
    }

    /// Iterate all values using snapshot MPH index (no atomic load).
    pub fn iter_mph_from_snapshot<'g>(&'g self, snapshot: &'g MPHIndex<K, StreamIndex<V>, I>) -> impl Iterator<Item = &'g V> + 'g {
        // snapshot.slots.iter().enumerate().map({
        //     move |(_slot_idx, slot)| {
        //         // Print slot address and StreamIndex address
        //         // let _slot_addr = slot as *const _ as usize;
        //         // let _sidx_addr = &slot.value as *const _ as usize;
        //         // let _sidx_page_addr = slot.value.page as usize;
                
        //         // debug_log!(
        //         //     "Slot[{}] @ 0x{:x} | StreamIndex @ 0x{:x} (offset: {}) | page: 0x{:x}, idx: {}",
        //         //     _slot_idx,
        //         //     _slot_addr,
        //         //     _sidx_addr,
        //         //     _sidx_addr - _slot_addr,  // Offset of StreamIndex within Slot
        //         //     _sidx_page_addr,
        //         //     slot.value.idx
        //         // );
                
        //         self.stream.resolve_ref_unchecked(&slot.value)
        //     }
        // })
        snapshot.slots.iter().map(move |slot | self.stream.resolve_ref_unchecked(&slot.value))        

    }

    /// Iterate stream indices using snapshot MPH index (no atomic load).
    pub fn iter_mph_index_from_snapshot<'g>(&'g self, snapshot: &'g MPHIndex<K, StreamIndex<V>, I>) -> impl Iterator<Item = &'g StreamIndex<V>> + 'g {
        // snapshot.slots.iter().enumerate().map({
        //     move |(_slot_idx, slot)| {
        //         // Print slot address and StreamIndex address
        //         let _slot_addr = slot as *const _ as usize;
        //         let _sidx_addr = &slot.value as *const _ as usize;
        //         let _sidx_page_addr = slot.value.page as usize;
                
        //         eprintln!(
        //             "Slot[{}] @ 0x{:x} | StreamIndex @ 0x{:x} (offset: {}) | page: 0x{:x}, idx: {}",
        //             _slot_idx,
        //             _slot_addr,
        //             _sidx_addr,
        //             _sidx_addr - _slot_addr,  // Offset of StreamIndex within Slot
        //             _sidx_page_addr,
        //             slot.value.idx
        //         );
                
        //         &slot.value
        //     }
        // })
        snapshot.slots.iter().map(move |slot | &slot.value)        
    }

    /// Hoist Snapshot the MPH index for hot-loop optimization (avoids repeated atomic loads).
    /// Returns a reference to the MPH index that remains valid for the guard's lifetime.
    pub fn snapshot<'g>(&'g self, guard: &'g epoch::Guard) -> &'g MPHIndex<K, StreamIndex<V>, I> {
        let mph_ptr = self.mph_index.load(std::sync::atomic::Ordering::Acquire, guard);
        unsafe { mph_ptr.as_ref().unwrap() }
    }

    /// Clear all data from the index (MPH, radix, and bloom filter).
    /// Resets the index to an empty state while preserving the indexer.
    pub fn clear(&self) {
        let guard = epoch::pin();
        
        // Clear radix index
        self.radix_index.clear_all(&guard);
        
        // Reset MPH to empty (preserve the indexer from the old MPH)
        let old_mph_ptr = self.mph_index.load(std::sync::atomic::Ordering::Acquire, &guard);
        if let Some(old_mph) = unsafe { old_mph_ptr.as_ref() } {
            let indexer = old_mph.indexer().clone();
            let empty_mph = MPHIndex::empty(indexer);
            let old = self.mph_index.swap(Owned::new(empty_mph), std::sync::atomic::Ordering::AcqRel, &guard);
            if !old.is_null() {
                unsafe { guard.defer_unchecked(move || drop(old.into_owned())); }
            }
        }
        
        // Reset bloom filter
        self.bloom.clear();
    }

    /// Consolidate the radix index.
    pub fn consolidate_radix_only(&self) {
        let guard = epoch::pin();
        self.radix_index.consolidate_buckets(&guard);
    }

    /// Consolidate the radix index.
    pub fn consolidate_radix_map_only(&self) {
        let guard = epoch::pin();
        self.radix_index.consolidate_snapshots_only(&guard);
    }    

    /// Get stats.
    pub fn stats(&self) -> OptimisedIndexStats {
        let guard = epoch::pin();
        let mph_ptr = self.mph_index.load(std::sync::atomic::Ordering::Acquire, &guard);
        let len_base = if let Some(mph) = unsafe { mph_ptr.as_ref() } {
            mph.len()
        } else {
            0
        };
        let len_delta = self.radix_index.collect_stats(&guard).total_keys;
        
        OptimisedIndexStats { len_base, len_delta }
    }

    /// Collect detailed RadixIndex statistics for diagnostics and performance analysis.
    pub fn radix_stats(&self, guard: &epoch::Guard) -> super::radix_stats::RadixIndexStats {
        self.radix_index.collect_stats(guard)
    }

    /// Get total number of entries (base + delta).
    pub fn len(&self) -> usize {
        let stats = self.stats();
        stats.len_base + stats.len_delta
    }

    /// Check if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get an owned copy of a value by key.
    pub fn get_owned(&self, key: &K) -> Option<V> {
        let guard = epoch::pin();
        self.get(key, &guard).map(|v| v.clone())
    }
}

// BBHashIndexer-specific convenience constructors (default indexer type).
impl<K, V> OptimisedIndexGen<K, V, BBHashIndexer<K>>
where
    K: Clone + Eq + std::hash::Hash + std::fmt::Debug + Default + Send + Sync + 'static,
    V: Clone + std::fmt::Debug + 'static,
{
    /// Create an empty index with default capacities and BBHashIndexer.
    /// Requires K: Default to create initial dummy indexer (rebuilt with real keys on first publish).
    pub fn new() -> Self {
        Self::new_with_capacity(4096, 8192)
    }

    /// Create an empty index with specific radix capacities and BBHashIndexer.
    /// Requires K: Default to create initial dummy indexer (rebuilt with real keys on first publish).
    pub fn new_with_capacity(
        radix_target_capacity: usize,
        radix_max_capacity: usize,
    ) -> Self {
        // Create a dummy BBHashIndexer with a single default key
        // This will be replaced with the real key set on first publish
        let dummy_key = K::default();
        let indexer = BBHashIndexer::build(&[dummy_key], Default::default());
        
        Self::new_with_indexer_and_capacity(indexer, radix_target_capacity, radix_max_capacity)
    }
}

impl<K: Clone, V, I: MphIndexer<K>> Drop for OptimisedIndexGen<K, V, I> {
    fn drop(&mut self) {
        debug_log!("OptimisedIndex DROP at {:p}", self as *const _);
    }
}
