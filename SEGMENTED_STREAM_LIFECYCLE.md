# SegmentedStream Lifecycle Analysis

## Construction (`new()`)

```rust
pub fn new() -> Self {
    let first: Arc<Page<T>> = Arc::from(Page::<T>::new(page_size, 0));  // Allocates ~1KB Page
    let first_ptr = Arc::into_raw(first.clone());                        // Arc refcount = 2
    let mut pages_vec = Vec::new();
    pages_vec.push(first);                                               // Vec holds 1 Arc
    
    SegmentedStream {
        pages: Mutex::new(pages_vec),                                     // Vec<Arc<Page>>
        active_page: Atomic::from(Owned::from(unsafe {                   // epoch::Atomic<Page>
            Box::from_raw(first_ptr as *mut Page<T>)
        })),
        ...
    }
}
```

### Memory Allocation per Instance:
- **Page struct**: ~1KB (64 entries * 16 bytes for V16)
  - AtomicU64, AtomicU32 x2, AtomicPtr: ~20 bytes
  - Box<[MaybeUninit<V16>]>: 64 * 16 = 1024 bytes
- **Arc metadata**: ~16 bytes
- **Vec metadata**: ~24 bytes
- **Mutex**: ~8 bytes
- **epoch::Atomic**: **8 bytes + GLOBAL EPOCH METADATA**
- **Total visible**: ~1.1KB per instance

### For 522,000 instances:
- Expected memory: 522K * 1.1KB = **~574MB**
- Actual failure: **256MB allocation** (single allocation!)

## Usage in Benchmark

Current bench with upsert disabled:
```rust
b.iter_batched(
    || OptimisedIndex::new_with_indexer_and_capacity(...),  // Creates SegmentedStream
    |idx| {
        for i in 0..n {
            black_box(idx.upsert(...));  // ← DISABLED, doesn't touch stream
        }
    },
    BatchSize::PerIteration  // ← Creates 522K instances during warmup!
}
```

**Key observation**: Stream is created but NEVER USED (upsert is empty), yet benchmark fails!

## Disposal (Drop)

**NO explicit Drop implementation!**

When SegmentedStream drops:
1. `Mutex<Vec<Arc<Page<T>>>>` drops → Vec drops → Arc refcount decrements
2. `active_page: Atomic<Page<T>>` drops → **What happens here?**

### Critical Question:
Does `epoch::Atomic<T>::drop()` automatically defer the `T` to epoch's garbage collection?

If yes, then:
- 522K SegmentedStream drops = 522K Page deferrals
- Epoch queues all 522K * 1KB pages internally
- This could cause epoch's internal structures to grow to 256MB+

## Memory Layout Issue

The construction does something suspicious:
```rust
let first: Arc<Page<T>> = Arc::from(Page::<T>::new(...));
let first_ptr = Arc::into_raw(first.clone());  // Convert Arc to raw pointer
pages_vec.push(first);                          // Vec holds Arc (refcount >= 1)

active_page: Atomic::from(Owned::from(unsafe {
    Box::from_raw(first_ptr as *mut Page<T>)   // Wrap raw pointer in Box, then Owned, then Atomic
})),
```

**This looks like a memory leak pattern!**
- An Arc is converted to a raw pointer
- That raw pointer is wrapped in a Box (taking ownership)
- But the Vec ALSO holds an Arc to the same Page
- Double ownership? Or is epoch managing the lifecycle?

## Hypothesis

**Root Cause**: When `Atomic<Page<T>>` drops, crossbeam-epoch defers the Page drop to its GC. With 522K rapid creations/drops during Criterion's warmup phase:

1. Epoch's deferred queue accumulates 522K Page objects (~1KB each)
2. At some point, epoch tries to allocate internal bookkeeping for this queue
3. The 256MB allocation is epoch's internal structure, not the Pages themselves

## Test Confirmation

✅ MockSegmentedStream (no epoch::Atomic) = PASSES
❌ Real SegmentedStream (with epoch::Atomic) = FAILS

This confirms the `active_page: Atomic<Page<T>>` field is the problem.

## Next Steps

1. Add explicit `Drop` for SegmentedStream that properly handles the Page
2. Or replace `Atomic<Page<T>>` with `ArcSwap<Page<T>>` to avoid epoch altogether
3. Investigate the Arc/Box double-ownership pattern in construction

