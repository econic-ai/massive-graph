# Radix Index – Next State Implementation Plan

## Executive Summary

This plan details the implementation of a lock-free, fixed-capacity radix index that eliminates mutex contention, reduces cache misses, and provides predictable performance through a 64-bit atomic mask-driven design.

**Key Changes:**
- Remove all locks (grow_mx, write_mx)
- Remove RCU/epoch-based TinyMap snapshots
- Remove active bucket tracking vector
- Add parallel metadata array (mask + tags)
- Add deterministic slot assignment
- Fixed-capacity buckets (panic on full, future: rebuild)

---

## 🔥 Hot-Path Analysis & Optimization Targets

### Critical Performance Paths (Must Optimize)

#### 1. **READ PATH: `get_with_hash()`**
**Current Architecture:**
- 3-5 atomic loads: bucket head ptr, snapshot ptr, snapshot_tail, buffer tail
- RCU pointer dereference
- TinyMap binary search on tags
- Full key comparison

**Target Architecture:**
- 1 atomic load: 64-bit mask with `Acquire`
- Deterministic preferred slot calculation (pure compute, no memory access)
- Bit-scan forward from preferred slot (CPU-optimized)
- Tag comparison before key comparison (early rejection)
- Direct array access to records

**Optimization Strategies:**
1. **Single Atomic Load:** `mask.load(Ordering::Acquire)` - ensures all subsequent reads see consistent state
2. **Deterministic Slot:** `preferred_slot = (hash >> (bucket_bits + 8)) & (bucket_slots - 1)` - no memory access
3. **Bit-Scan Walk:** Use hardware-accelerated bit manipulation:
   ```rust
   let mut probe_offset = 0;
   while probe_offset < bucket_slots {
       let slot = (preferred + probe_offset) & (bucket_slots - 1);
       if mask & (1u64 << slot) != 0 {
           // Check tag, then key
       }
       probe_offset += 1;
   }
   ```
4. **Early Tag Rejection:** Compare `tags[slot]` before full key (saves expensive key comparison)
5. **Cache-Line Locality:** Place mask and first 8 tags in same 64-byte cache line

**Performance Target:**
- Empty bucket: < 10 cycles (1 atomic + 1 branch)
- Found key (no collisions): < 50 cycles (1 atomic + 1-2 tag checks + 1 key check)
- Not found: < 100 cycles (1 atomic + scan all live slots)

**Critical Code Sections:**
```rust
#[inline(always)]
pub fn get_with_hash<'g>(&'g self, key: &K, hash: u64) -> Option<&'g V> {
    let bidx = self.bucket_index(hash);
    let tag = self.tag8(hash);
    let preferred = self.preferred_slot(hash);
    
    // HOT PATH: Single atomic load
    let mask = self.bucket_meta[bidx].mask.load(Ordering::Acquire);
    if mask == 0 { return None; } // Fast path for empty bucket
    
    // HOT PATH: Deterministic probe
    let bucket = &self.buckets[bidx];
    let tags = &self.bucket_meta[bidx].tags;
    
    for offset in 0..self.bucket_slots {
        let slot = (preferred + offset) & (self.bucket_slots - 1);
        
        // HOT PATH: Check bit before memory access
        if (mask & (1u64 << slot)) == 0 { continue; }
        
        // HOT PATH: Tag comparison before key comparison
        if tags[slot] != tag { continue; }
        
        // HOT PATH: Full key comparison
        let rec = &bucket.recs[slot];
        if &rec.key == key {
            return match rec.kind {
                0 => Some(&rec.value),
                _ => None,
            };
        }
    }
    None
}
```

---

#### 2. **WRITE PATH - NEW INSERT: `upsert()` with new key**
**Current Architecture:**
- Mutex acquisition (write_mx)
- atomic fetch_add on tail
- Check capacity, potential consolidation
- Write record
- Release mutex

**Target Architecture:**
- Compute hash → bucket → tag → preferred slot (pure compute)
- Bit-scan for first empty slot (0-bit in mask)
- Write record to slot
- Write tag to tags array
- Single atomic fetch_or to publish

**Optimization Strategies:**
1. **Lock-Free:** No mutex acquisition - only atomic mask operation
2. **Bit-Scan for Empty Slot:** Find first 0-bit from preferred position:
   ```rust
   let mask_current = mask.load(Ordering::Acquire);
   let mut probe_offset = 0;
   while probe_offset < bucket_slots {
       let slot = (preferred + probe_offset) & (bucket_slots - 1);
       if (mask_current & (1u64 << slot)) == 0 {
           // Found empty slot
           break;
       }
       probe_offset += 1;
   }
   ```
3. **Write-Before-Publish:** Establish happens-before relationship:
   ```rust
   // Write record first (no atomic needed)
   bucket.recs[slot] = Rec { kind: 0, key: key.clone(), value: *value, .. };
   
   // Write tag second (no atomic needed)
   bucket_meta.tags[slot] = tag;
   
   // Publish with Release ordering (synchronizes with readers' Acquire)
   bucket_meta.mask.fetch_or(1u64 << slot, Ordering::Release);
   ```
4. **Full Bucket Handling:** If no empty slot found, panic (future: trigger rebuild)

**Performance Target:**
- New insert (empty bucket): < 30 cycles
- New insert (partial bucket): < 80 cycles
- Full bucket: panic (rebuild path out of scope)

**Critical Code Sections:**
```rust
#[inline(always)]
pub fn upsert(&self, key: &K, value: &V) {
    let hash = hash64(key);
    let bidx = self.bucket_index(hash);
    let tag = self.tag8(hash);
    let preferred = self.preferred_slot(hash);
    
    // HOT PATH: Find empty slot
    let mask = self.bucket_meta[bidx].mask.load(Ordering::Acquire);
    let mut slot = None;
    
    for offset in 0..self.bucket_slots {
        let s = (preferred + offset) & (self.bucket_slots - 1);
        if (mask & (1u64 << s)) == 0 {
            slot = Some(s);
            break;
        }
    }
    
    let slot = slot.expect("Bucket full - rebuild not implemented");
    
    // HOT PATH: Write before publish
    let bucket = &self.buckets[bidx];
    let meta = &self.bucket_meta[bidx];
    
    // Non-atomic writes (safe because slot is unpublished)
    bucket.recs[slot] = Rec { 
        kind: 0, 
        _pad: [0; 7], 
        key: key.clone(), 
        value: *value 
    };
    meta.tags[slot] = tag;
    
    // HOT PATH: Single atomic to publish
    meta.mask.fetch_or(1u64 << slot, Ordering::Release);
}
```

---

#### 3. **WRITE PATH - UPDATE: `upsert()` with existing key**
**Current Architecture:**
- Mutex acquisition
- Reverse scan to find existing key
- Append new record
- Update snapshot_tail
- Release mutex

**Target Architecture:**
- Locate old slot (same as read path)
- Find new empty slot (same as insert path)
- Write new record
- CAS mask: atomic swap of old→new bits

**Optimization Strategies:**
1. **Lock-Free Update:** Single 64-bit CAS for atomic old→new swap:
   ```rust
   loop {
       let old_mask = mask.load(Ordering::Acquire);
       let new_mask = (old_mask | (1u64 << new_slot)) & !(1u64 << old_slot);
       
       match mask.compare_exchange(
           old_mask, new_mask,
           Ordering::AcqRel, Ordering::Acquire
       ) {
           Ok(_) => break,
           Err(_) => continue, // Retry with new mask
       }
   }
   ```
2. **Bounded Retry:** At most `bucket_slots` iterations (≤ 64)
3. **Reader Consistency:** Readers see either old value OR new value, never partial state
4. **Write New Before CAS:** Ensure new slot is fully written before publication

**Performance Target:**
- Update (no contention): < 100 cycles
- Update (with CAS retry): < 150 cycles

**Critical Code Sections:**
```rust
#[inline]
pub fn upsert_update(&self, key: &K, value: &V, hash: u64) {
    let bidx = self.bucket_index(hash);
    let tag = self.tag8(hash);
    let preferred = self.preferred_slot(hash);
    
    // HOT PATH: Find old slot
    let old_slot = self.find_slot_by_key(bidx, key, hash, preferred);
    let old_slot = old_slot.expect("Key must exist for update");
    
    // HOT PATH: Find new empty slot
    let new_slot = self.find_empty_slot(bidx, preferred);
    let new_slot = new_slot.expect("Bucket full - rebuild not implemented");
    
    // HOT PATH: Write new record before publishing
    let bucket = &self.buckets[bidx];
    let meta = &self.bucket_meta[bidx];
    
    bucket.recs[new_slot] = Rec { 
        kind: 0, 
        _pad: [0; 7], 
        key: key.clone(), 
        value: *value 
    };
    meta.tags[new_slot] = tag;
    
    // HOT PATH: Atomic old→new swap with CAS
    loop {
        let old_mask = meta.mask.load(Ordering::Acquire);
        let new_mask = (old_mask | (1u64 << new_slot)) & !(1u64 << old_slot);
        
        match meta.mask.compare_exchange(
            old_mask, new_mask,
            Ordering::AcqRel, Ordering::Acquire
        ) {
            Ok(_) => break,
            Err(_) => continue,
        }
    }
}
```

---

#### 4. **ITERATION PATH: `iter()` / `iter_with_keys()`**
**Current Architecture:**
- Load active bucket vector with epoch guard (1 atomic)
- Per bucket:
  - Load snapshot ptr (1 atomic)
  - Load buffer head (1 atomic)
  - Iterate TinyMap slots
  - Dereference records through buffer ptr
- Total: 2N+1 atomics for N active buckets

**Target Architecture:**
- Hoist all masks at start: 1 atomic per bucket, sequential access
- Store in local Vec<u64> or window (64 buckets)
- Iterate over local copy - ZERO atomics during yield
- Skip buckets with mask == 0

**Optimization Strategies:**
1. **Mask Hoisting:** Load all masks upfront in sequential pass:
   ```rust
   let masks: Vec<u64> = (0..self.buckets.len())
       .map(|i| self.bucket_meta[i].mask.load(Ordering::Acquire))
       .collect();
   ```
2. **Sequential Access:** Cache-friendly traversal of metadata array
3. **Zero-Atomic Yield:** Once masks are hoisted, no atomics during iteration
4. **Early Skip:** `if masks[i] == 0 { continue; }` avoids touching bucket data
5. **Optional Windowing:** Process 64 buckets at a time to keep working set small:
   ```rust
   const WINDOW_SIZE: usize = 64;
   for window_start in (0..self.buckets.len()).step_by(WINDOW_SIZE) {
       let window_end = (window_start + WINDOW_SIZE).min(self.buckets.len());
       let window_masks: [u64; WINDOW_SIZE] = ...;
       // Iterate using window_masks
   }
   ```

**Performance Target:**
- Mask hoisting overhead: < 1ns per bucket (just atomic load)
- Per-entry yield: < 10ns
- Empty bucket skip: < 1ns

**Critical Code Sections:**
```rust
pub fn iter<'g>(&'g self) -> impl Iterator<Item = &'g V> + 'g {
    // HOT PATH: Hoist all masks upfront (sequential access)
    let masks: Vec<u64> = (0..self.buckets.len())
        .map(|i| self.bucket_meta[i].mask.load(Ordering::Acquire))
        .collect();
    
    struct State<'a, K, V> {
        idx: &'a RadixIndex<K, V>,
        masks: Vec<u64>,
        bucket_pos: usize,
        slot_pos: usize,
    }
    
    let mut st = State {
        idx: self,
        masks,
        bucket_pos: 0,
        slot_pos: 0,
    };
    
    std::iter::from_fn(move || {
        loop {
            // HOT PATH: Check local mask copy (no atomics)
            if st.bucket_pos >= st.idx.buckets.len() { return None; }
            
            let mask = st.masks[st.bucket_pos];
            
            // HOT PATH: Skip empty buckets
            if mask == 0 {
                st.bucket_pos += 1;
                st.slot_pos = 0;
                continue;
            }
            
            // HOT PATH: Scan bits in mask
            while st.slot_pos < st.idx.bucket_slots {
                let slot = st.slot_pos;
                st.slot_pos += 1;
                
                if (mask & (1u64 << slot)) != 0 {
                    let rec = &st.idx.buckets[st.bucket_pos].recs[slot];
                    if rec.kind == 0 {
                        return Some(&rec.value);
                    }
                }
            }
            
            // Move to next bucket
            st.bucket_pos += 1;
            st.slot_pos = 0;
        }
    })
}
```

---

#### 5. **DELETE PATH: `delete()`**
**Current Architecture:**
- Mutex acquisition
- Append tombstone record
- Release mutex

**Target Architecture:**
- Locate slot (same as read path)
- Optionally mark kind=1 (tombstone) in record
- Single atomic fetch_and to unpublish bit

**Optimization Strategies:**
1. **Lock-Free:** No mutex, just atomic mask update
2. **Single Atomic:** `mask.fetch_and(!(1u64 << slot), Ordering::Release)`
3. **Tombstone Optional:** For debugging/rebuild, can mark kind=1 first
4. **Reader Grace Period:** Readers with old mask may still see slot (acceptable)

**Performance Target:**
- Delete: < 80 cycles

**Critical Code Sections:**
```rust
#[inline]
pub fn delete(&self, key: &K) {
    let hash = hash64(key);
    let bidx = self.bucket_index(hash);
    let preferred = self.preferred_slot(hash);
    
    // HOT PATH: Find slot
    let slot = self.find_slot_by_key(bidx, key, hash, preferred);
    let slot = match slot {
        Some(s) => s,
        None => return, // Key not found
    };
    
    // Optional: mark tombstone for debugging/rebuild
    let bucket = &self.buckets[bidx];
    bucket.recs[slot].kind = 1;
    
    // HOT PATH: Single atomic to unpublish
    self.bucket_meta[bidx].mask.fetch_and(!(1u64 << slot), Ordering::Release);
}
```

---

### Secondary Hot-Path Operations

#### 6. **Hash Bit Extraction**
**Requirement:** Three disjoint bit windows from single 64-bit hash

**Implementation:**
```rust
// Bucket selection: low `bucket_bits`
#[inline(always)]
fn bucket_index(&self, hash: u64) -> usize {
    (hash as usize) & ((1usize << self.bucket_bits) - 1)
}

// fp8 tag: bits [bucket_bits .. bucket_bits+8]
#[inline(always)]
fn tag8(&self, hash: u64) -> u8 {
    ((hash >> self.bucket_bits) & 0xFF) as u8
}

// Preferred slot: bits [bucket_bits+8 .. bucket_bits+8+slot_bits]
#[inline(always)]
fn preferred_slot(&self, hash: u64) -> usize {
    let slot_bits = (self.bucket_slots - 1).count_ones(); // log2(bucket_slots)
    let shift = self.bucket_bits + 8;
    ((hash >> shift) as usize) & (self.bucket_slots - 1)
}
```

**Performance Target:**
- All three functions: < 5 cycles (pure bit manipulation)

---

#### 7. **Bit-Scan Helpers**
**Requirement:** Fast slot search with wrap-around

**Implementation:**
```rust
// Find slot containing key (returns Option<usize>)
#[inline]
fn find_slot_by_key(&self, bidx: usize, key: &K, hash: u64, preferred: usize) -> Option<usize> {
    let mask = self.bucket_meta[bidx].mask.load(Ordering::Acquire);
    let tag = self.tag8(hash);
    let tags = &self.bucket_meta[bidx].tags;
    let bucket = &self.buckets[bidx];
    
    for offset in 0..self.bucket_slots {
        let slot = (preferred + offset) & (self.bucket_slots - 1);
        
        if (mask & (1u64 << slot)) == 0 { continue; }
        if tags[slot] != tag { continue; }
        
        let rec = &bucket.recs[slot];
        if &rec.key == key {
            return Some(slot);
        }
    }
    None
}

// Find empty slot (returns Option<usize>)
#[inline]
fn find_empty_slot(&self, bidx: usize, preferred: usize) -> Option<usize> {
    let mask = self.bucket_meta[bidx].mask.load(Ordering::Acquire);
    
    for offset in 0..self.bucket_slots {
        let slot = (preferred + offset) & (self.bucket_slots - 1);
        
        if (mask & (1u64 << slot)) == 0 {
            return Some(slot);
        }
    }
    None
}

// Fast path: count live entries
#[inline]
fn count_live_entries(&self, bidx: usize) -> u32 {
    let mask = self.bucket_meta[bidx].mask.load(Ordering::Relaxed);
    mask.count_ones()
}
```

---

## 📋 Implementation Roadmap

### Phase 1: Core Structure Refactoring (2-3 days)

**Step 1.1: Define New Types**
- [ ] Add `BucketMeta` struct:
  ```rust
  struct BucketMeta {
      mask: AtomicU64,
      tags: Box<[u8]>,
  }
  ```
- [ ] Simplify `Bucket` struct:
  ```rust
  struct Bucket<K, V> {
      recs: Box<[Rec<K, V>]>,
  }
  ```
- [ ] Update `RadixIndex` fields:
  ```rust
  pub struct RadixIndex<K, V> {
      buckets: Box<[Bucket<K, V>]>,
      bucket_meta: Box<[BucketMeta]>,
      bucket_bits: u32,
      bucket_slots: usize,
      arena: Arena,  // Keep for potential future use
  }
  ```
- [ ] Keep `Rec<K, V>` unchanged (already correct)

**Step 1.2: Add Hash Utility Functions**
- [ ] Add `preferred_slot_from_hash()` to `util.rs`:
  ```rust
  pub fn preferred_slot_from_hash(h: u64, bucket_bits: u32, bucket_slots: usize) -> usize {
      let slot_bits = (bucket_slots - 1).count_ones();
      let shift = bucket_bits + 8;
      ((h >> shift) as usize) & (bucket_slots - 1)
  }
  ```
- [ ] Add assertion for disjoint bit windows (debug mode):
  ```rust
  debug_assert!(bucket_bits + 8 + slot_bits <= 64, "Hash bit windows exceed 64 bits");
  ```

**Step 1.3: Update Constructor**
- [ ] Modify `with_capacity()`:
  - Calculate `bucket_slots` (max 64, power of 2)
  - Allocate `buckets` with pre-sized `recs` arrays
  - Allocate `bucket_meta` with masks (init to 0) and tags arrays
  - Remove `active`, `initial_buffer_capacity`, `theoretical_max_per_bucket`
- [ ] Calculate optimal `bucket_slots` based on expected distribution:
  ```rust
  let entries_per_bucket = max_capacity / bucket_count;
  let bucket_slots = entries_per_bucket.next_power_of_two().min(64).max(8);
  ```

**Step 1.4: Update Drop Implementation**
- [ ] Remove epoch flush calls
- [ ] Keep arena drop (automatic cleanup)
- [ ] Remove RCU cleanup logic

---

### Phase 2: Read Path Implementation (1-2 days)

**Step 2.1: Implement Helper Functions**
- [ ] Implement `find_slot_by_key()` (see Hot-Path #6)
- [ ] Add inline attributes: `#[inline(always)]` for hot functions
- [ ] Add safety comments for any unsafe access patterns

**Step 2.2: Implement `get_with_hash()`**
- [ ] Replace current implementation with mask-driven approach (see Hot-Path #1)
- [ ] Remove all epoch guard dependencies
- [ ] Remove TinyMap lookups
- [ ] Remove snapshot_tail checks

**Step 2.3: Update `get()`**
- [ ] Simplify to just call `get_with_hash()`
- [ ] Remove epoch::pin() call

**Step 2.4: Update `get_copy()`**
- [ ] Keep as-is (just calls `get().copied()`)

**Hot-Path Validation Checklist:**
- [ ] Single atomic load per read
- [ ] No mutex acquisition
- [ ] No epoch guard needed
- [ ] Tag comparison before key comparison
- [ ] Predictable probe order from preferred slot

---

### Phase 3: Write Path Implementation (2-3 days)

**Step 3.1: Implement Helper Functions**
- [ ] Implement `find_empty_slot()` (see Hot-Path #6)
- [ ] Add panic path for full bucket with diagnostic info

**Step 3.2: Implement New Key Insert**
- [ ] Replace `upsert()` with mask-driven approach (see Hot-Path #2)
- [ ] Remove mutex acquisition
- [ ] Remove `ensure_activated()` call
- [ ] Remove `reserve_slot()` call
- [ ] Implement write-before-publish pattern

**Step 3.3: Implement Update Path**
- [ ] Detect existing key in `upsert()`
- [ ] Implement old→new CAS swap (see Hot-Path #3)
- [ ] Add bounded retry loop (max 100 iterations)
- [ ] Add diagnostic logging for excessive retries

**Step 3.4: Implement Delete**
- [ ] Replace `delete()` with mask-driven approach (see Hot-Path #5)
- [ ] Remove mutex acquisition
- [ ] Remove append-tombstone logic
- [ ] Implement fetch_and unpublish

**Hot-Path Validation Checklist:**
- [ ] No mutex acquisition
- [ ] Write-before-publish ordering
- [ ] Single atomic for insert (fetch_or)
- [ ] Single atomic for delete (fetch_and)
- [ ] Single CAS for update
- [ ] Bounded retry loops

---

### Phase 4: Iteration Path Implementation (1-2 days)

**Step 4.1: Implement Mask Hoisting**
- [ ] Add `hoist_masks()` helper (see Hot-Path #4)
- [ ] Implement sequential mask loading
- [ ] Optional: Add windowing for large bucket counts

**Step 4.2: Implement `iter()`**
- [ ] Replace current implementation (see Hot-Path #4)
- [ ] Remove active bucket vector loading
- [ ] Remove epoch guard dependencies
- [ ] Remove TinyMap iteration
- [ ] Add early skip for mask == 0

**Step 4.3: Implement `iter_with_keys()`**
- [ ] Same as `iter()` but yield `(&K, &V)`
- [ ] Reuse mask hoisting logic

**Hot-Path Validation Checklist:**
- [ ] Masks hoisted once at start
- [ ] Zero atomics during yield loop
- [ ] Cache-friendly sequential access
- [ ] Early skip for empty buckets

---

### Phase 5: Cleanup & Removal (1 day)

**Step 5.1: Remove Old Fields**
- [ ] Remove from `Bucket`:
  - `head: AtomicPtr<Buffer<K, V>>`
  - `grow_mx: Mutex<()>`
  - `write_mx: Mutex<()>`
  - `registered: AtomicBool`
  - `snapshot: epoch::Atomic<ReadTinyMap>`
  - `snapshot_tail: AtomicUsize`
- [ ] Remove from `RadixIndex`:
  - `active: epoch::Atomic<Vec<u16>>`
  - `initial_buffer_capacity: usize`
  - `theoretical_max_per_bucket: usize`
- [ ] Remove `Buffer<K, V>` struct entirely

**Step 5.2: Remove Old Methods**
- [ ] Remove `rebuild_snapshot()`
- [ ] Remove `consolidate_bucket()`
- [ ] Remove `compact_bucket()`
- [ ] Remove `consolidate_buckets()`
- [ ] Remove `compact_buckets()`
- [ ] Remove `consolidate_snapshots_only()`
- [ ] Remove `ensure_activated()`
- [ ] Remove `reserve_slot()`
- [ ] Remove `alloc_buffer()`
- [ ] Remove `alloc_colocated_buffer()`

**Step 5.3: Update `clear_all()`**
- [ ] Remove epoch guard parameter
- [ ] Replace with: iterate all buckets, set mask to 0
- [ ] No RCU cleanup needed

**Step 5.4: Update `collect_stats()`**
- [ ] Remove active bucket vector loading
- [ ] Remove epoch guard parameter
- [ ] Iterate all buckets directly
- [ ] Use `count_ones()` on masks for live entry counts
- [ ] Remove TinyMap size tracking

---

### Phase 6: Testing & Validation (2-3 days)

**Step 6.1: Unit Tests**
- [ ] Test empty index: `get()` returns None
- [ ] Test single insert: `upsert()` then `get()` returns value
- [ ] Test update: `upsert()` twice same key, `get()` returns new value
- [ ] Test delete: `upsert()` then `delete()` then `get()` returns None
- [ ] Test collision handling: multiple keys to same bucket
- [ ] Test deterministic slot assignment: verify preferred slot calculation
- [ ] Test full bucket panic: insert > bucket_slots entries to same bucket
- [ ] Test iteration correctness: verify all entries yielded exactly once
- [ ] Test iteration with empty buckets: verify skipping
- [ ] Test concurrent reads: spawn multiple readers
- [ ] Test concurrent writes: spawn multiple writers
- [ ] Test concurrent read+write: mixed workload

**Step 6.2: Correctness Tests**
- [ ] Verify disjoint hash bit windows (no overlap)
- [ ] Verify mask publication ordering (write before publish)
- [ ] Verify CAS update atomicity (readers never see partial state)
- [ ] Verify delete visibility (readers eventually see deletion)
- [ ] Verify iteration staleness (acceptable to see stale data, not torn data)

**Step 6.3: Performance Tests**
- [ ] Benchmark `get()` on empty index (target: < 10 cycles)
- [ ] Benchmark `get()` on populated index (target: < 50 cycles)
- [ ] Benchmark `upsert()` new key (target: < 30 cycles)
- [ ] Benchmark `upsert()` existing key (target: < 100 cycles)
- [ ] Benchmark `delete()` (target: < 80 cycles)
- [ ] Benchmark `iter()` on large index (target: < 10ns per entry)
- [ ] Measure cache miss rates with `perf stat`
- [ ] Measure lock contention (should be zero)
- [ ] Compare vs current implementation (baseline metrics)

**Step 6.4: Integration Tests**
- [ ] Test with `OptimisedIndex` integration
- [ ] Test concurrent read/write workloads (multiple threads)
- [ ] Test large-scale insertion (100K+ keys)
- [ ] Test realistic workload patterns (read-heavy, write-heavy, mixed)
- [ ] Verify no memory leaks (valgrind or similar)
- [ ] Verify no data races (ThreadSanitizer)

---

## 🎯 Performance Targets Summary

| Operation | Current | Target | Improvement |
|-----------|---------|--------|-------------|
| `get()` - empty bucket | ~20 cycles | < 10 cycles | 2x |
| `get()` - found | ~100 cycles | < 50 cycles | 2x |
| `upsert()` - new key | ~200 cycles | < 30 cycles | 6-7x |
| `upsert()` - update | ~250 cycles | < 100 cycles | 2-3x |
| `delete()` | ~220 cycles | < 80 cycles | 2-3x |
| `iter()` - per entry | ~30ns | < 10ns | 3x |
| Cache misses | ~2-3 per op | < 1 per op | 2-3x |
| Lock contention | High | Zero | ∞ |

---

## 🚨 Risk Mitigation

### Risk 1: Hash Bit Window Overlap
**Impact:** Keys may collide incorrectly or preferred slots may be biased
**Mitigation:**
- Add debug assertions in constructor
- Unit test with known hash values
- Document bit layout clearly

### Risk 2: Full Bucket Panic
**Impact:** Production crashes if bucket fills
**Mitigation:**
- Document limitation in public API docs
- Add metrics tracking for bucket fullness
- Plan Phase 7 rebuild path
- Choose bucket_slots conservatively (e.g., 2x expected per bucket)

### Risk 3: CAS Update Contention
**Impact:** High contention may cause excessive retries
**Mitigation:**
- Bounded retry loop (max 100 iterations)
- Add diagnostic logging for excessive retries
- Benchmark under high-contention workloads
- Consider backoff strategy if needed

### Risk 4: Iteration Staleness
**Impact:** Iterators may see stale data (old values)
**Mitigation:**
- Document in API: iterators provide snapshot-at-start semantics
- Acceptable for delta index use case
- Test that torn reads don't occur (single-bit vs multi-bit changes)

### Risk 5: Memory Ordering Bugs
**Impact:** Data races, torn reads, visibility issues
**Mitigation:**
- Explicit Acquire/Release ordering throughout
- Safety comments on all unsafe blocks
- ThreadSanitizer validation
- Miri validation (if feasible)

---

## 📝 Code Quality Requirements

- [ ] All functions have single-line doc comments
- [ ] All warnings fixed (zero warnings policy)
- [ ] Hot-path functions marked `#[inline(always)]` or `#[inline]`
- [ ] Atomic orderings explicitly documented
- [ ] Unsafe blocks have safety comments
- [ ] No dead code remaining
- [ ] No commented-out code
- [ ] Consistent naming conventions
- [ ] Update/correct existing comments (don't delete)

---

## 🔗 Dependencies

### New Dependencies
- None (use std library atomics and bit manipulation)

### Removed Dependencies
- Potentially remove `crossbeam_epoch` if only used for RadixIndex
  - **Action:** Search codebase for other epoch usages before removing

### Keep Dependencies
- `ahash` (for `hash64()`)
- `Arena` (keep for future use, currently unused in new design)

---

## ⏱️ Timeline Estimate

| Phase | Duration | Tasks |
|-------|----------|-------|
| Phase 1: Core Structure | 2-3 days | Types, constructor, cleanup |
| Phase 2: Read Path | 1-2 days | `get()` implementation |
| Phase 3: Write Path | 2-3 days | `upsert()`, `delete()` |
| Phase 4: Iteration Path | 1-2 days | `iter()` implementation |
| Phase 5: Cleanup | 1 day | Remove old code |
| Phase 6: Testing | 2-3 days | Unit, perf, integration tests |
| **Total** | **9-14 days** | |

---

## 🔍 Hot-Path Summary

### Critical Optimizations (Must Have)

1. **Single Atomic Read Load** (`get()`)
   - Location: `get_with_hash()` first line
   - Target: 1 atomic load with Acquire ordering
   - Benefit: Eliminates 3-4 atomic loads from current implementation

2. **Lock-Free Write Publication** (`upsert()` new key)
   - Location: After writing record and tag
   - Target: Single `fetch_or()` with Release ordering
   - Benefit: Eliminates mutex acquisition (major contention point)

3. **CAS-Based Update** (`upsert()` existing key)
   - Location: After writing new record
   - Target: Single 64-bit CAS with bounded retry
   - Benefit: Lock-free update with atomic old→new swap

4. **Mask Hoisting for Iteration** (`iter()`)
   - Location: Start of iterator construction
   - Target: Sequential load of all masks, then zero atomics
   - Benefit: Eliminates 2N+1 atomics from current implementation

5. **Deterministic Slot Assignment**
   - Location: All read/write paths
   - Target: Pure computation, no memory access
   - Benefit: Predictable probe order, no pointer chasing

### Secondary Optimizations (Should Have)

6. **Tag-Before-Key Comparison**
   - Location: All read paths
   - Benefit: Early rejection saves expensive key comparisons

7. **Early Bucket Skip**
   - Location: Iteration path
   - Benefit: Avoids touching empty bucket data

8. **Cache-Line Alignment**
   - Location: `BucketMeta` array layout
   - Benefit: Mask + first 8 tags in same cache line

---

## 📚 Documentation Requirements

- [ ] Update `RadixIndex` struct doc comment with new architecture
- [ ] Document mask publication rules (3 types)
- [ ] Document disjoint hash bit windows
- [ ] Document full bucket behavior (panic)
- [ ] Document iteration staleness semantics
- [ ] Add inline code examples to public methods
- [ ] Update architectural docs in `/docs`

---

## ✅ Definition of Done

A phase is considered complete when:
1. All checklist items are marked done
2. Code compiles without warnings
3. All tests pass
4. Performance targets met (if applicable)
5. Code reviewed (self or peer)
6. Documentation updated

The entire project is considered complete when:
1. All phases complete
2. All performance targets met or exceeded
3. Zero known bugs
4. Integration tests pass
5. Benchmarks show improvement over baseline
6. Documentation is accurate and complete

---

## 🎯 Success Criteria

This implementation will be considered successful if:

1. **Performance:** All hot-path operations meet or exceed target cycle counts
2. **Correctness:** Zero data races, no torn reads, no memory leaks
3. **Scalability:** Linear scaling to 100K+ keys with predictable performance
4. **Simplicity:** Codebase is simpler than current implementation (fewer lines, fewer concepts)
5. **Reliability:** No panics under normal workload (within bucket capacity limits)

---

## 🔄 Future Work (Phase 7+)

Out of scope for initial implementation, but planned:

1. **Full Bucket Rebuild Path**
   - Replace panic with slow-path rebuild
   - Double bucket count or increase bucket_slots
   - Migrate existing records to new structure

2. **Adaptive Bucket Sizing**
   - Monitor bucket fullness
   - Trigger rebuild before reaching capacity
   - Optimize for actual distribution

3. **Memory Compaction**
   - Reclaim space from deleted entries
   - Rebuild buckets with high delete rates

4. **Advanced Bit-Scan**
   - Use SIMD for parallel slot scanning
   - Hardware accelerated popcount/ctz

5. **Epoch Removal**
   - Evaluate removing crossbeam_epoch entirely
   - Simpler memory model without RCU

---

## 📖 References

- [Zero Overhead Storage Architecture Implementation.md](./docs/Zero%20Overhead%20Storage%20Architecture%20Implementation.md)
- [Delta Processing and Propagation Architecture.md](./docs/Delta%20Processing%20and%20Propagation%20Architecture.md)
- [Memory Storage Architecture v2.md](./docs/Memory%20Storage%20Architecture%20v2.md)
- Current implementation: `radix_index.rs` (lines 1-1070)
- Hash utilities: `util.rs` (lines 1-42)
- Arena allocator: `arena.rs` (lines 1-134)

---

**Document Status:** Ready for Implementation
**Last Updated:** 2025-11-02
**Author:** AI Assistant (with user consultation)
**Approval:** Pending user review

