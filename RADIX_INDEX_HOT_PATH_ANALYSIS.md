# Radix Index Hot-Path Analysis

## Critical Hot-Path Operations

This document identifies the specific hot-path operations that must be optimized for maximum performance in the new radix index design.

## 1. Read Path (`get_with_hash`) - PRIMARY HOT PATH

### Current Implementation Issues
```rust
// Current: 5-8 atomic loads + TinyMap search + reverse scan
pub fn get_with_hash<'g>(&'g self, key: &K, hash: u64, guard: &'g epoch::Guard) -> Option<&'g V> {
    // 1. Atomic load: bucket.head
    let cur = b.head.load(Ordering::Acquire);
    // 2. Atomic load: snapshot_tail  
    let snapshot_tail = b.snapshot_tail.load(Ordering::Acquire);
    // 3. Atomic load: current_tail
    let current_tail = buf.tail.load(Ordering::Acquire);
    // 4. Reverse scan loop (cache misses)
    for i in (snapshot_tail..current_tail).rev() { ... }
    // 5. Atomic load: snapshot
    let snap = b.snapshot.load(Ordering::Acquire, guard);
    // 6. TinyMap binary search (pointer chasing)
    for slot_idx in map.iter_tags_linear(tag) { ... }
}
```

### New Implementation Target
```rust
// Target: 1 atomic load + bit-scan + tag comparison
pub fn get_with_hash<'g>(&'g self, key: &K, hash: u64) -> Option<&'g V> {
    let bidx = self.bucket_index(hash);
    let preferred = self.preferred_slot(hash);
    let tag = self.tag8(hash);
    
    // SINGLE HOT-PATH ATOMIC LOAD
    let mask = self.bucket_meta[bidx].mask.load(Ordering::Acquire);
    
    // Bit-scan walk from preferred slot (CPU-optimized)
    let mut slot = preferred;
    for _ in 0..self.bucket_slots {
        if (mask >> slot) & 1 == 1 {
            // Early tag comparison (cache-friendly)
            if self.bucket_meta[bidx].tags[slot] == tag {
                // Full key comparison (only if tag matches)
                if &self.buckets[bidx].recs[slot].key == key {
                    return Some(&self.buckets[bidx].recs[slot].value);
                }
            }
        }
        slot = (slot + 1) % self.bucket_slots;
    }
    None
}
```

### Optimization Techniques
1. **Single Atomic Load**: Use `Acquire` ordering once, then work on local copy
2. **CPU Bit-Scan**: Use `u64::trailing_zeros()` for efficient bit scanning
3. **Early Tag Filter**: Compare 8-bit tag before expensive key comparison
4. **Cache-Line Alignment**: Keep mask and tags in same cache line
5. **Branch Prediction**: Predictable probe order (preferred → wrap)

### Performance Target
- **Empty bucket**: < 10 cycles (early return on mask == 0)
- **Found key**: < 50 cycles (1-2 probe steps typical)
- **Not found**: < 100 cycles (full bucket scan worst case)

## 2. Write Path - New Insert (`upsert` - new key) - PRIMARY HOT PATH

### Current Implementation Issues
```rust
// Current: Mutex lock + atomic fetch_add + write
pub fn upsert(&self, key: &K, value: &V, _guard: &epoch::Guard) {
    let b = &self.buckets[bidx];
    // MUTEX ACQUISITION (contention point)
    let _wl = b.write_mx.lock().unwrap();
    // Atomic fetch_add (contention point)
    let (buf, slot_idx) = self.reserve_slot(b);
    // Write
    unsafe { (*rec_ptr).key = key.clone(); ... }
}
```

### New Implementation Target
```rust
// Target: Bit-scan + write + single atomic fetch_or
pub fn upsert(&self, key: &K, value: &V) {
    let h = hash64(key);
    let bidx = self.bucket_index(h);
    let preferred = self.preferred_slot(h);
    let tag = self.tag8(h);
    
    // Find empty slot (lock-free bit-scan)
    let mask = self.bucket_meta[bidx].mask.load(Ordering::Acquire);
    let slot = self.find_empty_slot(mask, preferred)?;
    
    // Write key/value/tag BEFORE publishing (store ordering)
    self.buckets[bidx].recs[slot] = Rec { kind: 0, key: key.clone(), value: *value };
    self.bucket_meta[bidx].tags[slot] = tag;
    
    // SINGLE HOT-PATH ATOMIC PUBLICATION
    self.bucket_meta[bidx].mask.fetch_or(1u64 << slot, Ordering::Release);
}
```

### Optimization Techniques
1. **Lock-Free Slot Finding**: Bit-scan without mutex
2. **Write-Before-Publish**: Store ordering ensures readers see complete record
3. **Single Atomic**: `fetch_or` with Release ordering is sufficient
4. **No Epoch Guard**: Eliminate epoch pin overhead

### Performance Target
- **Empty slot found**: < 30 cycles
- **Slot found after 1-2 probes**: < 50 cycles
- **Full bucket**: Panic (future: rebuild path)

## 3. Write Path - Update Existing (`upsert` - existing key) - SECONDARY HOT PATH

### Current Implementation Issues
```rust
// Current: Mutex lock + reverse scan + append
pub fn upsert(&self, key: &K, value: &V, _guard: &epoch::Guard) {
    // Always appends, never updates in-place
    // Consolidation required to deduplicate
}
```

### New Implementation Target
```rust
// Target: Locate old + find new + CAS swap
pub fn upsert(&self, key: &K, value: &V) {
    let h = hash64(key);
    let bidx = self.bucket_index(h);
    let preferred = self.preferred_slot(h);
    let tag = self.tag8(h);
    
    // Locate old slot (same as read path)
    let mask = self.bucket_meta[bidx].mask.load(Ordering::Acquire);
    let old_slot = self.find_slot_by_key(mask, preferred, tag, key)?;
    
    // Reserve new slot
    let new_slot = self.find_empty_slot(mask, preferred)?;
    
    // Write new record
    self.buckets[bidx].recs[new_slot] = Rec { kind: 0, key: key.clone(), value: *value };
    self.bucket_meta[bidx].tags[new_slot] = tag;
    
    // SINGLE HOT-PATH CAS FOR ATOMIC SWAP
    loop {
        let old_mask = self.bucket_meta[bidx].mask.load(Ordering::Acquire);
        let new_mask = (old_mask | (1u64 << new_slot)) & !(1u64 << old_slot);
        match self.bucket_meta[bidx].mask.compare_exchange(
            old_mask, new_mask, Ordering::AcqRel, Ordering::Acquire
        ) {
            Ok(_) => break,
            Err(_) => continue, // Retry (bounded by bucket size)
        }
    }
}
```

### Optimization Techniques
1. **Single CAS**: Atomic old→new swap prevents torn reads
2. **Bounded Retry**: Max 64 iterations (bucket size)
3. **No Mutex**: Lock-free via CAS

### Performance Target
- **Update with CAS success**: < 100 cycles
- **Update with 1 retry**: < 150 cycles
- **Update worst case**: < 500 cycles (multiple retries)

## 4. Iteration Path (`iter` / `iter_with_keys`) - SECONDARY HOT PATH

### Current Implementation Issues
```rust
// Current: Epoch load per bucket + TinyMap iteration + multiple atomics
pub fn iter<'g>(&'g self, guard: &'g epoch::Guard) -> impl Iterator<Item = &'g V> + 'g {
    // Epoch load per bucket iteration
    let active_ptr = self.active.load(Ordering::Acquire, guard);
    // ...
    for bidx in active.iter() {
        // Multiple atomics per bucket
        let map_ptr = b.snapshot.load(Ordering::Acquire, guard);
        let cur = b.head.load(Ordering::Acquire);
        // TinyMap iteration (pointer chasing)
        for slot_idx in map.slots() { ... }
    }
}
```

### New Implementation Target
```rust
// Target: Hoist all masks + iterate local copy
pub fn iter<'g>(&'g self) -> impl Iterator<Item = &'g V> + 'g {
    // HOIST ALL MASKS AT START (single pass, cache-friendly)
    let masks: Vec<u64> = (0..self.buckets.len())
        .map(|i| self.bucket_meta[i].mask.load(Ordering::Acquire))
        .collect();
    
    // Iterate over local copy - ZERO ATOMICS DURING YIELD
    self.buckets.iter().enumerate().flat_map(move |(bidx, bucket)| {
        let mask = masks[bidx];
        if mask == 0 { return None; } // Early skip
        
        // Bit-scan walk over mask (CPU-optimized)
        (0..self.bucket_slots)
            .filter(|slot| (mask >> slot) & 1 == 1)
            .filter_map(|slot| {
                if bucket.recs[slot].kind == 0 { // Skip tombstones
                    Some(&bucket.recs[slot].value)
                } else {
                    None
                }
            })
    })
}
```

### Optimization Techniques
1. **Mask Hoisting**: Single atomic load per bucket upfront
2. **Zero Atomics During Yield**: Work on local copy
3. **Early Skip**: Skip buckets with mask == 0
4. **Window-Based Hoisting**: Process 64 buckets at a time to limit cache footprint

### Performance Target
- **Per-bucket overhead**: < 1ns
- **Per-entry yield**: < 10ns
- **Cache misses**: < 1 per 100 entries (after warmup)

## 5. Delete Path (`delete`) - TERTIARY HOT PATH

### Implementation Target
```rust
pub fn delete(&self, key: &K) {
    let h = hash64(key);
    let bidx = self.bucket_index(h);
    let preferred = self.preferred_slot(h);
    let tag = self.tag8(h);
    
    // Locate slot (same as read path)
    let mask = self.bucket_meta[bidx].mask.load(Ordering::Acquire);
    let slot = self.find_slot_by_key(mask, preferred, tag, key)?;
    
    // Optionally mark tombstone
    self.buckets[bidx].recs[slot].kind = 1;
    
    // SINGLE HOT-PATH ATOMIC CLEAR
    self.bucket_meta[bidx].mask.fetch_and(!(1u64 << slot), Ordering::Release);
}
```

### Optimization Techniques
1. **Single Atomic**: `fetch_and` with Release ordering
2. **Reuse Read Path**: Same slot-finding logic

### Performance Target
- **Delete**: < 80 cycles (similar to read + atomic clear)

## Critical Helper Functions (Must Be Inlined)

### `find_empty_slot()` - Called by writes
```rust
#[inline(always)]
fn find_empty_slot(&self, mask: u64, start: usize) -> Option<usize> {
    // Use CPU bit-scan instructions
    let inverted = !mask;
    if inverted == 0 { return None; } // Full bucket
    
    // Find first 0-bit starting from preferred slot
    let mut slot = start;
    for _ in 0..self.bucket_slots {
        if (mask >> slot) & 1 == 0 {
            return Some(slot);
        }
        slot = (slot + 1) % self.bucket_slots;
    }
    None
}
```

### `find_slot_by_key()` - Called by reads/updates/deletes
```rust
#[inline(always)]
fn find_slot_by_key(&self, mask: u64, start: usize, tag: u8, key: &K) -> Option<usize> {
    let mut slot = start;
    for _ in 0..self.bucket_slots {
        if (mask >> slot) & 1 == 1 {
            // Early tag comparison (cache-friendly)
            if self.bucket_meta[bidx].tags[slot] == tag {
                // Full key comparison (only if tag matches)
                if &self.buckets[bidx].recs[slot].key == key {
                    return Some(slot);
                }
            }
        }
        slot = (slot + 1) % self.bucket_slots;
    }
    None
}
```

### `preferred_slot()` - Called by all operations
```rust
#[inline(always)]
fn preferred_slot(&self, hash: u64) -> usize {
    let shift = self.bucket_bits + 8; // Skip bucket + tag bits
    let slot_bits = self.bucket_slots.trailing_zeros();
    ((hash >> shift) & ((1usize << slot_bits) - 1)) as usize
}
```

## CPU Optimization Opportunities

### Bit-Scan Instructions
- Use `u64::trailing_zeros()` / `u64::leading_zeros()` for efficient bit scanning
- Consider `__builtin_ctz` / `__builtin_clz` for platform-specific optimizations
- Use `u64::count_ones()` for quick population checks

### Cache Optimization
- Keep `mask` and first 8 `tags` in same cache line (64 bytes)
- Place `bucket_meta` array adjacent to `buckets` array
- Prefer sequential access patterns over random access

### Branch Prediction
- Predictable probe order (preferred → wrap) helps CPU prefetch
- Early returns (empty bucket, tag mismatch) reduce mispredictions
- Tag comparison before key comparison reduces expensive comparisons

## Measurement Points

### Profiling Targets
1. **Atomic operations**: Count `load`/`store`/`fetch_or`/`fetch_and`/`CAS` calls
2. **Cache misses**: Measure L1/L2/L3 misses per operation
3. **Branch mispredictions**: Count mispredicted branches
4. **Cycles per operation**: Measure using `rdtsc` or `perf`

### Benchmark Scenarios
1. **Read-heavy**: 90% reads, 10% writes
2. **Write-heavy**: 10% reads, 90% writes
3. **Mixed**: 50% reads, 50% writes
4. **Iteration**: Full index scan

## Summary

The new design eliminates:
- ❌ Mutex contention (removed `write_mx`, `grow_mx`)
- ❌ RCU epoch overhead (removed epoch loads per bucket)
- ❌ TinyMap pointer chasing (removed TinyMap snapshots)
- ❌ Multiple atomic loads per read (reduced to 1)

The new design optimizes:
- ✅ Single atomic load per read
- ✅ Lock-free writes via atomic operations
- ✅ Cache-friendly memory layout
- ✅ CPU-optimized bit-scan operations
- ✅ Early tag filtering before expensive comparisons

