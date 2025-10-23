# Radix Index Hot-Path Quick Reference

This document provides a condensed view of the critical hot-path operations that require special optimization attention during implementation.

---

## 🔥 The Five Critical Hot Paths

### 1. READ: `get_with_hash()` - THE HOTTEST PATH

**Why Critical:** Most frequently called operation, directly impacts query latency

**Target:** < 50 cycles (found), < 10 cycles (empty bucket)

**Optimizations:**
- ✅ **Single atomic load** of mask with `Acquire`
- ✅ **Early exit** if mask == 0 (empty bucket)
- ✅ **Deterministic preferred slot** (pure compute, no memory)
- ✅ **Tag comparison before key comparison** (cheap rejection)
- ✅ **Cache-line locality** (mask + tags adjacent)

**Code Pattern:**
```rust
#[inline(always)]
pub fn get_with_hash<'g>(&'g self, key: &K, hash: u64) -> Option<&'g V> {
    let bidx = self.bucket_index(hash);           // Pure compute
    let tag = self.tag8(hash);                    // Pure compute
    let preferred = self.preferred_slot(hash);    // Pure compute
    
    // ⚡ HOT: Single atomic load
    let mask = self.bucket_meta[bidx].mask.load(Ordering::Acquire);
    if mask == 0 { return None; }  // ⚡ Fast path
    
    // ⚡ HOT: Deterministic probe
    for offset in 0..self.bucket_slots {
        let slot = (preferred + offset) & (self.bucket_slots - 1);
        
        if (mask & (1u64 << slot)) == 0 { continue; }      // ⚡ Bit check
        if self.bucket_meta[bidx].tags[slot] != tag { continue; }  // ⚡ Tag check
        
        let rec = &self.buckets[bidx].recs[slot];
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

**Critical Measurements:**
- Count atomics per call (target: 1)
- Measure cycles for empty bucket (target: < 10)
- Measure cycles for found key (target: < 50)
- Measure cache misses (target: < 1 after warmup)

---

### 2. WRITE (New Key): `upsert()` - NEW KEY INSERT

**Why Critical:** Write throughput bottleneck, previously had mutex contention

**Target:** < 30 cycles (empty slot), < 80 cycles (partial bucket)

**Optimizations:**
- ✅ **No mutex acquisition** (was major bottleneck)
- ✅ **Write before publish** (establishes happens-before)
- ✅ **Single atomic publication** with `fetch_or` and `Release`
- ✅ **Deterministic slot search** from preferred position

**Code Pattern:**
```rust
#[inline]
pub fn upsert(&self, key: &K, value: &V) {
    let hash = hash64(key);
    let bidx = self.bucket_index(hash);
    let tag = self.tag8(hash);
    let preferred = self.preferred_slot(hash);
    
    // ⚡ HOT: Find empty slot (no atomics needed)
    let mask = self.bucket_meta[bidx].mask.load(Ordering::Acquire);
    let mut slot = None;
    for offset in 0..self.bucket_slots {
        let s = (preferred + offset) & (self.bucket_slots - 1);
        if (mask & (1u64 << s)) == 0 {
            slot = Some(s);
            break;
        }
    }
    
    let slot = slot.expect("Bucket full");
    
    // ⚡ HOT: Write before publish (non-atomic writes safe)
    self.buckets[bidx].recs[slot] = Rec { 
        kind: 0, _pad: [0; 7], 
        key: key.clone(), 
        value: *value 
    };
    self.bucket_meta[bidx].tags[slot] = tag;
    
    // ⚡ HOT: Single atomic to publish
    self.bucket_meta[bidx].mask.fetch_or(1u64 << slot, Ordering::Release);
}
```

**Critical Measurements:**
- Count atomics per call (target: 1)
- Measure cycles for insert (target: < 30 empty, < 80 partial)
- Verify no mutex contention (target: 0)
- Measure write throughput under concurrency

---

### 3. WRITE (Update): `upsert()` - EXISTING KEY UPDATE

**Why Critical:** Common operation in delta index, must be atomic for correctness

**Target:** < 100 cycles (no contention), < 150 cycles (with CAS retry)

**Optimizations:**
- ✅ **Lock-free CAS-based update** (no mutex)
- ✅ **Single 64-bit CAS** for atomic old→new swap
- ✅ **Bounded retry loop** (≤ bucket_slots iterations)
- ✅ **Write new record before CAS** (ensures consistency)

**Code Pattern:**
```rust
#[inline]
fn upsert_update(&self, bidx: usize, old_slot: usize, key: &K, value: &V, hash: u64) {
    let tag = self.tag8(hash);
    let preferred = self.preferred_slot(hash);
    
    // ⚡ HOT: Find new empty slot
    let new_slot = self.find_empty_slot(bidx, preferred).expect("Bucket full");
    
    // ⚡ HOT: Write new record before CAS
    self.buckets[bidx].recs[new_slot] = Rec { 
        kind: 0, _pad: [0; 7], 
        key: key.clone(), 
        value: *value 
    };
    self.bucket_meta[bidx].tags[new_slot] = tag;
    
    // ⚡ HOT: Atomic old→new swap with CAS
    let meta = &self.bucket_meta[bidx];
    loop {
        let old_mask = meta.mask.load(Ordering::Acquire);
        let new_mask = (old_mask | (1u64 << new_slot)) & !(1u64 << old_slot);
        
        match meta.mask.compare_exchange(
            old_mask, new_mask,
            Ordering::AcqRel, Ordering::Acquire
        ) {
            Ok(_) => break,
            Err(_) => continue,  // ⚡ Bounded retry
        }
    }
}
```

**Critical Measurements:**
- Measure cycles for update (target: < 100)
- Count CAS retries (target: < 3 average)
- Test under high contention (10+ concurrent writers)
- Verify no torn reads by concurrent readers

---

### 4. ITERATION: `iter()` - FULL INDEX SCAN

**Why Critical:** Used for consolidation, queries, stats - affects throughput

**Target:** < 10ns per entry, < 1ns per empty bucket

**Optimizations:**
- ✅ **Hoist all masks upfront** (sequential atomic loads)
- ✅ **Zero atomics during yield** (iterate local copy)
- ✅ **Early skip for mask == 0** (avoid touching bucket data)
- ✅ **Cache-friendly sequential access** of metadata array

**Code Pattern:**
```rust
pub fn iter<'g>(&'g self) -> impl Iterator<Item = &'g V> + 'g {
    // ⚡ HOT: Hoist all masks upfront (sequential loads)
    let masks: Vec<u64> = (0..self.buckets.len())
        .map(|i| self.bucket_meta[i].mask.load(Ordering::Acquire))
        .collect();
    
    struct State<'a, K, V> {
        idx: &'a RadixIndex<K, V>,
        masks: Vec<u64>,
        bucket_pos: usize,
        slot_pos: usize,
    }
    
    let mut st = State { idx: self, masks, bucket_pos: 0, slot_pos: 0 };
    
    std::iter::from_fn(move || {
        loop {
            if st.bucket_pos >= st.idx.buckets.len() { return None; }
            
            // ⚡ HOT: Check local mask copy (no atomics!)
            let mask = st.masks[st.bucket_pos];
            
            // ⚡ HOT: Skip empty buckets
            if mask == 0 {
                st.bucket_pos += 1;
                st.slot_pos = 0;
                continue;
            }
            
            // ⚡ HOT: Scan live slots
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
            
            st.bucket_pos += 1;
            st.slot_pos = 0;
        }
    })
}
```

**Critical Measurements:**
- Count atomics during yield phase (target: 0)
- Measure ns per entry (target: < 10ns)
- Measure empty bucket skip cost (target: < 1ns)
- Compare throughput vs current implementation

---

### 5. DELETE: `delete()` - KEY REMOVAL

**Why Critical:** Common in delta updates, affects consistency

**Target:** < 80 cycles

**Optimizations:**
- ✅ **No mutex acquisition** (was contention point)
- ✅ **Single atomic unpublish** with `fetch_and`
- ✅ **Optional tombstone marking** for debugging

**Code Pattern:**
```rust
#[inline]
pub fn delete(&self, key: &K) {
    let hash = hash64(key);
    let bidx = self.bucket_index(hash);
    let preferred = self.preferred_slot(hash);
    
    // ⚡ HOT: Find slot (same as read path)
    let slot = self.find_slot_by_key(bidx, key, hash, preferred);
    let slot = match slot {
        Some(s) => s,
        None => return,
    };
    
    // Optional: mark tombstone for debugging
    self.buckets[bidx].recs[slot].kind = 1;
    
    // ⚡ HOT: Single atomic to unpublish
    self.bucket_meta[bidx].mask.fetch_and(!(1u64 << slot), Ordering::Release);
}
```

**Critical Measurements:**
- Count atomics per call (target: 1)
- Measure cycles (target: < 80)
- Verify eventual visibility to readers

---

## 🎯 Performance Targets Summary

| Operation | Current | Target | Critical? |
|-----------|---------|--------|-----------|
| **`get()` - empty** | ~20 cycles | **< 10 cycles** | ⚡ CRITICAL |
| **`get()` - found** | ~100 cycles | **< 50 cycles** | ⚡ CRITICAL |
| **`upsert()` - new** | ~200 cycles | **< 30 cycles** | ⚡ CRITICAL |
| **`upsert()` - update** | ~250 cycles | **< 100 cycles** | ⚡ CRITICAL |
| **`iter()` - per entry** | ~30ns | **< 10ns** | ⚡ CRITICAL |
| `delete()` | ~220 cycles | < 80 cycles | Important |

---

## 🔍 Hot-Path Helpers (Secondary)

### Hash Bit Extraction - Pure Compute, Zero Atomics

**Why Important:** Called on every operation, must be inlined

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
    let slot_bits = (self.bucket_slots - 1).count_ones();
    let shift = self.bucket_bits + 8;
    ((hash >> shift) as usize) & (self.bucket_slots - 1)
}
```

**Target:** < 5 cycles total for all three

---

### Bit-Scan Helpers - Single Atomic Per Call

```rust
// Find slot containing key
#[inline]
fn find_slot_by_key(&self, bidx: usize, key: &K, hash: u64, preferred: usize) -> Option<usize> {
    let mask = self.bucket_meta[bidx].mask.load(Ordering::Acquire);  // ⚡ 1 atomic
    let tag = self.tag8(hash);
    let tags = &self.bucket_meta[bidx].tags;
    let bucket = &self.buckets[bidx];
    
    for offset in 0..self.bucket_slots {
        let slot = (preferred + offset) & (self.bucket_slots - 1);
        if (mask & (1u64 << slot)) == 0 { continue; }      // ⚡ Bit check
        if tags[slot] != tag { continue; }                  // ⚡ Tag check
        if &bucket.recs[slot].key == key { return Some(slot); }
    }
    None
}

// Find empty slot
#[inline]
fn find_empty_slot(&self, bidx: usize, preferred: usize) -> Option<usize> {
    let mask = self.bucket_meta[bidx].mask.load(Ordering::Acquire);  // ⚡ 1 atomic
    
    for offset in 0..self.bucket_slots {
        let slot = (preferred + offset) & (self.bucket_slots - 1);
        if (mask & (1u64 << slot)) == 0 { return Some(slot); }
    }
    None
}
```

---

## 🚦 Memory Ordering Rules

### Three Publication Patterns

#### 1. **Single-Slot Changes** (Insert, Delete)
```rust
// Write data first (non-atomic)
bucket.recs[slot] = ...;
bucket_meta.tags[slot] = ...;

// Publish with Release (synchronizes with readers' Acquire)
mask.fetch_or(1u64 << slot, Ordering::Release);   // Insert
// or
mask.fetch_and(!(1u64 << slot), Ordering::Release);  // Delete
```

#### 2. **Multi-Slot Changes** (Update)
```rust
// Write new data first (non-atomic)
bucket.recs[new_slot] = ...;
bucket_meta.tags[new_slot] = ...;

// Atomic swap with CAS (single operation)
let old_mask = mask.load(Ordering::Acquire);
let new_mask = (old_mask | (1u64 << new_slot)) & !(1u64 << old_slot);
mask.compare_exchange(old_mask, new_mask, Ordering::AcqRel, Ordering::Acquire);
```

#### 3. **Iteration Hoisting** (Read Snapshot)
```rust
// Load all masks upfront with Acquire
let masks: Vec<u64> = buckets.iter()
    .map(|i| meta[i].mask.load(Ordering::Acquire))
    .collect();

// Iterate local copy with no atomics
for (bidx, mask) in masks.iter().enumerate() {
    // Zero atomics here - work on local mask copy
}
```

**Critical:** Readers may see stale data (old mask), but never torn data (partial multi-bit update)

---

## ⚙️ Compiler Hints & Attributes

### Inline Attributes

```rust
// Always inline (critical hot paths)
#[inline(always)]
fn get_with_hash(...) { ... }
#[inline(always)]
fn bucket_index(...) { ... }
#[inline(always)]
fn tag8(...) { ... }
#[inline(always)]
fn preferred_slot(...) { ... }

// Inline hint (secondary hot paths)
#[inline]
fn upsert(...) { ... }
#[inline]
fn delete(...) { ... }
#[inline]
fn find_slot_by_key(...) { ... }
#[inline]
fn find_empty_slot(...) { ... }
```

### Branch Hints (Rust Nightly)

```rust
// If using nightly, can hint likely branches
#[cold]
fn handle_full_bucket_panic() -> ! { ... }

// Empty bucket is rare after warmup
if likely(mask != 0) {
    // hot path
}
```

---

## 📊 Profiling Checklist

### What to Measure

- [ ] **Atomic count per operation** (perf stat -e atomic-ops)
- [ ] **Cycle count per operation** (rdtsc or perf)
- [ ] **Cache miss rate** (perf stat -e cache-misses)
- [ ] **Branch mispredictions** (perf stat -e branch-misses)
- [ ] **Lock contention** (should be zero)
- [ ] **Throughput under concurrency** (ops/sec with N threads)

### How to Profile

```bash
# Count atomics
perf stat -e mem_inst_retired.lock_loads ./benchmark

# Measure cycles
perf stat -e cycles,instructions ./benchmark

# Cache misses
perf stat -e cache-references,cache-misses ./benchmark

# Full profile
perf record -g ./benchmark
perf report
```

---

## 🎯 Success Criteria Per Hot Path

### `get()` Success
- ✅ 1 atomic load per call
- ✅ < 10 cycles for empty bucket
- ✅ < 50 cycles for found key
- ✅ < 1 cache miss per call (after warmup)

### `upsert()` New Key Success
- ✅ 1 atomic per call (no mutex)
- ✅ < 30 cycles for empty slot
- ✅ < 80 cycles for partial bucket
- ✅ Zero lock contention

### `upsert()` Update Success
- ✅ 1 CAS per call (avg < 3 retries)
- ✅ < 100 cycles without contention
- ✅ No torn reads by concurrent readers

### `iter()` Success
- ✅ N atomics for N buckets (hoist phase)
- ✅ 0 atomics during yield phase
- ✅ < 10ns per entry yield
- ✅ < 1ns per empty bucket skip

### `delete()` Success
- ✅ 1 atomic per call
- ✅ < 80 cycles
- ✅ Eventual visibility to readers

---

## 🔬 Debugging Hot-Path Issues

### Issue: High Cycle Count on `get()`
**Check:**
- Is `#[inline(always)]` applied?
- Is compiler optimizing away the loop?
- Are there branch mispredictions?
- Is mask == 0 fast path working?

### Issue: High Atomic Count
**Check:**
- Are helpers being inlined?
- Is mask being reloaded unnecessarily?
- Is compiler optimizing atomic loads?

### Issue: CAS Retry Loop Spinning
**Check:**
- Is there high write contention?
- Is bucket near full?
- Is preferred slot biased?
- Consider adding backoff or increasing bucket_slots

### Issue: Cache Misses
**Check:**
- Is metadata array cache-line aligned?
- Are buckets and metadata adjacent in memory?
- Is prefetching helping/hurting?

---

**Document Purpose:** Quick reference during implementation
**Last Updated:** 2025-11-02
**Use:** Keep open while coding hot-path functions

