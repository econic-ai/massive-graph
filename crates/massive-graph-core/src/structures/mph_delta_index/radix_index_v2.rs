use std::hash::Hash;
use std::sync::atomic::{Ordering, AtomicU64};
use crate::debug_log;
use super::util::{hash64, fpn_from_hash, tag8_from_hash_disjoint, preferred_slot_from_hash};
use super::radix_stats::{RadixIndexStats, BucketStats};

/// Target slots per bucket for initial sizing.
const TARGET_SLOTS_PER_BUCKET: usize = 8;

/// Maximum slots per bucket (limited by 64-bit mask).
const MAX_BUCKET_SLOTS: usize = 64;

/// Next-state radix delta index with fixed-capacity buckets and mask-driven visibility.
/// Uses fixed-capacity buckets with 64-bit active-slot masks and fp8 tags for lock-free operations.
/// Generic over V: Copy for inline cache-aligned storage.
pub struct RadixIndexV2<K, V> {
    buckets: Box<[Bucket<K, V>]>,      // Fixed array of buckets
    bucket_meta: Box<[BucketMeta]>,    // 1:1 metadata array (masks + tags)
    bucket_bits: u32,                  // Hash bits used to pick bucket
    bucket_slots: usize,               // Runtime slot count per bucket (≤ 64)
    slot_bits: u32,                     // Bits needed for slot index (cached)
    arena: super::arena::Arena,        // Arena-backed per-bucket storage
}

/// Per-bucket metadata: 64-bit mask and fp8 tags array.
struct BucketMeta {
    /// 1 bit per slot: 1 = live, 0 = empty/tombstone
    mask: AtomicU64,
    /// fp8/tag per slot, aligned with bucket slots
    tags: Box<[u8]>,
}

/// Fixed-capacity bucket with contiguous records.
struct Bucket<K, V> {
    /// Fixed-capacity slots, allocated from arena at construction
    /// Raw pointer - memory is managed by arena, not this struct
    recs: *mut Rec<K, V>,
}

#[repr(C)]
/// Record stored in a bucket slot.
struct Rec<K, V> {
    /// 0 = upsert, 1 = tombstone (for rebuilds); liveness comes from mask
    kind: u8,
    _pad: [u8; 7],
    key: K,
    value: V,
}


impl<K, V> RadixIndexV2<K, V>
where
    K: Eq + Hash + Clone + std::fmt::Debug,
    V: Copy + std::fmt::Debug + 'static,
{

    /// Calculate optimal bucket count based on target capacity.
    /// Returns (bucket_count, bucket_bits) using fixed TARGET_SLOTS_PER_BUCKET ratio.
    /// Minimum 2 buckets, no maximum limit.
    fn calculate_buckets(target_capacity: usize) -> (usize, u32) {
        let ideal = (target_capacity / TARGET_SLOTS_PER_BUCKET).max(1);
        let bucket_count = ideal.next_power_of_two().max(2);
        let bucket_bits = bucket_count.trailing_zeros();
        (bucket_count, bucket_bits)
    }

    /// Create a new empty radix index with default 256 buckets.
    // pub fn new() -> Self {
    //     Self::with_capacity(4096) // Default: optimize for ~4K entries
    // }

    /// Create a new empty radix index optimized for the given target capacity.
    /// Bucket count will be chosen to maintain ~8 entries per bucket (TARGET_SLOTS_PER_BUCKET).
    /// Bucket slots are fixed at MAX_BUCKET_SLOTS (64) for optimal performance.
    pub fn with_capacity(target_capacity: usize, _max_capacity: usize) -> Self {
        let (bucket_count, bucket_bits) = Self::calculate_buckets(target_capacity);
        
        // Use maximum bucket slots for optimal performance (64 slots per bucket)
        let bucket_slots = MAX_BUCKET_SLOTS;
        let slot_bits = bucket_slots.next_power_of_two().trailing_zeros() as u32;
        
        debug_log!("radix_index_v2 with_capacity bucket_count={} bucket_bits={} bucket_slots={} slot_bits={}", 
                   bucket_count, bucket_bits, bucket_slots, slot_bits);
        
        use std::mem::{size_of, align_of};
        let arena = super::arena::Arena::new(64 * 1024); // 64KB regions
        
        // Allocate buckets and metadata arrays
        let mut buckets_vec = Vec::with_capacity(bucket_count);
        let mut meta_vec = Vec::with_capacity(bucket_count);
        
        // Calculate size needed for one bucket's records
        let rec_size = size_of::<Rec<K, V>>();
        let bucket_recs_size = bucket_slots * rec_size;
        
        for _bidx in 0..bucket_count {
            // Allocate records array for this bucket from arena
            let recs_ptr = arena.alloc_bytes(bucket_recs_size, align_of::<Rec<K, V>>()) as *mut Rec<K, V>;
            
            // Initialize records array (all zeros = empty)
            unsafe {
                let recs_slice = std::slice::from_raw_parts_mut(recs_ptr, bucket_slots);
                for rec in recs_slice.iter_mut() {
                    core::ptr::write_bytes(rec as *mut Rec<K, V> as *mut u8, 0, rec_size);
                }
            }
            
            // Create bucket with raw pointer (arena manages memory, not Box)
            buckets_vec.push(Bucket {
                recs: recs_ptr,
            });
            
            // Create metadata: mask starts at 0, tags array initialized to 0
            let tags = vec![0u8; bucket_slots].into_boxed_slice();
            meta_vec.push(BucketMeta {
                mask: AtomicU64::new(0),
                tags,
            });
        }
        
        Self {
            buckets: buckets_vec.into_boxed_slice(),
            bucket_meta: meta_vec.into_boxed_slice(),
            bucket_bits,
            bucket_slots,
            slot_bits,
            arena,
        }
    }


    /// Get the bucket index for a hash using the configured bucket_bits.
    #[inline]
    fn bucket_index(&self, h: u64) -> usize {
        fpn_from_hash(h, self.bucket_bits)
    }

    /// Get the 8-bit tag for a hash using bits disjoint from bucket_bits.
    #[inline]
    fn tag8(&self, h: u64) -> u8 {
        tag8_from_hash_disjoint(h, self.bucket_bits)
    }

    /// Get the preferred slot index for a hash using bits disjoint from bucket_bits and tag bits.
    #[inline]
    fn preferred_slot(&self, h: u64) -> usize {
        // bucket_slots is always MAX_BUCKET_SLOTS (64), which is a power of 2,
        // so preferred_slot_from_hash already returns a value in [0, bucket_slots)
        preferred_slot_from_hash(h, self.bucket_bits, self.slot_bits)
    }

    /// Find an empty slot in a bucket starting from preferred slot, wrapping around.
    /// Returns None if bucket is full (all slots have mask bit set).
    #[inline]
    fn find_empty_slot(&self, mask: u64, start: usize) -> Option<usize> {
        // Check if bucket is full
        if mask.count_ones() as usize >= self.bucket_slots {
            return None;
        }
        
        // Walk forward from preferred slot, wrapping around
        let mut slot = start;
        for _ in 0..self.bucket_slots {
            if (mask >> slot) & 1 == 0 {
                return Some(slot);
            }
            slot = (slot + 1) % self.bucket_slots;
        }
        None
    }

    /// Find slot containing a key by walking from preferred slot.
    /// Returns None if key not found.
    #[inline]
    fn find_slot_by_key(&self, bidx: usize, mask: u64, start: usize, tag: u8, key: &K) -> Option<usize> {
        let mut slot = start;
        for _ in 0..self.bucket_slots {
            // Check if slot is live
            if (mask >> slot) & 1 == 1 {
                // Early tag comparison (cache-friendly)
                if self.bucket_meta[bidx].tags[slot] == tag {
                    // Full key comparison (only if tag matches)
                    unsafe {
                        let rec = &*self.buckets[bidx].recs.add(slot);
                        if &rec.key == key {
                            return Some(slot);
                        }
                    }
                }
            }
            slot = (slot + 1) % self.bucket_slots;
        }
        None
    }

    /// Clear all buckets by resetting masks and tags.
    pub fn clear_all(&self) {
        debug_log!("radix_index_v2 clear_all");
        for meta in self.bucket_meta.iter() {
            meta.mask.store(0, Ordering::Release);
            // Tags array already zeroed, no need to clear
        }
    }

    /// Get a reference to the value for a key (returns reference, no copy).
    /// Returns Some(&V) for upsert, None for tombstone or absent key.
    pub fn get(&self, key: &K) -> Option<&V> {
        let h = hash64(key);
        self.get_with_hash(key, h)
    }

    /// Get a reference to the value for a key with pre-computed hash (avoids redundant hashing).
    /// Returns Some(&V) for upsert, None for tombstone or absent key.
    /// Hot path: single atomic load + bit-scan walk + tag comparison.
    pub fn get_with_hash(&self, key: &K, hash: u64) -> Option<&V> {
        
        // Get the bucket index
        let bidx = self.bucket_index(hash);
        
        // SINGLE HOT-PATH ATOMIC LOAD
        let mask = self.bucket_meta[bidx].mask.load(Ordering::Acquire);
        
        // Early return if bucket is empty
        if mask == 0 {
            return None;
        }
        
        let preferred = self.preferred_slot(hash);
        let tag = self.tag8(hash);
                
        // Bit-scan walk from preferred slot
        if let Some(slot) = self.find_slot_by_key(bidx, mask, preferred, tag, key) {
            unsafe {
                let rec = &*self.buckets[bidx].recs.add(slot);
                return match rec.kind {
                    0 => Some(&rec.value), // Upsert
                    1 => None, // Tombstone
                    _ => None,
                };
            }
        }
        
        None
    }

    /// Get a copy of the value for a key (explicit copy operation).
    /// Returns Some(V) for upsert, None for tombstone or absent key.
    /// Use this when you need an owned value; prefer `get()` for references.
    pub fn get_copy(&self, key: &K) -> Option<V> {
        self.get(key).copied()
    }

    /// Streaming iterator over upsert records (returns &V directly).
    /// Zero allocations, optimal cache locality.
    /// Uses mask hoisting: loads all masks upfront, then iterates over local copy.
    pub fn iter(&self) -> impl Iterator<Item = &V> {
        // HOIST ALL MASKS AT START (single pass, cache-friendly)
        let masks: Vec<u64> = (0..self.buckets.len())
            .map(|i| self.bucket_meta[i].mask.load(Ordering::Acquire))
            .collect();
        
        // Iterate over local copy - ZERO ATOMICS DURING YIELD
        self.buckets.iter().enumerate().flat_map(move |(bidx, bucket)| {
            let mask = masks[bidx];
            if mask == 0 {
                return None; // Early skip empty buckets
            }
            
            // Bit-scan walk over mask
            Some((0..self.bucket_slots)
                .filter(move |slot| (mask >> slot) & 1 == 1)
                .filter_map(move |slot| {
                    unsafe {
                        let rec = &*bucket.recs.add(slot);
                        if rec.kind == 0 { // Skip tombstones
                            Some(&rec.value)
                        } else {
                            None
                        }
                    }
                }))
        }).flatten()
    }

    /// Streaming iterator over (&K, &V) upserts (excludes tombstones); zero allocations.
    /// Uses mask hoisting for optimal performance.
    pub fn iter_with_keys(&self) -> impl Iterator<Item = (&K, &V)> {
        // HOIST ALL MASKS AT START
        let masks: Vec<u64> = (0..self.buckets.len())
            .map(|i| self.bucket_meta[i].mask.load(Ordering::Acquire))
            .collect();
        
        // Iterate over local copy
        self.buckets.iter().enumerate().flat_map(move |(bidx, bucket)| {
            let mask = masks[bidx];
            if mask == 0 {
                return None;
            }
            
            Some((0..self.bucket_slots)
                .filter(move |slot| (mask >> slot) & 1 == 1)
                .filter_map(move |slot| {
                    unsafe {
                        let rec = &*bucket.recs.add(slot);
                        if rec.kind == 0 {
                            Some((&rec.key, &rec.value))
                        } else {
                            None
                        }
                    }
                }))
        }).flatten()
    }

    /// Convenience adaptor returning owned key plus optional value.
    // iter_items removed.

    /// Insert or update a key-value pair (lock-free).
    /// Fast path for new keys: bit-scan + write + single atomic fetch_or.
    /// Update path for existing keys: locate old + find new + CAS swap.
    pub fn upsert(&self, key: &K, value: &V) {
        debug_log!("radix_index_v2 upsert key={:?}", key);
        let h = hash64(key);
        let bidx = self.bucket_index(h);
        let preferred = self.preferred_slot(h);
        let tag = self.tag8(h);
        
        // Load mask to check for existing key
        let mask = self.bucket_meta[bidx].mask.load(Ordering::Acquire);
        
        // Check if key already exists
        if let Some(old_slot) = self.find_slot_by_key(bidx, mask, preferred, tag, key) {
            // UPDATE PATH: Key exists, need to update
            // Find a new slot for the new value
            let new_slot = match self.find_empty_slot(mask, preferred) {
                Some(slot) => slot,
                None => {
                    // Bucket is full - panic for now (future: rebuild path)
                    panic!("radix_index_v2: bucket {} is full ({} slots)", bidx, self.bucket_slots);
                }
            };
            
            // Write new record to new slot
            unsafe {
                let bucket_ptr = self.buckets.as_ptr().add(bidx) as *mut Bucket<K, V>;
                let rec_ptr = (*bucket_ptr).recs.add(new_slot);
                (*rec_ptr) = Rec {
                    kind: 0,
                    _pad: [0; 7],
                    key: key.clone(),
                    value: *value,
                };
            }
            
            // Write tag to new slot
            unsafe {
                let meta_ptr = self.bucket_meta.as_ptr().add(bidx) as *mut BucketMeta;
                (*meta_ptr).tags[new_slot] = tag;
            }
            
            // CAS mask: atomically clear old slot and set new slot
            loop {
                let old_mask = self.bucket_meta[bidx].mask.load(Ordering::Acquire);
                let new_mask = (old_mask | (1u64 << new_slot)) & !(1u64 << old_slot);
                
                match self.bucket_meta[bidx].mask.compare_exchange(
                    old_mask,
                    new_mask,
                    Ordering::AcqRel,
                    Ordering::Acquire
                ) {
                    Ok(_) => break, // Success
                    Err(_) => continue, // Retry (bounded by bucket size)
                }
            }
        } else {
            // INSERT PATH: New key
            // Find empty slot
            let slot = match self.find_empty_slot(mask, preferred) {
                Some(slot) => slot,
                None => {
                    // Bucket is full - panic for now (future: rebuild path)
                    panic!("radix_index_v2: bucket {} is full ({} slots)", bidx, self.bucket_slots);
                }
            };
            
            // Write key/value/tag BEFORE publishing (store ordering)
            unsafe {
                let bucket_ptr = self.buckets.as_ptr().add(bidx) as *mut Bucket<K, V>;
                let rec_ptr = (*bucket_ptr).recs.add(slot);
                (*rec_ptr) = Rec {
                    kind: 0,
                    _pad: [0; 7],
                    key: key.clone(),
                    value: *value,
                };
            }
            unsafe {
                let meta_ptr = self.bucket_meta.as_ptr().add(bidx) as *mut BucketMeta;
                (*meta_ptr).tags[slot] = tag;
            }
            
            // SINGLE HOT-PATH ATOMIC PUBLICATION
            self.bucket_meta[bidx].mask.fetch_or(1u64 << slot, Ordering::Release);
        }
    }

    /// Delete a key by clearing its mask bit.
    /// Optionally marks tombstone in record for debugging/rebuilds.
    pub fn delete(&self, key: &K)
    where
        V: Default,
    {
        let h = hash64(key);
        let bidx = self.bucket_index(h);
        let preferred = self.preferred_slot(h);
        let tag = self.tag8(h);
        
        // Locate slot (same as read path)
        let mask = self.bucket_meta[bidx].mask.load(Ordering::Acquire);
        let slot = match self.find_slot_by_key(bidx, mask, preferred, tag, key) {
            Some(slot) => slot,
            None => return, // Key not found
        };
        
        // Optionally mark tombstone
        unsafe {
            let bucket_ptr = self.buckets.as_ptr().add(bidx) as *mut Bucket<K, V>;
            let rec_ptr = (*bucket_ptr).recs.add(slot);
            (*rec_ptr).kind = 1;
        }
        
        // SINGLE HOT-PATH ATOMIC CLEAR
        self.bucket_meta[bidx].mask.fetch_and(!(1u64 << slot), Ordering::Release);
    }

    /// Collect comprehensive statistics for diagnostic analysis.
    /// Simplified version for V2 - counts live entries from masks.
    pub fn collect_stats(&self) -> RadixIndexStats {
        let bucket_count = self.buckets.len();
        let mut bucket_details = Vec::new();
        
        // Collect per-bucket statistics
        for bidx in 0..bucket_count {
            let mask = self.bucket_meta[bidx].mask.load(Ordering::Relaxed);
            let key_count = mask.count_ones() as usize;
            
            // Count actual live records (exclude tombstones)
            let mut live_count = 0;
            for slot in 0..self.bucket_slots {
                if (mask >> slot) & 1 == 1 {
                    unsafe {
                        let rec = &*self.buckets[bidx].recs.add(slot);
                        if rec.kind == 0 {
                            live_count += 1;
                        }
                    }
                }
            }
            
            bucket_details.push(BucketStats {
                bucket_idx: bidx,
                key_count: live_count,
                total_records: key_count,
                buffer_capacity: self.bucket_slots,
                tinymap_size: 0, // No TinyMap in V2
                growth_count: 0,
                tag8_collisions: 0, // TODO: calculate if needed
                unique_tags: 0, // TODO: calculate if needed
            });
        }
        
        let arena_stats = self.arena.stats();
        let mut stats = RadixIndexStats {
            total_buckets: bucket_count,
            bucket_bits: self.bucket_bits,
            active_buckets: bucket_count, // All buckets are always "active" in V2
            total_keys: bucket_details.iter().map(|b| b.key_count).sum(),
            bucket_utilization: 0.0,
            avg_bucket_depth: 0.0,
            max_bucket_depth: 0,
            min_bucket_depth: 0,
            bucket_depth_stddev: 0.0,
            bucket_distribution_entropy: 0.0,
            avg_tag16_uniqueness: 0.0,
            hotpath_arena: arena_stats.clone(),
            buffer_arena: super::arena::ArenaStats::empty(),
            record_arena: super::arena::ArenaStats::empty(),
            bucket_details,
        };
        
        // Calculate derived statistics
        stats.calculate_derived_stats();
        
        stats
    }

}

impl<K, V> Drop for RadixIndexV2<K, V> {
    fn drop(&mut self) {
        // Arena will clean up allocations automatically
        debug_log!("radix_index_v2 drop");
    }
}

// Unit tests moved to /tests/radix_index_tests.rs

