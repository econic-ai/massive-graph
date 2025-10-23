# 256MB Allocation Diagnostic Test Plan

## Test Configurations

Edit these constants to systematically isolate the 256MB allocation source:

### In `optimised_index.rs` (line ~34):
```rust
const TEST_MPH_ATOMIC: bool = true;      // epoch::Atomic<MPHIndex>
const TEST_BLOOM_ATOMIC: bool = true;    // epoch::Atomic<DeltaBloom>
const TEST_STREAM: bool = true;          // SegmentedStream
const TEST_RADIX: bool = true;           // RadixIndex
```

### In `radix_index.rs` with_capacity_minimal() (line ~125):
```rust
const USE_BUCKET_ATOMICS: bool = true;   // epoch::Atomic in buckets
```

## Test Sequence

### Test 1: Baseline (all enabled)
- MPH_ATOMIC=true, BLOOM_ATOMIC=true, STREAM=true, RADIX=true, BUCKET_ATOMICS=true
- **Expected**: FAIL at ~522K indexes

### Test 2: No MPH epoch::Atomic
- MPH_ATOMIC=**false**, BLOOM_ATOMIC=true, STREAM=true, RADIX=true, BUCKET_ATOMICS=true
- **Tests**: Is MPHIndex epoch::Atomic the culprit?

### Test 3: No Bloom epoch::Atomic
- MPH_ATOMIC=true, BLOOM_ATOMIC=**false**, STREAM=true, RADIX=true, BUCKET_ATOMICS=true
- **Tests**: Is DeltaBloom epoch::Atomic the culprit?

### Test 4: No epoch::Atomics in OptimisedIndex
- MPH_ATOMIC=**false**, BLOOM_ATOMIC=**false**, STREAM=true, RADIX=true, BUCKET_ATOMICS=true
- **Tests**: Are the 2 epoch::Atomics per OptimisedIndex the issue?

### Test 5: No bucket epoch::Atomics
- MPH_ATOMIC=false, BLOOM_ATOMIC=false, STREAM=true, RADIX=true, BUCKET_ATOMICS=**false**
- **Tests**: Is it the bucket snapshot atomics? (1 per bucket)

### Test 6: Absolutely minimal (no epoch::Atomic anywhere)
- MPH_ATOMIC=false, BLOOM_ATOMIC=false, STREAM=true, RADIX=true, BUCKET_ATOMICS=false
- **Expected**: PASS if epoch::Atomic is the root cause

## Run Command
```bash
cd /Users/jordan/code/econic/massive-graph
cargo bench --bench compare_indexes_insert_bench 2>&1 | grep -E "memory allocation|=== Epoch" | tail -5
```

## Results Log

| Test | MPH | Bloom | Bucket | Result | Notes |
|------|-----|-------|--------|--------|-------|
| 1    | ✓   | ✓     | ✓      | FAIL   | Baseline |
| 2    | ✗   | ✓     | ✓      |        |        |
| 3    | ✓   | ✗     | ✓      |        |        |
| 4    | ✗   | ✗     | ✓      |        |        |
| 5    | ✗   | ✗     | ✗      |        |        |

## Hypothesis
Creating 522K OptimisedIndex instances = 1,044,000 epoch::Atomic instances (2 per index: mph + bloom).
Plus 8 buckets × 522K = 4,176,000 epoch::Atomic<ReadTinyMap> instances.
**Total: ~5.2 million epoch::Atomic instances** → triggers 256MB allocation in epoch's global state.



