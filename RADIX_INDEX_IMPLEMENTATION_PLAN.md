# Radix Index Next-State Implementation Plan

## Overview

This document outlines the implementation plan for transitioning the radix index from its current RCU-based design with TinyMap snapshots to a fixed-capacity, mask-driven design that eliminates mutex contention and improves cache locality.

## Current State Analysis

### Current Architecture
- **RCU-managed buffers**: Append-only tail with atomic pointer swaps
- **TinyMap snapshots**: RCU-managed ReadTinyMap for fast lookups
- **Mutex-protected writes**: `grow_mx` and `write_mx` serialize operations
- **Epoch-based active tracking**: RCU-managed Vec<u16> for active buckets
- **Snapshot tail tracking**: Tracks dirty vs clean state for partial scans

### Performance Bottlenecks Identified
1. **Write contention**: Mutex serialization on `write_mx` for all upserts
2. **Cache misses**: Pointer chasing through RCU-managed TinyMaps
3. **Epoch overhead**: Multiple atomic loads per read (mask, snapshot, tail)
4. **Memory overhead**: Colocated TinyMap allocations increase fragmentation
5. **Iteration complexity**: Multiple epoch loads per bucket during iteration

## Target Architecture

### New Structure
```rust
pub struct RadixIndex<K, V> {
    buckets: Box<[Bucket<K, V>]>,      // Fixed array, allocated once
    bucket_meta: Box<[BucketMeta]>,    // 1:1 metadata array
    bucket_bits: u32,                  // Hash bits for bucket selection
    bucket_slots: usize,               // Fixed slots per bucket (≤ 64)
    arena: Arena,                      // Arena-backed storage
}

struct BucketMeta {
    mask: AtomicU64,                    // 1 bit per slot: live/unused
    tags: Box<[u8]>,                    // fp8 tags, aligned with slots
}

struct Bucket<K, V> {
    recs: Box<[Rec<K, V>]>,            // Fixed-capacity records
}
```

### Key Changes
1. **Fixed-capacity buckets**: No growth, panic on full (future: rebuild path)
2. **Single atomic mask**: 64-bit word per bucket (supports ≤64 slots)
3. **Deterministic slot assignment**: Hash-driven preferred slot + linear probe
4. **No mutexes**: Lock-free operations via atomic mask updates
5. **No RCU/TinyMap**: Direct mask+tag array for visibility

## Hot-Path Operations & Optimization Targets

### 🔥 Critical Hot Paths (Must Optimize)

#### 1. **Read Path (`get` / `get_with_hash`)**
**Current**: ~5-8 atomic loads + TinyMap binary search + reverse scan
**Target**: 1 atomic load + bit-scan walk + 1-2 tag comparisons

**Optimizations**:
- Single `Acquire` load of 64-bit mask
- CPU-optimized bit-scan forward (use `__builtin_ctz` / `u64::trailing_zeros`)
- Early tag comparison before key comparison
- Minimize branch mispredictions with predictable probe order
- Cache-line friendly: mask and tags in adjacent memory

**Performance Target**: < 10 cycles for empty bucket, < 50 cycles for found key

#### 2. **Write Path - New Insert (`upsert` - new key)**
**Current**: Mutex lock + atomic fetch_add + write
**Target**: Bit-scan + write + single atomic fetch_or

**Optimizations**:
- Bit-scan for first 0-bit (use `__builtin_clz` / `u64::leading_zeros` with mask inversion)
- Single atomic `fetch_or` with Release ordering
- Write key/value/tag BEFORE publishing mask bit (store ordering)
- No mutex acquisition needed

**Performance Target**: < 30 cycles for empty slot insertion

#### 3. **Write Path - Update Existing (`upsert` - existing key)**
**Current**: Mutex lock + reverse scan + append
**Target**: Locate old slot + find new slot + CAS mask swap

**Optimizations**:
- Locate old slot using same bit-scan as read path
- Reserve new slot (bit-scan for 0-bit)
- Single 64-bit CAS for atomic old→new swap
- Retry loop bounded by bucket size (≤64 iterations max)

**Performance Target**: < 100 cycles for update (including CAS retry)

#### 4. **Iteration Path (`iter` / `iter_with_keys`)**
**Current**: Epoch loads per bucket + TinyMap iteration + multiple atomics
**Target**: Hoist all masks at start + iterate local copy

**Optimizations**:
- Single pass: load all masks into Vec<u64> (1 atomic per bucket, all adjacent)
- Iterate over local copy - zero atomics during yield
- Window-based hoisting (64 buckets at a time) to keep cache footprint small
- Skip buckets with mask == 0 early

**Performance Target**: < 1ns per bucket overhead, < 10ns per entry yield

### ⚡ Secondary Optimizations

#### 5. **Hash Bit Extraction**
**Requirement**: Three disjoint hash bit windows
- Bucket selection: low `bucket_bits` 
- fp8 tag: bits `[bucket_bits .. bucket_bits+8]`
- Preferred slot: bits `[bucket_bits+8 .. bucket_bits+8+slot_bits]`

**Implementation**:
```rust
#[inline]
fn preferred_slot(hash: u64, bucket_bits: u32, slot_bits: u32) -> usize {
    let shift = bucket_bits + 8;  // Skip bucket + tag bits
    ((hash >> shift) & ((1usize << slot_bits) - 1)) as usize
}
```

#### 6. **Bit-Scan Operations**
**Requirement**: Fast forward/backward bit scanning from preferred slot

**Implementation**:
- Use `u64::trailing_zeros()` / `u64::leading_zeros()` for empty slot search
- Use `u64::count_ones()` for mask population checks
- Manual wrap-around for circular probe (modulo bucket_slots)

#### 7. **Memory Layout Optimization**
**Alignment**:
- `BucketMeta` array: cache-line aligned (64 bytes)
- `tags` array: tight-packed u8, aligned with `recs` indices
- `recs` array: natural alignment for `Rec<K, V>`

**Cache Line Strategy**:
- Place `mask` and first 8 `tags` in same cache line
- Keep `bucket_meta` array adjacent to `buckets` array
- Prefer sequential access patterns

## Implementation Steps

### Phase 1: Core Structure Refactoring

#### Step 1.1: Add New Types
- [ ] Define `BucketMeta` struct with `AtomicU64` mask and `Box<[u8]>` tags
- [ ] Modify `Bucket` to remove RCU pointers, mutexes, snapshot fields
- [ ] Update `RadixIndex` fields: add `bucket_meta`, remove `active`, `snapshot_tail`
- [ ] Update `Rec` struct (keep as-is, already correct)

#### Step 1.2: Update Hash Utilities
- [ ] Add `preferred_slot_from_hash()` function in `util.rs`
- [ ] Ensure disjoint bit windows (bucket + tag + slot)
- [ ] Add validation/assertions for bit window overlap

#### Step 1.3: Constructor Refactoring
- [ ] Update `with_capacity()` to allocate `bucket_meta` array alongside `buckets`
- [ ] Pre-allocate fixed-capacity `recs` arrays for all buckets (or lazy allocate)
- [ ] Remove `ensure_activated()` logic (all buckets pre-allocated)
- [ ] Set `bucket_slots` based on capacity (max 64 slots)

### Phase 2: Read Path Implementation

#### Step 2.1: Implement Bit-Scan Walk
- [ ] Add `find_slot_by_key()` helper: scans mask starting at preferred slot
- [ ] Optimize with `trailing_zeros()` / manual wrap-around
- [ ] Early tag comparison before full key comparison
- [ ] Return `Option<usize>` for slot index

#### Step 2.2: Implement `get_with_hash()`
- [ ] Load mask with `Acquire` ordering
- [ ] Call `find_slot_by_key()` starting at preferred slot
- [ ] Return `Option<&V>` directly from found slot
- [ ] Handle tombstone case (kind=1)

**Hot-Path Checklist**:
- [ ] Single atomic load
- [ ] No mutex acquisition
- [ ] No epoch guard needed
- [ ] Early tag comparison
- [ ] Predictable probe order

### Phase 3: Write Path Implementation

#### Step 3.1: Implement Bit-Scan for Empty Slot
- [ ] Add `find_empty_slot()` helper: scans mask for first 0-bit
- [ ] Start from preferred slot, wrap around
- [ ] Return `Option<usize>` or panic if full

#### Step 3.2: Implement New Key Insert
- [ ] Compute hash → bucket → tag → preferred slot
- [ ] Call `find_empty_slot()` to reserve slot
- [ ] Write `Rec { kind: 0, key, value }` to slot
- [ ] Write fp8 tag to `bucket_meta[b].tags[slot]`
- [ ] Publish with `mask.fetch_or(1u64 << slot, Ordering::Release)`

**Hot-Path Checklist**:
- [ ] No mutex acquisition
- [ ] Write-before-publish ordering
- [ ] Single atomic for publication
- [ ] Handle full bucket (panic initially)

#### Step 3.3: Implement Update Path
- [ ] Locate old slot using `find_slot_by_key()`
- [ ] Reserve new slot using `find_empty_slot()`
- [ ] Write new record to new slot
- [ ] Write tag to new slot
- [ ] CAS mask: `(old_mask | (1 << new_slot)) & !(1 << old_slot)`
- [ ] Retry on CAS failure (bounded loop)

**Hot-Path Checklist**:
- [ ] Single CAS for atomic swap
- [ ] Bounded retry loop (≤64 iterations)
- [ ] No mutex needed

#### Step 3.4: Implement Delete
- [ ] Locate slot using `find_slot_by_key()`
- [ ] Optionally mark `kind = 1` (tombstone)
- [ ] `mask.fetch_and(!(1u64 << slot), Ordering::Release)`

### Phase 4: Iteration Path Implementation

#### Step 4.1: Implement Mask Hoisting
- [ ] Add `hoist_masks()` helper: loads all masks into `Vec<u64>`
- [ ] Single `Acquire` load per bucket (sequential access)
- [ ] Optionally implement windowing (64 buckets at a time)

#### Step 4.2: Implement `iter()`
- [ ] Call `hoist_masks()` at start
- [ ] Iterate over local mask copy
- [ ] For each bucket with non-zero mask: bit-scan and yield values
- [ ] Skip buckets with mask == 0 early

**Optimization Checklist**:
- [ ] Zero atomics during yield loop
- [ ] Cache-friendly sequential access
- [ ] Early skip for empty buckets

#### Step 4.3: Implement `iter_with_keys()`
- [ ] Same as `iter()` but yield `(&K, &V)` pairs
- [ ] Reuse mask hoisting logic

### Phase 5: Cleanup & Removal

#### Step 5.1: Remove Old Fields
- [ ] Remove `active: epoch::Atomic<Vec<u16>>`
- [ ] Remove `snapshot: epoch::Atomic<ReadTinyMap>`
- [ ] Remove `snapshot_tail: AtomicUsize`
- [ ] Remove `grow_mx`, `write_mx` mutexes
- [ ] Remove `registered: AtomicBool`
- [ ] Remove `tinymap_ptr` from `Buffer`

#### Step 5.2: Remove Old Methods
- [ ] Remove `rebuild_snapshot()`
- [ ] Remove `consolidate_bucket()` (or adapt for future rebuild path)
- [ ] Remove `ensure_activated()`
- [ ] Update `reserve_slot()` (or remove if no longer needed)

#### Step 5.3: Update `clear_all()`
- [ ] Reset all masks to 0
- [ ] Optionally clear tags array
- [ ] No epoch defer needed (no RCU structures)

### Phase 6: Testing & Validation

#### Step 6.1: Unit Tests
- [ ] Test single-key insert/read
- [ ] Test update path (same key, different value)
- [ ] Test delete path
- [ ] Test collision handling (same bucket, different keys)
- [ ] Test full bucket panic
- [ ] Test iteration correctness

#### Step 6.2: Performance Tests
- [ ] Benchmark `get()` vs current implementation
- [ ] Benchmark `upsert()` vs current implementation
- [ ] Benchmark `iter()` vs current implementation
- [ ] Measure cache miss rates (perf stat)
- [ ] Measure atomic contention (lock contention profiling)

#### Step 6.3: Integration Tests
- [ ] Test with `OptimisedIndex` integration
- [ ] Test concurrent read/write workloads
- [ ] Test large-scale insertion (10K+ keys)
- [ ] Verify no memory leaks

### Phase 7: Future Enhancements (Out of Scope)

#### Step 7.1: Full Bucket Handling
- [ ] Replace panic with slow-path rebuild
- [ ] Implement bucket doubling strategy
- [ ] Migrate existing records to new bucket

#### Step 7.2: Epoch Removal (if possible)
- [ ] Evaluate if epoch guard still needed for other components
- [ ] Consider removing epoch dependency entirely

## Performance Benchmarks to Track

### Baseline Metrics (Current Implementation)
- `get()`: ~XX cycles (avg over 1000 lookups)
- `upsert()`: ~XX cycles (avg over 1000 inserts)
- `iter()`: ~XX ns per entry
- Cache misses: ~XX per operation

### Target Metrics (New Implementation)
- `get()`: < 50 cycles (empty bucket: < 10 cycles)
- `upsert()`: < 100 cycles (new key: < 30 cycles)
- `iter()`: < 10ns per entry
- Cache misses: < 1 per operation (after warmup)

## Risk Mitigation

### Risk 1: Hash Bit Collision
**Mitigation**: Assert disjoint bit windows, validate with test suite

### Risk 2: Full Bucket Panic
**Mitigation**: Document limitation, add metrics tracking, plan rebuild path

### Risk 3: Concurrent Write Contention
**Mitigation**: Use CAS with bounded retry, measure contention in benchmarks

### Risk 4: Breaking Changes
**Mitigation**: Maintain API compatibility, feature flag for gradual rollout

## Code Quality Checklist

- [ ] All functions have single-line documentation
- [ ] All warnings fixed (zero warnings policy)
- [ ] Hot-path functions marked `#[inline]`
- [ ] Atomic orderings explicitly documented
- [ ] Unsafe blocks have safety comments
- [ ] No dead code remaining

## Dependencies

### New Dependencies
- None (use std library atomics and bit manipulation)

### Removed Dependencies
- Can potentially remove `crossbeam_epoch` if only used here (check other usages)

## Timeline Estimate

- **Phase 1**: 2-3 days (structure refactoring)
- **Phase 2**: 1-2 days (read path)
- **Phase 3**: 2-3 days (write path)
- **Phase 4**: 1-2 days (iteration)
- **Phase 5**: 1 day (cleanup)
- **Phase 6**: 2-3 days (testing)
- **Total**: ~10-15 days

## Notes

- Start with single-threaded correctness, then validate concurrent behavior
- Profile each phase before moving to next
- Keep old implementation as reference until fully validated
- Consider feature flag for A/B testing

