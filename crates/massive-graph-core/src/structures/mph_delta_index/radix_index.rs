use std::hash::Hash;
use std::sync::Mutex;
use std::sync::atomic::{Ordering, AtomicPtr};
use std::ptr;
use std::collections::HashMap;
use crate::debug_log;
use super::util::{hash64, fpn_from_hash, tag8_from_hash_disjoint};
use super::radix_stats::{RadixIndexStats, BucketStats};
use crossbeam_epoch as epoch;
use crossbeam_epoch::{Owned, Shared};
use super::tiny_map::ReadTinyMap;
// DIAGNOSTIC: Epoch tracker disabled for benchmarking
// use super::epoch_tracker::{EPOCH_STATS, track_defer};

// TARGET_ENTRIES_PER_BUCKET is now adaptive - see calculate_buckets()

// MIN_BUFFER_CAPACITY and MAX_BUFFER_CAPACITY are now calculated on-demand
// based on the adaptive bucket sizing strategy. See initial_buffer_capacity()
// and max_buffer_capacity() methods.

/// Target slots per bucket for initial sizing.
const TARGET_SLOTS_PER_BUCKET: usize = 8;

/// New-keys radix delta index with configurable bucket count and RCU buffers.
/// Uses arena allocator for colocated [Buffer | Recs | TinyMap] allocations.
/// Generic over V: Copy for inline cache-aligned storage.
pub struct RadixIndex<K, V> {
    buckets: Box<[Bucket<K, V>]>,     // Owned bucket array
    bucket_bits: u32,                  // Number of bits for bucket indexing
    active: epoch::Atomic<Vec<u16>>,   // RCU-managed sorted active bucket indices (for cache-friendly iteration)
    arena: super::arena::Arena,        // Arena for colocated allocations
    initial_buffer_capacity: usize,    // Initial buffer size per bucket (based on max_capacity)
    theoretical_max_per_bucket: usize, // Warning threshold for bucket growth
}

/// Per-bucket state: RCU-managed buffer, grow lock, writer-side dedup index and registration flag.
struct Bucket<K, V> {
    head: AtomicPtr<Buffer<K, V>>,        // RCU pointer to buffer
    grow_mx: Mutex<()>,          // serialize rare grow
    write_mx: Mutex<()>,          // serialize snapshot publishes and appends
    registered: core::sync::atomic::AtomicBool, // set once per bucket lifetime
    snapshot: epoch::Atomic<ReadTinyMap>, // RCU reader snapshot of latest entries
    snapshot_tail: core::sync::atomic::AtomicUsize, // records covered by snapshot [0..tail), 0=no snapshot
}

/// Buffer of records with append-only tail and fixed capacity.
/// Part of colocated [Buffer | Recs | TinyMap] arena allocation.
/// recs points to memory immediately following this Buffer header.
struct Buffer<K, V> {
    tail: core::sync::atomic::AtomicUsize, // committed length
    cap:  usize,
    recs: *mut Rec<K, V>,  // Points to recs array immediately after Buffer header
    tinymap_ptr: *const ReadTinyMap, // Points to TinyMap at end of allocation
}

#[repr(C)]
/// Record stored in a bucket buffer.
struct Rec<K, V> {
    kind:  u8,           // 0=Upsert, 1=Tombstone
    _pad:  [u8;7],       // keep alignment predictable
    key:   K,            // full key (sizeof(K))
    value: V,            // value (V: Copy for inline storage)
}


impl<K, V> RadixIndex<K, V>
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
    /// Initial buffer capacity per bucket is based on max_capacity / bucket_count.
    /// No upper limit on bucket count - scales to any capacity.
    pub fn with_capacity(target_capacity: usize, max_capacity: usize) -> Self {
        let (bucket_count, bucket_bits) = Self::calculate_buckets(target_capacity);
        
        // Calculate initial buffer capacity based on max_capacity distribution
        let initial_buffer_capacity = (max_capacity / bucket_count).max(TARGET_SLOTS_PER_BUCKET);
        
        // Theoretical max is the same as initial capacity (we expect even distribution)
        let theoretical_max_per_bucket = initial_buffer_capacity;
        
        debug_log!("radix_index with_capacity bucket_count={} bucket_bits={} initial_buffer_capacity={} theoretical_max_per_bucket={}", 
                   bucket_count, bucket_bits, initial_buffer_capacity, theoretical_max_per_bucket);
        
        // Create owned bucket array - lazy initialization via ensure_activated
        let buckets: Vec<Bucket<K, V>> = (0..bucket_count).map(|_| Bucket {
            head: AtomicPtr::new(core::ptr::null_mut()),
            grow_mx: Mutex::new(()),
            write_mx: Mutex::new(()),
            registered: core::sync::atomic::AtomicBool::new(false),
            snapshot: epoch::Atomic::null(),
            snapshot_tail: core::sync::atomic::AtomicUsize::new(0), // 0 = no snapshot
        }).collect();
        
        let active = epoch::Atomic::new(Vec::new());
        let arena = super::arena::Arena::new(64 * 1024); // 64KB regions
        
        // Track creation
        // EPOCH_STATS.register_radix_create();
        
        Self { 
            buckets: buckets.into_boxed_slice(),
            bucket_bits,
            active,
            arena,
            initial_buffer_capacity,
            theoretical_max_per_bucket,
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

    /// Get initial buffer capacity for new buckets.
    /// Calculated as max_capacity / bucket_count (minimum TARGET_SLOTS_PER_BUCKET).
    #[inline]
    fn initial_buffer_capacity(&self) -> usize {
        self.initial_buffer_capacity
    }

    /// Clear all buckets and active map; retire buffers via RCU.
    pub fn clear_all(&self, guard: &epoch::Guard) {
        debug_log!("radix_index clear_all");
        for (_i, b) in self.buckets.iter().enumerate() {
            debug_log!("radix_index clear_all bucket: {:?}", &b as *const _);
            let cur = b.head.swap(core::ptr::null_mut(), Ordering::AcqRel);
            if !cur.is_null() {
                // Arena-allocated buffer will be freed when arena drops (no defer needed)
                debug_log!("radix_index clear_all buffer: {:?}", cur);
            }
            // Clear snapshot and retire old ReadTinyMap
            let prev_snap = b.snapshot.swap(Shared::null(), Ordering::AcqRel, guard);
            if !prev_snap.is_null() {
                debug_log!("radix_index clear_all prev_snap: {:?}",  prev_snap );
                // let old_ptr = prev_snap.as_raw() as *mut ReadTinyMap;
                // track_defer();
                unsafe { guard.defer_destroy(prev_snap); }
                // unsafe { guard.defer_unchecked(move || drop(Box::from_raw(old_ptr))); }
            }
            b.registered.store(false, Ordering::Relaxed);
            b.snapshot_tail.store(0, Ordering::Relaxed); // Reset to no snapshot
        }
        // Reset active list with epoch RCU
        let old = self.active.swap(Owned::new(Vec::<u16>::new()), Ordering::AcqRel, guard);
        if !old.is_null() {
            unsafe { guard.defer_unchecked(move || drop(old.into_owned())); }
        }
    }

    /// Get a reference to the value for a key (returns reference, no copy).
    /// Returns Some(&V) for upsert, None for tombstone or absent key.
    /// When snapshot is dirty, scans only new records since last snapshot, then falls back to TinyMap.
    pub fn get<'g>(&'g self, key: &K, guard: &'g epoch::Guard) -> Option<&'g V> {
        let h = hash64(key);
        self.get_with_hash(key, h, guard)
    }

    /// Get a reference to the value for a key with pre-computed hash (avoids redundant hashing).
    /// Returns Some(&V) for upsert, None for tombstone or absent key.
    pub fn get_with_hash<'g>(&'g self, key: &K, hash: u64, guard: &'g epoch::Guard) -> Option<&'g V> {
        let b = &self.buckets[self.bucket_index(hash)];
        let tag = self.tag8(hash);
        
        let cur = b.head.load(Ordering::Acquire);
        if cur.is_null() { return None; }
        let buf = unsafe { &*cur };
        
        // Check for records added since snapshot (hot path optimization)
        let snapshot_tail = b.snapshot_tail.load(Ordering::Acquire);
        let current_tail = buf.tail.load(Ordering::Acquire);
        
        if snapshot_tail > 0 && snapshot_tail < current_tail {
            // Scan only NEW records (from snapshot_tail to current tail) in reverse
            for i in (snapshot_tail..current_tail).rev() {
                let rec = unsafe { &*buf.recs.add(i) };
                if &rec.key == key {
                    return match rec.kind {
                        0 => Some(&rec.value), // Upsert - return reference to value
                        1 => None, // Tombstone
                        _ => None,
                    };
                }
            }
            // Not found in new records - fall through to check TinyMap below
        }
        
        // Check TinyMap snapshot
        let snap = b.snapshot.load(Ordering::Acquire, guard);
        if snap.is_null() { return None; }
        let map = unsafe { snap.deref() };
        
        // Binary search on sorted tags
        for slot_idx in map.iter_tags_linear(tag) {
            let rec = unsafe { &*buf.recs.add(slot_idx as usize) };
            if &rec.key == key {
                return Some(&rec.value);  // Return reference to value
            }
        }
        None
    }

    /// Get a copy of the value for a key (explicit copy operation).
    /// Returns Some(V) for upsert, None for tombstone or absent key.
    /// Use this when you need an owned value; prefer `get()` for references.
    pub fn get_copy(&self, key: &K, guard: &epoch::Guard) -> Option<V> {
        self.get(key, guard).copied()
    }

    /// Streaming iterator over upsert records (returns &V directly).
    /// Zero allocations, optimal cache locality.
    pub fn iter<'g>(&'g self, guard: &'g epoch::Guard) -> impl Iterator<Item = &'g V> + 'g {
        struct State<'a, K, V> {
            idx: &'a RadixIndex<K, V>,
            guard: &'a epoch::Guard,
            active: &'a Vec<u16>,  // Reference to active list (kept alive by epoch guard)
            bucket_pos: usize,
            cur_buf: *const Buffer<K, V>,
            slot_iter: Option<std::iter::Copied<std::slice::Iter<'a, u16>>>,
        }
        
        // Load active list with epoch guard (sorted bucket indices for cache-friendly iteration)
        let active_ptr = self.active.load(Ordering::Acquire, guard);
        let active_vec = unsafe { active_ptr.as_ref().unwrap() };
        
        let mut st = State {
            idx: self,
            guard,
            active: active_vec,
            bucket_pos: 0,
            cur_buf: core::ptr::null(),
            slot_iter: None,
        };
        
        std::iter::from_fn(move || {
            loop {
                // Try to get next slot from current bucket
                if let Some(ref mut iter) = st.slot_iter {
                    if let Some(slot_idx) = iter.next() {
                        if !st.cur_buf.is_null() {
                            let buf = unsafe { &*st.cur_buf };
                            let rec = unsafe { &*buf.recs.add(slot_idx as usize) };
                            return Some(&rec.value);
                        }
                    } else {
                        // Exhausted current bucket
                        st.slot_iter = None;
                    }
                }
                
                // Advance to next bucket
                if st.bucket_pos >= st.active.len() { return None; }
                let bidx = st.active[st.bucket_pos] as usize;
                st.bucket_pos += 1;
                
                let b = &st.idx.buckets[bidx];
                let map_ptr = b.snapshot.load(Ordering::Acquire, st.guard);
                if map_ptr.is_null() { continue; }
                let map = unsafe { map_ptr.deref() };
                
                let cur = b.head.load(Ordering::Acquire);
                if cur.is_null() { continue; }
                
                st.cur_buf = cur;
                st.slot_iter = Some(map.slots().iter().copied());
            }
        })
    }

    /// Streaming iterator over (&K, &V) upserts (excludes tombstones); zero allocations.
    /// Manual state machine for optimal performance - no iterator adapter overhead.
    pub fn iter_with_keys<'g>(&'g self, guard: &'g epoch::Guard) -> impl Iterator<Item = (&'g K, &'g V)> + 'g {
        struct State<'a, K, V> {
            idx: &'a RadixIndex<K, V>,
            guard: &'a epoch::Guard,
            active: &'a Vec<u16>,  // Reference to active list (kept alive by epoch guard)
            bucket_pos: usize,
            cur_buf: *const Buffer<K, V>,
            slot_iter: Option<std::iter::Copied<std::slice::Iter<'a, u16>>>,
        }
        
        // Load active list with epoch guard (sorted bucket indices for cache-friendly iteration)
        let active_ptr = self.active.load(Ordering::Acquire, guard);
        let active_vec = unsafe { active_ptr.as_ref().unwrap() };
        
        let mut st = State {
            idx: self,
            guard,
            active: active_vec,
            bucket_pos: 0,
            cur_buf: core::ptr::null(),
            slot_iter: None,
        };
        
        std::iter::from_fn(move || {
            loop {
                // Try to get next slot from current bucket
                if let Some(ref mut iter) = st.slot_iter {
                    if let Some(slot_idx) = iter.next() {
                        if !st.cur_buf.is_null() {
                            let buf = unsafe { &*st.cur_buf };
                            let rec = unsafe { &*buf.recs.add(slot_idx as usize) };
                            return Some((&rec.key, &rec.value));  // Return references to both
                        }
                    } else {
                        // Exhausted current bucket
                        st.slot_iter = None;
                    }
                }
                
                // Advance to next bucket
                if st.bucket_pos >= st.active.len() { return None; }
                let bidx = st.active[st.bucket_pos] as usize;
                st.bucket_pos += 1;
                
                let b = &st.idx.buckets[bidx];
                let map_ptr = b.snapshot.load(Ordering::Acquire, st.guard);
                if map_ptr.is_null() { continue; }
                let map = unsafe { map_ptr.deref() };
                
                let cur = b.head.load(Ordering::Acquire);
                if cur.is_null() { continue; }
                
                st.cur_buf = cur;
                st.slot_iter = Some(map.slots().iter().copied());
            }
        })
    }

    /// Convenience adaptor returning owned key plus optional value.
    // iter_items removed.

    /// Insert or update a key for new-keys delta.
    /// Fast O(1) append - defers TinyMap rebuild until grow/consolidation.
    pub fn upsert(&self, key: &K, value: &V, _guard: &epoch::Guard) {
        let h = hash64(key);
        let bidx = self.bucket_index(h);
        let b = &self.buckets[bidx];
        // debug_log!("radix_index upsert key({:?}) bucket_index({})", key, bidx);

        self.ensure_activated(b, bidx as u16);

        // let _wl = b.write_mx.lock().unwrap();
        
        // Just append to buffer - O(1) operation
        let (buf, slot_idx) = self.reserve_slot(b);
        unsafe {
            let buf_ref = &mut *buf;
            let rec_ptr = buf_ref.recs.add(slot_idx);
            (*rec_ptr).kind = 0; // Upsert
            (*rec_ptr).key = key.clone();
            (*rec_ptr).value = *value;  // Copy V from reference (V is Copy)
        }
        
        // snapshot_tail stays unchanged - it marks where the TinyMap ends, new records go beyond it
    }

    /// Tombstone a key in new-keys delta.
    /// Appends tombstone record and rebuilds snapshot.
    pub fn delete(&self, key: &K, _guard: &epoch::Guard) 
    where
        V: Default,
    {
        let h = hash64(key);
        let bidx = self.bucket_index(h);
        // debug_log!("radix_index delete key({:?}) bucket_index({})", key, bidx);
        let b = &self.buckets[bidx];
        self.ensure_activated(b, bidx as u16);

        let _wl = b.write_mx.lock().unwrap();
        
        // Append tombstone record - O(1) operation
        let (buf, i) = self.reserve_slot(b);
        unsafe {
            let buf_ref = &mut *buf;
            let rec_ptr = buf_ref.recs.add(i);
            (*rec_ptr).kind = 1; // Tombstone
            (*rec_ptr).key = key.clone();
            (*rec_ptr).value = V::default();  // Placeholder value for tombstone
        }
        
        // snapshot_tail stays unchanged - tombstone goes beyond it
    }

    /// Rebuild TinyMap snapshot from current buffer state.
    /// Used by consolidate_snapshots_only to rebuild stale snapshots without reallocation.
    fn rebuild_snapshot(&self, b: &Bucket<K, V>, guard: &epoch::Guard) {
        debug_log!("radix_index rebuild_snapshot bucket");
        let cur = b.head.load(Ordering::Acquire);
        if cur.is_null() { return; }
        
        let buf = unsafe { &*cur };
        let tail = buf.tail.load(Ordering::Acquire);
        
        // Collect entries from buffer: (tag8, hash64, slot_idx, kind)
        let mut entries: Vec<(u8, u64, usize, u8)> = Vec::with_capacity(tail);
        for i in 0..tail {
            let rec = unsafe { &*buf.recs.add(i) };
            let h = hash64(&rec.key);
            let tag = self.tag8(h);
            entries.push((tag, h, i, rec.kind));
        }
        
        // Calculate TinyMap size and allocate from arena (not colocated with buffer)
        let filtered_count = entries.iter().filter(|(_, _, _, k)| *k == 0).count();
        let tinymap_size = ReadTinyMap::calculate_size(filtered_count);
        
        if tinymap_size == 0 {
            // Empty TinyMap - just clear snapshot
            let prev = b.snapshot.swap(Shared::null(), Ordering::AcqRel, guard);
            if !prev.is_null() {
                // track_defer();
                unsafe { guard.defer_unchecked(move || drop(Box::from_raw(prev.as_raw() as *mut ReadTinyMap))); }
            }
            b.snapshot_tail.store(0, Ordering::Release); // No snapshot
            return;
        }
        
        // Allocate TinyMap data from arena
        let tinymap_data_ptr = self.arena.alloc_bytes(tinymap_size, core::mem::align_of::<u16>());
        let new_snap = ReadTinyMap::new_in_place(tinymap_data_ptr, entries);
        let new_snap_ptr = Box::into_raw(Box::new(new_snap));
        
        // Swap and retire old
        let prev = b.snapshot.swap(
            unsafe { Owned::from_raw(new_snap_ptr) },
            Ordering::AcqRel,
            guard
        );
        if !prev.is_null() {
            let old_ptr = prev.as_raw() as *mut ReadTinyMap;
            // track_defer();
            unsafe { guard.defer_unchecked(move || drop(Box::from_raw(old_ptr))); }
        }
        
        // Update snapshot_tail to mark all records as covered
        b.snapshot_tail.store(tail, Ordering::Release);
    }

    /// Consolidate a bucket by deduplicating records (keeping only latest version of each key).
    /// **Preserves tombstones** - use when RadixIndex is a delta over MPH base.
    /// Allocates new buffer with deduplicated records, rebuilds snapshot, and retires old buffer.
    /// Smart capacity: doubles if >50% full after dedup, otherwise keeps same size.
    fn consolidate_bucket(&self, b: &Bucket<K, V>, guard: &epoch::Guard) {
        // debug_log!("radix_index consolidate_bucket");
        let _l = b.grow_mx.lock(); // Serialize with growth
        let cur = b.head.load(Ordering::Acquire);
        if cur.is_null() { return; }
        
        let old_buf = unsafe { &*cur };
        let old_tail = old_buf.tail.load(Ordering::Acquire);
        let old_cap = old_buf.cap;
        
        // DEDUPLICATION: Track slot indices only (not full records for performance)
        let mut seen_keys = std::collections::HashSet::new();
        let mut keep_slots = Vec::new();  // Just usize indices (cheap!)
        
        // Scan buffer in reverse (most recent first) to find latest version of each key
        for i in (0..old_tail).rev() {
            let rec = unsafe { &*old_buf.recs.add(i) };
            
            // Keep only first (most recent) occurrence of each key
            if seen_keys.insert(rec.key.clone()) {
                // Keep both upserts (kind=0) and tombstones (kind=1)
                keep_slots.push(i);  // Just store the slot index
            }
        }
        
        // Reverse to restore chronological order (oldest first)
        keep_slots.reverse();
        
        let new_count = keep_slots.len();
        if new_count == 0 {
            // All records were duplicates - just clear
            // Arena-allocated buffer will be freed when arena drops (no defer needed)
            let _prev = b.head.swap(core::ptr::null_mut(), Ordering::Release);
            let prev_snap = b.snapshot.swap(Shared::null(), Ordering::AcqRel, guard);
            if !prev_snap.is_null() {
                // track_defer();
                unsafe { guard.defer_unchecked(move || drop(Box::from_raw(prev_snap.as_raw() as *mut ReadTinyMap))); }
            }
            b.snapshot_tail.store(0, Ordering::Release); // No snapshot
            return;
        }
        
        // SMART CAPACITY: Double if >50% full after dedup, else keep same size
        let new_cap = if new_count > (old_cap / 2) {
            // More than 50% full after dedup → double
            old_cap.saturating_mul(2)
        } else {
            // Less than 50% full → keep same size
            old_cap
        }.max(self.initial_buffer_capacity());
        
        // Diagnostic warning if exceeding theoretical maximum
        if new_cap > self.theoretical_max_per_bucket {
            let _stats = self.collect_stats(&epoch::pin());
            debug_log!(
                "RadixIndex::consolidate_bucket CAPACITY WARNING!\n\
                 Bucket ptr: {:p}\n\
                 Deduplicated count: {}\n\
                 Old capacity: {}\n\
                 New capacity: {}\n\
                 Theoretical max: {}\n\
                 Bucket bits: {}\n\
                 \n\
                 Statistics:\n\
                 {}\n\
                 \n\
                 Consider increasing bucket_bits or investigating key distribution.",
                b as *const _,
                new_count,
                old_cap,
                new_cap,
                self.theoretical_max_per_bucket,
                self.bucket_bits,
                _stats
            );
        }
        
        // Build TinyMap entries from deduplicated slots (includes tombstones)
        let mut entries = Vec::with_capacity(new_count);
        for (new_slot_idx, &old_slot_idx) in keep_slots.iter().enumerate() {
            let rec = unsafe { &*old_buf.recs.add(old_slot_idx) };
            let h = hash64(&rec.key);
            let tag = self.tag8(h);
            entries.push((tag, h, new_slot_idx, rec.kind));
        }
        
        // Allocate colocated [Buffer | Recs | TinyMap] with calculated capacity
        let new_buf = self.alloc_colocated_buffer(new_cap, entries);
        
        // DIRECT COPY: From old buffer to new buffer (no intermediate Vec)
        for (new_slot_idx, &old_slot_idx) in keep_slots.iter().enumerate() {
            unsafe {
                let new_buf_ref = &mut *new_buf;
                let dst = new_buf_ref.recs.add(new_slot_idx);
                let src = old_buf.recs.add(old_slot_idx) as *const Rec<K, V>;
                core::ptr::copy_nonoverlapping(src, dst, 1);  // Direct memcpy
            }
        }
        
        unsafe { (&mut *new_buf).tail.store(new_count, Ordering::Relaxed); }
        
        // Update bucket snapshot from colocated TinyMap
        let tinymap_ptr = unsafe { (*new_buf).tinymap_ptr };
        if !tinymap_ptr.is_null() {
            let prev_snap = b.snapshot.swap(
                unsafe { Owned::from_raw(tinymap_ptr as *mut ReadTinyMap) },
                Ordering::AcqRel,
                guard
            );
            if !prev_snap.is_null() {
                // track_defer();
                unsafe { guard.defer_unchecked(move || drop(Box::from_raw(prev_snap.as_raw() as *mut ReadTinyMap))); }
            }
            b.snapshot_tail.store(new_count, Ordering::Release); // Snapshot covers all records [0..new_count)
        }
        
        // Swap buffers - old buffer is arena-allocated and will be freed when arena drops
        let _prev = b.head.swap(new_buf, Ordering::Release);
        
        debug_log!("radix_index consolidate_bucket: {} records -> {} deduplicated (cap: {} -> {})", 
                   old_tail, new_count, old_cap, new_cap);
    }

    /// Compact a bucket by deduplicating records and **removing tombstones**.
    /// Use when RadixIndex is standalone or when tombstones are no longer needed.
    /// Allocates new buffer with only live records, rebuilds snapshot, and retires old buffer.
    fn compact_bucket(&self, b: &Bucket<K, V>, guard: &epoch::Guard) {
        debug_log!("radix_index compact_bucket");
        let _l = b.grow_mx.lock(); // Serialize with growth
        let cur = b.head.load(Ordering::Acquire);
        if cur.is_null() { return; }
        
        let old_buf = unsafe { &*cur };
        let old_tail = old_buf.tail.load(Ordering::Acquire);
        
        // Scan buffer in reverse (most recent first) and track seen keys
        let mut seen_keys = std::collections::HashSet::new();
        let mut live_records: Vec<(usize, Rec<K, V>)> = Vec::new();
        
        for i in (0..old_tail).rev() {
            let rec = unsafe { &*old_buf.recs.add(i) };
            
            // Keep only first (most recent) occurrence of each key
            if seen_keys.insert(rec.key.clone()) {
                // Skip tombstones during compaction - only keep upserts
                if rec.kind == 0 {
                    live_records.push((i, Rec {
                        kind: rec.kind,
                        _pad: rec._pad,
                        key: rec.key.clone(),
                        value: rec.value,  // V is Copy
                    }));
                }
            }
        }
        
        // Reverse to restore chronological order (oldest first)
        live_records.reverse();
        
        let new_count = live_records.len();
        if new_count == 0 {
            // All records were tombstones or duplicates - just clear
            // Arena-allocated buffer will be freed when arena drops (no defer needed)
            let _prev = b.head.swap(core::ptr::null_mut(), Ordering::Release);
            let prev_snap = b.snapshot.swap(Shared::null(), Ordering::AcqRel, guard);
            if !prev_snap.is_null() {
                // track_defer();
                unsafe { guard.defer_unchecked(move || drop(Box::from_raw(prev_snap.as_raw() as *mut ReadTinyMap))); }
            }
            b.snapshot_tail.store(0, Ordering::Release); // No snapshot (no live records)
            return;
        }
        
        // Collect entries for TinyMap (only live upserts)
        let mut entries = Vec::with_capacity(new_count);
        for (slot_idx, (_old_idx, rec)) in live_records.iter().enumerate() {
            let h = hash64(&rec.key);
            let tag = self.tag8(h);
            entries.push((tag, h, slot_idx, rec.kind));
        }
        
        // Allocate colocated [Buffer | Recs | TinyMap] sized to fit live records with growth room
        let new_cap = (new_count * 2).max(self.initial_buffer_capacity());
        let new_buf = self.alloc_colocated_buffer(new_cap, entries);
        
        // Copy live records to new buffer
        for (slot_idx, (_old_idx, rec)) in live_records.iter().enumerate() {
            unsafe {
                let new_buf_ref = &mut *new_buf;
                let dst = new_buf_ref.recs.add(slot_idx);
                core::ptr::write(dst, Rec {
                    kind: rec.kind,
                    _pad: rec._pad,
                    key: rec.key.clone(),
                    value: rec.value,  // V is Copy
                });
            }
        }
        
        unsafe { (&mut *new_buf).tail.store(new_count, Ordering::Relaxed); }
        
        // Update bucket snapshot from colocated TinyMap
        let tinymap_ptr = unsafe { (*new_buf).tinymap_ptr };
        if !tinymap_ptr.is_null() {
            let prev_snap = b.snapshot.swap(
                unsafe { Owned::from_raw(tinymap_ptr as *mut ReadTinyMap) },
                Ordering::AcqRel,
                guard
            );
            if !prev_snap.is_null() {
                // track_defer();
                unsafe { guard.defer_unchecked(move || drop(Box::from_raw(prev_snap.as_raw() as *mut ReadTinyMap))); }
            }
            b.snapshot_tail.store(new_count, Ordering::Release); // Snapshot covers all records [0..new_count)
        }
        
        // Swap buffers - old buffer is arena-allocated and will be freed when arena drops
        let _prev = b.head.swap(new_buf, Ordering::Release);
        
        debug_log!("radix_index compact_bucket: {} records -> {} live records (cap: {})", 
                   old_tail, new_count, new_cap);
    }

    fn ensure_activated(&self, b: &Bucket<K, V>, bucket_idx: u16) {
        if !b.registered.load(Ordering::Relaxed) {
            debug_log!("radix_index activating bucket: {}", bucket_idx);
            if b.registered.compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
                let idx = bucket_idx;
                // Load current active list with epoch guard
                let guard = epoch::pin();
                let old_ptr = self.active.load(Ordering::Acquire, &guard);
                let old_vec = unsafe { old_ptr.as_ref().unwrap() };
                
                // Binary search to find insertion point (maintains sorted order for cache-friendly iteration)
                let insert_pos = old_vec.binary_search(&idx).unwrap_or_else(|pos| pos);
                
                // Build new sorted vec
                let mut new_vec = Vec::with_capacity(old_vec.len() + 1);
                new_vec.extend_from_slice(&old_vec[..insert_pos]);
                new_vec.push(idx);
                new_vec.extend_from_slice(&old_vec[insert_pos..]);
                
                // Swap with epoch RCU
                let old = self.active.swap(Owned::new(new_vec), Ordering::AcqRel, &guard);
                if !old.is_null() {
                    unsafe { guard.defer_unchecked(move || drop(old.into_owned())); }
                }
                
                // lazy buffer alloc if null
                let cur = b.head.load(Ordering::Acquire);
                if cur.is_null() {
                    let buf = self.alloc_buffer(self.initial_buffer_capacity());
                    let _ = b.head.compare_exchange(cur, buf, Ordering::Release, Ordering::Relaxed);
                }
            }
        }
    }

    fn reserve_slot<'g>(&self, b: &Bucket<K, V>) -> (*mut Buffer<K, V>, usize) {
        loop {
            let cur = b.head.load(Ordering::Acquire);
            let buf = cur as *mut Buffer<K, V>;
            let idx = unsafe { (&*buf).tail.fetch_add(1, Ordering::AcqRel) };
            if idx < unsafe { (&*buf).cap } { return (buf, idx); }
            // Need to consolidate bucket - full capacity reached
            unsafe { let buf_ref = &*buf; buf_ref.tail.fetch_sub(1, Ordering::AcqRel) };
            let guard = epoch::pin();
            self.consolidate_bucket(b, &guard);
        }
    }

    // rec_update removed: append-only model doesn't need in-place updates

    /// Rebuild snapshots for all dirty buckets.
    /// Uses active bucket list to iterate only registered buckets.
    /// Returns count of snapshots rebuilt.
    pub fn consolidate_snapshots_only(&self, guard: &epoch::Guard) -> usize {
        // Load active list with epoch guard
        let active_ptr = self.active.load(Ordering::Acquire, guard);
        let active_indices = unsafe { active_ptr.as_ref().unwrap() };
        if active_indices.is_empty() { return 0; }
        
        let mut rebuilt_count = 0;
        
        for &bidx in active_indices.iter() {
            let b = &self.buckets[bidx as usize];
            
            // Check if snapshot is stale (tail has advanced beyond snapshot coverage)
            let cur = b.head.load(Ordering::Acquire);
            if !cur.is_null() {
                let buf = unsafe { &*cur };
                let current_tail = buf.tail.load(Ordering::Acquire);
                let snapshot_tail = b.snapshot_tail.load(Ordering::Acquire);
                
                if snapshot_tail < current_tail {
                    let _wl = b.write_mx.lock().unwrap();
                    // Re-check under lock
                    let current_tail = buf.tail.load(Ordering::Acquire);
                    let snapshot_tail = b.snapshot_tail.load(Ordering::Acquire);
                    if snapshot_tail < current_tail {
                        self.rebuild_snapshot(b, guard);
                        rebuilt_count += 1;
                    }
                }
            }
        }
        
        debug_log!("radix_index rebuild_dirty_snapshots: {} buckets rebuilt", rebuilt_count);
        rebuilt_count
    }

    /// Consolidate all active buckets by deduplicating records.
    /// **Preserves tombstones** - use when RadixIndex is a delta over MPH base.
    /// This reclaims memory from obsolete record versions (updates).
    /// Returns (buckets_consolidated, total_records_before, total_records_after).
    pub fn consolidate_buckets(&self, guard: &epoch::Guard) -> (usize, usize, usize) {
        // Load active list with epoch guard
        let active_ptr = self.active.load(Ordering::Acquire, guard);
        let active_indices = unsafe { active_ptr.as_ref().unwrap() };
        if active_indices.is_empty() { return (0, 0, 0); }
        
        let mut consolidated_count = 0;
        let mut total_before = 0;
        let mut total_after = 0;
        
        for &bidx in active_indices.iter() {
            let b = &self.buckets[bidx as usize];
            
            let cur = b.head.load(Ordering::Acquire);
            if cur.is_null() { continue; }
            
            let buf = unsafe { &*cur };
            let tail_before = buf.tail.load(Ordering::Acquire);
            total_before += tail_before;
            
            // Consolidate this bucket (keeps tombstones)
            self.consolidate_bucket(b, guard);
            
            // Count after consolidation
            let cur_after = b.head.load(Ordering::Acquire);
            if !cur_after.is_null() {
                let buf_after = unsafe { &*cur_after };
                let tail_after = buf_after.tail.load(Ordering::Acquire);
                total_after += tail_after;
            }
            
            consolidated_count += 1;
        }
        
        debug_log!("radix_index consolidate_buckets: {} buckets, {} -> {} total records ({:.1}% reduction)", 
                   consolidated_count, total_before, total_after,
                   if total_before > 0 { 100.0 * (total_before - total_after) as f64 / total_before as f64 } else { 0.0 });
        
        (consolidated_count, total_before, total_after)
    }

    /// Compact all active buckets by deduplicating records and **removing tombstones**.
    /// Use when RadixIndex is standalone or when tombstones are no longer needed.
    /// This reclaims more memory than consolidate by removing deleted keys entirely.
    /// Returns (buckets_compacted, total_records_before, total_live_after).
    pub fn compact_buckets(&self, guard: &epoch::Guard) -> (usize, usize, usize) {
        // Load active list with epoch guard
        let active_ptr = self.active.load(Ordering::Acquire, guard);
        let active_indices = unsafe { active_ptr.as_ref().unwrap() };
        if active_indices.is_empty() { return (0, 0, 0); }
        
        let mut compacted_count = 0;
        let mut total_before = 0;
        let mut total_after = 0;
        
        for &bidx in active_indices.iter() {
            let b = &self.buckets[bidx as usize];
            
            let cur = b.head.load(Ordering::Acquire);
            if cur.is_null() { continue; }
            
            let buf = unsafe { &*cur };
            let tail_before = buf.tail.load(Ordering::Acquire);
            total_before += tail_before;
            
            // Compact this bucket (removes tombstones)
            self.compact_bucket(b, guard);
            
            // Count after compaction
            let cur_after = b.head.load(Ordering::Acquire);
            if !cur_after.is_null() {
                let buf_after = unsafe { &*cur_after };
                let tail_after = buf_after.tail.load(Ordering::Acquire);
                total_after += tail_after;
            }
            
            compacted_count += 1;
        }
        
        debug_log!("radix_index compact_buckets: {} buckets, {} -> {} live records ({:.1}% reduction)", 
                   compacted_count, total_before, total_after,
                   if total_before > 0 { 100.0 * (total_before - total_after) as f64 / total_before as f64 } else { 0.0 });
        
        (compacted_count, total_before, total_after)
    }

    /// Allocate [Buffer | Recs] without TinyMap (for initial allocation).
    /// TinyMap will be built later during first snapshot rebuild.
    fn alloc_buffer(&self, cap: usize) -> *mut Buffer<K, V> {
        use std::mem::{size_of, align_of};
        
        let buffer_size = size_of::<Buffer<K, V>>();
        let recs_size = cap * size_of::<Rec<K, V>>();
        let total_size = buffer_size + recs_size;
        
        // Allocate from arena
        let base_ptr = self.arena.alloc_bytes(total_size, align_of::<Buffer<K, V>>());
        
        let buf_ptr = base_ptr as *mut Buffer<K, V>;
        let recs_ptr = unsafe { base_ptr.add(buffer_size) as *mut Rec<K, V> };
        
        unsafe {
            buf_ptr.write(Buffer {
                tail: core::sync::atomic::AtomicUsize::new(0),
                cap,
                recs: recs_ptr,
                tinymap_ptr: ptr::null(), // No TinyMap initially
            });
        }
        
        buf_ptr
    }
    
    /// Allocate colocated [Buffer | Recs | TinyMap] in single arena allocation.
    /// Used during growth and consolidation for optimal memory layout.
    fn alloc_colocated_buffer(
        &self,
        cap: usize,
        tinymap_entries: Vec<(u8, u64, usize, u8)>
    ) -> *mut Buffer<K, V> {
        use std::mem::{size_of, align_of};
        
        // Calculate sizes
        let buffer_size = size_of::<Buffer<K, V>>();
        let recs_size = cap * size_of::<Rec<K, V>>();
        let recs_offset = buffer_size;
        
        // Calculate TinyMap size
        let tinymap_len = tinymap_entries.iter().filter(|(_, _, _, k)| *k == 0).count();
        let tinymap_size = ReadTinyMap::calculate_size(tinymap_len);
        let tinymap_offset = super::arena::align_up(recs_offset + recs_size, align_of::<u16>());
        
        let total_size = tinymap_offset + tinymap_size;
        
        // Allocate from arena
        let base_ptr = self.arena.alloc_bytes(total_size, align_of::<Buffer<K, V>>());
        
        let buf_ptr = base_ptr as *mut Buffer<K, V>;
        let recs_ptr = unsafe { base_ptr.add(recs_offset) as *mut Rec<K, V> };
        let tinymap_data_ptr = unsafe { base_ptr.add(tinymap_offset) };
        
        // Build TinyMap at offset
        let tinymap = ReadTinyMap::new_in_place(tinymap_data_ptr, tinymap_entries);
        
        // Allocate TinyMap struct itself on heap for epoch management
        let tinymap_ptr = Box::into_raw(Box::new(tinymap));
        
        unsafe {
            buf_ptr.write(Buffer {
                tail: core::sync::atomic::AtomicUsize::new(0),
                cap,
                recs: recs_ptr,
                tinymap_ptr,
            });
        }
        
        buf_ptr
    }

    /// Collect comprehensive statistics for diagnostic analysis.
    pub fn collect_stats(&self, guard: &epoch::Guard) -> RadixIndexStats {
        // let buckets = self.buckets;
        let bucket_count = self.buckets.len();
        
        // Get active bucket list with epoch guard
        let active_ptr = self.active.load(Ordering::Acquire, guard);
        let active_indices = unsafe { active_ptr.as_ref().unwrap() };
        
        let mut bucket_details = Vec::with_capacity(active_indices.len());
        
        // Collect per-bucket statistics
        for &idx in active_indices.iter() {
            let b = &self.buckets[idx as usize];
            let head_ptr = b.head.load(Ordering::Acquire);
            
            if head_ptr.is_null() {
                continue;
            }
            
            let buf = unsafe { &*head_ptr };
            let tail = buf.tail.load(Ordering::Relaxed);
            let total_records = tail; // Total records (upserts + tombstones)
            
            // Count keys and analyze tag8 distribution
            let mut key_count = 0;
            let mut tag_counts: HashMap<u8, usize> = HashMap::new();
            
            for i in 0..tail {
                let rec = unsafe { &*buf.recs.add(i) };
                // kind: 0=Upsert, 1=Tombstone
                if rec.kind == 0 {
                    key_count += 1;
                    // Compute tag8 from the key
                    let h = hash64(&rec.key);
                    let tag = tag8_from_hash_disjoint(h, self.bucket_bits);
                    *tag_counts.entry(tag).or_insert(0) += 1;
                }
            }
            
            let unique_tags = tag_counts.len();
            let tag8_collisions = tag_counts.values().filter(|&&count| count > 1).count();
            
            // Get ReadTinyMap size
            let snap = b.snapshot.load(Ordering::Acquire, guard);
            let tinymap_size = if snap.is_null() {
                0
            } else {
                unsafe { (*snap.as_raw()).len() }
            };
            
            bucket_details.push(BucketStats {
                bucket_idx: idx as usize,
                key_count,
                total_records,
                buffer_capacity: buf.cap,
                tinymap_size,
                growth_count: 0, // TODO: track this if needed
                tag8_collisions,
                unique_tags,
            });
        }
        
        // Get actual arena statistics
        let arena_stats = self.arena.stats();
        let hotpath_arena = arena_stats.clone();
        
        // Deprecated fields (kept for API compatibility)
        let buffer_arena = super::arena::ArenaStats::empty();
        let record_arena = super::arena::ArenaStats::empty();
        
        let mut stats = RadixIndexStats {
            total_buckets: bucket_count,
            bucket_bits: self.bucket_bits,
            active_buckets: 0,
            total_keys: 0,
            bucket_utilization: 0.0,
            avg_bucket_depth: 0.0,
            max_bucket_depth: 0,
            min_bucket_depth: 0,
            bucket_depth_stddev: 0.0,
            bucket_distribution_entropy: 0.0,
            avg_tag16_uniqueness: 0.0,
            hotpath_arena,
            buffer_arena,
            record_arena,
            bucket_details,
        };
        
        // Calculate derived statistics
        stats.calculate_derived_stats();
        
        stats
    }

}

impl<K, V> Drop for RadixIndex<K, V> {
    fn drop(&mut self) {
        // Track drop
        // EPOCH_STATS.register_radix_drop();
        
        // Print stats every 1000 drops
        // let drops = EPOCH_STATS.snapshot().radix_dropped;
        // if drops % 10000 == 0 {
        //     let snap = EPOCH_STATS.snapshot();
        //     snap.print_summary();
        // }
        
        // NOTE: We used to call flush() here, but with 500K+ rapid drops,
        // calling flush() on an empty defer queue causes epoch's internal
        // bookkeeping to allocate pathologically (256MB allocation failure).
        // 
        // Since we have 0 defers in the hot path (no snapshots built for n=4),
        // and arena-allocated buffers are cleaned up with the arena,
        // there's nothing that needs epoch cleanup on drop.
        //
        // If we add defers back (e.g., for snapshots), we'll need a different
        // strategy - perhaps batch flushes or lazy cleanup.
    }
}

// Unit tests moved to /tests/radix_index_tests.rs

