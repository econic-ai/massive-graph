use std::{
mem::MaybeUninit,
ptr,
sync::atomic::{AtomicPtr, AtomicU32, AtomicU64, Ordering},
sync::Arc,
};

use crossbeam_epoch::{self as epoch, Atomic, Owned, Shared};
use std::sync::atomic::AtomicUsize;
use std::sync::Mutex;
use std::thread::JoinHandle;


use super::*;
use std::time::Duration;

// NOTE: For basic tests we avoid forcing page rollover because ENTRIES_PER_PAGE
// maps to PAGE_SIZE. Advanced rollover tests are provided as ignored skeletons below.

#[test]
fn stream_new_initial_state() {
    // Goal: Ensure a new stream has a single empty page with next == null
    let s: SegmentedStream<u32> = SegmentedStream::new();
    let guard = epoch::pin();
    let head_shared = s.active_page.load(Ordering::Acquire, &guard);
    let head = unsafe { head_shared.as_ref().unwrap() };
    assert_eq!(head.claimed.load(Ordering::Relaxed), 0);
    assert_eq!(head.committed.load(Ordering::Relaxed), 0);
    assert!(head.next.load(Ordering::Acquire).is_null());
}

#[test]
fn single_writer_append_and_read_order() {
    // Goal: Appending a few items makes them visible to a cursor in order
    let s: SegmentedStream<u32> = SegmentedStream::new();
    assert!(s.append(10).is_ok());
    assert!(s.append(20).is_ok());
    assert!(s.append(30).is_ok());

    let mut c = Cursor::new_at_head(&s);
    // Call next() sequentially and copy values to avoid holding borrows across calls
    let mut got = Vec::new();
    if let Some(v) = c.next() { got.push(*v); }
    if let Some(v) = c.next() { got.push(*v); }
    if let Some(v) = c.next() { got.push(*v); }
    assert_eq!(got, vec![10, 20, 30]);

    // Tail: no more items yet
    assert!(c.next().is_none());
}

#[test]
fn next_batch_basic() {
    // Goal: next_batch returns the committed suffix from current index
    let s: SegmentedStream<u32> = SegmentedStream::new();
    for v in [1_u32, 2, 3] {
        s.append(v).unwrap();
    }
    let mut c = Cursor::new_at_head(&s);
    let batch = c.next_batch();
    assert_eq!(batch, &[1, 2, 3]);
    // Subsequent call at tail yields empty slice
    let batch2 = c.next_batch();
    assert!(batch2.is_empty());
}

// ---------- Skeletons for advanced tests (ignored for now) ----------

#[test]
fn page_boundary_and_linking_single_link_guarantee() {
    // Goal: Force rollover with small test page size, verify linking and counters
    let page_size = 64;
    let s: SegmentedStream<u32> = SegmentedStream::with_page_size(page_size);
    // Fill the first page exactly
    for i in 0..(page_size as u32) {
        s.append(i).unwrap();
    }
    // First page should be full
    let pages = s.pages.lock().unwrap();
    let head = &pages[0];
    assert_eq!(head.committed.load(Ordering::Acquire), page_size as u32);
    
    // Next page should be pre-linked at LINK_AHEAD
    let next_ptr = head.next.load(Ordering::Acquire);
    assert!(!next_ptr.is_null(), "next page should be pre-linked");
    drop(pages);
    
    // Append more items to the second page
    s.append(100).unwrap();
    s.append(101).unwrap();

    let next_page = unsafe { &*next_ptr };
    // The new page should have at least the two appended items committed
    assert!(next_page.committed.load(Ordering::Acquire) >= 2);
    
    // Verify we have at least 2 pages in the pages Vec
    let pages = s.pages.lock().unwrap();
    assert!(pages.len() >= 2, "Should have at least 2 pages after rollover");
}

#[test]
fn multi_writer_correctness_no_gaps_no_dups() {
    // Goal: Spawn multiple writers, append concurrently, ensure total count and no gaps per page
    let writers = 8usize;
    let per = 200usize;
    let total = writers * per;

    let s: Arc<SegmentedStream<u64>> = Arc::new(SegmentedStream::with_page_size(64));

    let mut handles = Vec::new();
    for w in 0..writers {
        let s_cloned = Arc::clone(&s);
        handles.push(std::thread::spawn(move || {
            for i in 0..(per as u64) {
                // Encode writer and sequence to enable exact multiset check
                let v: u64 = ((w as u64) << 32) | i;
                s_cloned.append(v).unwrap();
            }
        }));
    }
    for h in handles { let _ = h.join(); }

    // Read all items back
    let mut c = Cursor::new_at_head(&s);
    let mut items: Vec<u64> = Vec::with_capacity(total);
    while let Some(v) = c.next() { items.push(*v); }
    assert_eq!(items.len(), total);

    // Validate multiset equality
    let mut expected: Vec<u64> = Vec::with_capacity(total);
    for w in 0..writers { for i in 0..(per as u64) { expected.push(((w as u64) << 32) | i); } }
    items.sort_unstable();
    expected.sort_unstable();
    assert_eq!(items, expected);

    // Per-page invariants: claimed >= committed; full pages have committed == page_size
    let guard = epoch::pin();
    let head_shared = s.active_page.load(Ordering::Acquire, &guard);
    let head = unsafe { head_shared.as_ref().unwrap() };
    let mut page_ref: &Page<u64> = &*head;
    loop {
        let claimed = page_ref.claimed.load(Ordering::Relaxed);
        let committed = page_ref.committed.load(Ordering::Acquire);
        let page_size = page_ref.size();
        assert!(claimed >= committed);
        if committed < page_size as u32 {
            // Tail page may be partial; must be the last page
            assert!(page_ref.next.load(Ordering::Acquire).is_null());
            break;
        }
        let next_ptr = page_ref.next.load(Ordering::Acquire);
        if next_ptr.is_null() { break; }
        // SAFETY: pages are backed by leaked Arc pointers; deref is valid
        page_ref = unsafe { &*next_ptr };
    }
}

#[test]
fn tail_behavior_and_resume_after_more_appends() {
    // Goal: Cursor at tail yields None, then after more appends returns items
    let s: SegmentedStream<u32> = SegmentedStream::new();
    s.append(1).unwrap();
    s.append(2).unwrap();
    let mut c = Cursor::new_at_head(&s);
    assert_eq!(c.next().copied(), Some(1));
    assert_eq!(c.next().copied(), Some(2));
    // At tail now
    assert!(c.next().is_none());
    // Append more and ensure the cursor can continue
    s.append(3).unwrap();
    s.append(4).unwrap();
    assert_eq!(c.next().copied(), Some(3));
    assert_eq!(c.next().copied(), Some(4));
}

#[test]
fn batch_read_hops_to_next_page_on_full() {
    // Goal: next_batch returns full first page, then empty while hopping, then next page slice
    let page_size = 64;
    let s: SegmentedStream<u32> = SegmentedStream::with_page_size(page_size);
    let total = page_size + 3;
    for i in 0..(total as u32) {
        s.append(i).unwrap();
    }
    let mut c = Cursor::new_at_head(&s);
    let b1 = c.next_batch();
    assert_eq!(b1.len(), page_size);
    // Next call should perform hop and return empty slice
    let b2 = c.next_batch();
    assert!(b2.is_empty());
    // Third call should expose remaining items from the next page
    let b3 = c.next_batch();
    assert_eq!(b3.len(), 3);
}

#[test]
fn pool_prefiller_fills_ring() {
    // Prefiller should fill up to capacity and allow pops
    let pool = StreamPagePool::<u32>::with_capacity(0).with_prefiller(8);
    let ready = pool.ready.as_ref().unwrap().clone();
    // wait until some pages are available
    let mut spins = 0;
    while ready.len() < 4 && spins < 100 {
        std::thread::sleep(Duration::from_millis(10));
        spins += 1;
    }
    assert!(ready.len() >= 1, "prefiller did not produce pages");
    // Pop a few and ensure they are valid
    for _ in 0..ready.len().min(3) {
        if let Some(ptr) = ready.pop_ptr() {
            // reconstruct Arc and let it drop
            let _arc = unsafe { Arc::from_raw(ptr) };
        }
    }
}

#[test]
fn stream_uses_ready_ring_without_prefiller() {
    // Manually prefill ready ring and verify SegmentedStream consumes it during pre-linking
    let mut pool = StreamPagePool::<u32>::with_capacity(0);
    let ready = Arc::new(Ring::new(4));
    // prefill three pages
    let _ = ready.push_arc(Arc::from(Page::<u32>::new(64, 0)));
    let _ = ready.push_arc(Arc::from(Page::<u32>::new(64, 0)));
    let _ = ready.push_arc(Arc::from(Page::<u32>::new(64, 0)));
    // install ready ring into pool
    pool.ready = Some(ready.clone());

    let page_size = 64;
    let link_ahead = (page_size / 2) as u32;
    let s = SegmentedStream::with_pool_and_page_size(pool, page_size);
    let initial_count = ready.len();
    
    // Fill to LINK_AHEAD to trigger pre-linking (which consumes from pool)
    for i in 0..=link_ahead { 
        s.append(i).unwrap(); 
    }
    
    // Give a tiny moment for the allocation to reflect
    std::thread::sleep(Duration::from_millis(1));
    let after_link_ahead = ready.len();
    
    // Pool should have been consumed during pre-linking
    assert!(after_link_ahead < initial_count, 
        "expected ready ring to be consumed during pre-linking (initial={}, after={})", 
        initial_count, after_link_ahead);
}

#[test]
#[ignore]
fn active_page_update_only_by_cas_winner() {
    // Goal: Ensure only the thread that links next updates active_page
    unimplemented!("observe active_page pointer evolution under contention");
}

#[test]
#[ignore]
fn pool_reuse_resets_page_fields() {
    // Goal: When pool is enabled, recycled pages reset claimed/committed/next
    unimplemented!("enable pool, recycle pages, then reuse and assert resets");
}

// ========================================================================
// Tests for append_with_index and StreamIndex resolution across pages
// ========================================================================

#[test]
fn append_with_index_single_page_all_resolve() {
    // Goal: Verify append_with_index returns valid StreamIndex for items within a single page
    let s: SegmentedStream<u64> = SegmentedStream::with_page_size(64);
    let count = 20; // Well below page size (64)
    
    let mut indices = Vec::new();
    for i in 0..count {
        let sidx = s.append_with_index(i).expect("append should succeed");
        indices.push(sidx);
    }
    
    // Verify all indices resolve to correct values
    for (i, sidx) in indices.iter().enumerate() {
        let resolved = s.resolve_ref_unchecked(sidx);
        assert_eq!(*resolved, i as u64, 
            "StreamIndex[{}] should resolve to {}", i, i);
    }
}

#[test]
fn append_with_index_across_link_ahead_boundary() {
    // Goal: Verify StreamIndex remains valid when crossing LINK_AHEAD (32)
    // This is where pre-linking happens but we're still writing to the current page
    let s: SegmentedStream<u64> = SegmentedStream::with_page_size(64);
    let count = 40; // Cross LINK_AHEAD=32 (page_size/2), but stay within first page (64)
    
    let mut indices = Vec::new();
    for i in 0..count {
        let sidx = s.append_with_index(i).expect("append should succeed");
        indices.push(sidx);
    }
    
    // Critical: indices 0-31 should still resolve after index 32 triggers pre-linking
    for (i, sidx) in indices.iter().enumerate() {
        let resolved = s.resolve_ref_unchecked(sidx);
        assert_eq!(*resolved, i as u64, 
            "StreamIndex[{}] should resolve to {} even after LINK_AHEAD", i, i);
    }
    
    // Verify page structure: should still be on first page
    let guard = epoch::pin();
    let head_shared = s.active_page.load(Ordering::Acquire, &guard);
    let head = unsafe { head_shared.as_ref().unwrap() };
    assert_eq!(head.committed.load(Ordering::Acquire), count as u32);
    
    // Next page should be pre-linked but not used yet
    let next_ptr = head.next.load(Ordering::Acquire);
    assert!(!next_ptr.is_null(), "Next page should be pre-linked at LINK_AHEAD");
    let next_page = unsafe { &*next_ptr };
    assert_eq!(next_page.committed.load(Ordering::Acquire), 0, 
        "Next page should be linked but empty");
}

#[test]
fn append_with_index_across_page_boundary() {
    // Goal: Verify StreamIndex remains valid when crossing page boundary (64)
    let s: SegmentedStream<u64> = SegmentedStream::with_page_size(64);
    let count = 100usize; // Cross page boundary at 64, spanning two pages
    
    let mut indices = Vec::new();
    for i in 0..count {
        let sidx = s.append_with_index(i as u64).expect("append should succeed");
        indices.push(sidx);
    }
    
    // Critical: ALL indices should resolve, including those from the first page
    for (i, sidx) in indices.iter().enumerate() {
        let resolved = s.resolve_ref_unchecked(sidx);
        assert_eq!(*resolved, i as u64, 
            "StreamIndex[{}] should resolve to {} across page boundary", i, i);
    }
    
    // Verify page structure
    let guard = epoch::pin();
    let head_shared = s.active_page.load(Ordering::Acquire, &guard);
    let head = unsafe { head_shared.as_ref().unwrap() };
    let next_ptr = head.next.load(Ordering::Acquire);
    assert!(!next_ptr.is_null(), "First page should link to second page");
    
    // First page should be full
    let page_size = head.size();
    assert_eq!(head.committed.load(Ordering::Acquire), page_size as u32,
        "First page should be completely filled");
    
    // Second page should have remaining items
    let second_page = unsafe { &*next_ptr };
    let expected_in_second = count - page_size;
    assert_eq!(second_page.committed.load(Ordering::Acquire), expected_in_second as u32,
        "Second page should have {} items", expected_in_second);
}

#[test]
fn append_with_index_random_access_after_multiple_pages() {
    // Goal: Verify random access to old pages via StreamIndex after multiple page transitions
    let s: SegmentedStream<u64> = SegmentedStream::with_page_size(64);
    let count = 200usize; // Span 3+ pages (64 * 3 = 192)
    
    let mut indices = Vec::new();
    for i in 0..count {
        let sidx = s.append_with_index(i as u64).expect("append should succeed");
        indices.push(sidx);
    }
    
    // Test random access pattern: backwards, forwards, random
    
    // 1. Backwards traversal
    for i in (0..count).rev() {
        let resolved = s.resolve_ref_unchecked(&indices[i]);
        assert_eq!(*resolved, i as u64, 
            "Backwards: StreamIndex[{}] should resolve to {}", i, i);
    }
    
    // 2. Random access to first page after being on third page
    let first_page_indices = [0usize, 10, 31, 50, 63];
    for &i in &first_page_indices {
        let resolved = s.resolve_ref_unchecked(&indices[i]);
        assert_eq!(*resolved, i as u64, 
            "Random access: StreamIndex[{}] should resolve to {}", i, i);
    }
    
    // 3. Interleaved access across pages
    let interleaved = [5usize, 70, 15, 130, 25, 190, 35];
    for &i in &interleaved {
        let resolved = s.resolve_ref_unchecked(&indices[i]);
        assert_eq!(*resolved, i as u64, 
            "Interleaved: StreamIndex[{}] should resolve to {}", i, i);
    }
}

#[test]
fn append_with_index_forward_cursor_still_works() {
    // Goal: Verify that Cursor (forward iteration) still works alongside StreamIndex
    let s: SegmentedStream<u64> = SegmentedStream::with_page_size(64);
    let count = 100usize;
    
    let mut indices = Vec::new();
    for i in 0..count {
        let sidx = s.append_with_index(i as u64).expect("append should succeed");
        indices.push(sidx);
    }
    
    // Forward iteration via Cursor should work
    let mut cursor = Cursor::new_at_head(&s);
    let mut collected = Vec::new();
    while let Some(v) = cursor.next() {
        collected.push(*v);
    }
    
    assert_eq!(collected.len(), count);
    for (i, &v) in collected.iter().enumerate() {
        assert_eq!(v, i as u64, "Cursor iteration should yield values in order");
    }
    
    // Random access via StreamIndex should also work
    for (i, sidx) in indices.iter().enumerate() {
        let resolved = s.resolve_ref_unchecked(sidx);
        assert_eq!(*resolved, i as u64, 
            "StreamIndex resolution should work alongside Cursor");
    }
}

#[test]
fn append_with_index_page_transition_seamless() {
    // Goal: Verify seamless transition at exact page boundary
    let s: SegmentedStream<u64> = SegmentedStream::with_page_size(64);
    
    // Fill first page exactly
    let page_size = 64;
    let mut indices = Vec::new();
    for i in 0..page_size {
        let sidx = s.append_with_index(i as u64).expect("append should succeed");
        indices.push(sidx);
    }
    
    // Verify first page is full
    let pages = s.pages.lock().unwrap();
    let head = &pages[0];
    assert_eq!(head.committed.load(Ordering::Acquire), head.size() as u32,
        "First page should be exactly full");
    
    // Verify next page was pre-linked
    let next_ptr = head.next.load(Ordering::Acquire);
    assert!(!next_ptr.is_null(), "Next page should be pre-linked");
    drop(pages);
    
    // Add one more item to trigger actual page transition
    let sidx_next = s.append_with_index(page_size as u64).expect("append should succeed");
    indices.push(sidx_next);
    
    // Verify second page has exactly one item
    let second_page = unsafe { &*next_ptr };
    assert_eq!(second_page.committed.load(Ordering::Acquire), 1,
        "Second page should have exactly one item");
    
    // Critical: ALL indices from first page should still resolve
    for (i, sidx) in indices[0..page_size].iter().enumerate() {
        let resolved = s.resolve_ref_unchecked(sidx);
        assert_eq!(*resolved, i as u64, 
            "First page StreamIndex[{}] should still resolve after page transition", i);
    }
    
    // And the new item on second page should resolve
    let resolved_next = s.resolve_ref_unchecked(&sidx_next);
    assert_eq!(*resolved_next, page_size as u64, "Second page item should resolve");
}

#[test]
fn append_with_index_interleaved_with_append() {
    // Goal: Verify append_with_index and append can be used together
    let s: SegmentedStream<u64> = SegmentedStream::with_page_size(64);
    
    let mut indices = Vec::new();
    
    // Mix append_with_index and regular append
    for i in 0..50 {
        if i % 2 == 0 {
            let sidx = s.append_with_index(i).expect("append_with_index");
            indices.push(Some(sidx));
        } else {
            s.append(i).expect("append");
            indices.push(None);
        }
    }
    
    // Verify all saved indices resolve correctly
    for (i, opt_sidx) in indices.iter().enumerate() {
        if let Some(sidx) = opt_sidx {
            let resolved = s.resolve_ref_unchecked(sidx);
            assert_eq!(*resolved, i as u64, 
                "Mixed usage: StreamIndex[{}] should resolve", i);
        }
    }
    
    // Verify forward iteration still works
    let mut cursor = Cursor::new_at_head(&s);
    let mut collected = Vec::new();
    while let Some(v) = cursor.next() {
        collected.push(*v);
    }
    assert_eq!(collected.len(), 50);
}

#[test]
fn append_with_index_stress_many_pages() {
    // Goal: Stress test with many pages to ensure no leaks or corruption
    let s: SegmentedStream<u64> = SegmentedStream::with_page_size(64);
    let count = 1000usize; // ~15 pages (64 * 15 = 960)
    
    let mut indices = Vec::new();
    for i in 0..count {
        let sidx = s.append_with_index(i as u64).expect("append should succeed");
        indices.push(sidx);
    }
    
    // Sample every 10th index for performance
    for i in (0..count).step_by(10) {
        let resolved = s.resolve_ref_unchecked(&indices[i]);
        assert_eq!(*resolved, i as u64, 
            "Stress test: StreamIndex[{}] should resolve", i);
    }
    
    // Verify first and last items
    let first = s.resolve_ref_unchecked(&indices[0]);
    assert_eq!(*first, 0);
    let last = s.resolve_ref_unchecked(&indices[count - 1]);
    assert_eq!(*last, (count - 1) as u64);
}

#[test]
fn append_with_index_page_pointers_remain_valid() {
    // Goal: Verify that page pointers in StreamIndex don't become dangling
    let s: SegmentedStream<u64> = SegmentedStream::with_page_size(64);
    
    // Insert across multiple pages
    let mut indices = Vec::new();
    for i in 0..150 {
        let sidx = s.append_with_index(i).expect("append");
        indices.push(sidx);
    }
    
    // Extract page pointers from first page indices
    let first_page_ptr = indices[0].page;
    let second_page_ptr = indices[64].page;
    let third_page_ptr = indices[128].page;
    
    // Verify pointers are distinct
    assert_ne!(first_page_ptr, second_page_ptr, "Pages should have different addresses");
    assert_ne!(second_page_ptr, third_page_ptr, "Pages should have different addresses");
    
    // Verify all indices from first page have same page pointer
    for i in 0..64 {
        assert_eq!(indices[i].page, first_page_ptr,
            "All indices in first page should share same page pointer");
    }
    
    // Verify all indices from second page have same page pointer
    for i in 64..128 {
        assert_eq!(indices[i].page, second_page_ptr,
            "All indices in second page should share same page pointer");
    }
    
    // Most importantly: verify the page pointers are still valid (not freed)
    // by successfully resolving through them
    for (i, sidx) in indices.iter().enumerate() {
        let resolved = s.resolve_ref_unchecked(sidx);
        assert_eq!(*resolved, i as u64, 
            "Page pointer for index {} should remain valid", i);
    }
}
