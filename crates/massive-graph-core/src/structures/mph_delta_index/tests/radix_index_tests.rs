// RadixIndex unit tests - moved from radix_index.rs module
// These tests need to be updated to work with the new value-agnostic RadixIndex API

use crate::structures::mph_delta_index::radix_index::RadixIndex;
use crate::structures::segmented_stream::{SegmentedStream, StreamIndex};
use crate::types::ID16;
use crossbeam_epoch as epoch;

fn make_id16(i: usize) -> ID16 {
    let mut b = [b'0'; 16];
    let mut x = i;
    let mut pos = 15;
    if x == 0 { b[pos] = b'0'; } 
    else { 
        while x > 0 && pos < 16 { 
            b[pos] = b'0' + (x % 10) as u8; 
            x /= 10; 
            if pos==0 { break; } 
            pos -= 1; 
        } 
    }
    ID16::from_bytes(b)
}

#[test]
fn radix_single_page_all_resolve() {
    // Goal: Verify basic upsert and get within a single stream page
    let stream: SegmentedStream<u64> = SegmentedStream::with_page_size(64);
    let idx: RadixIndex<ID16, StreamIndex<u64>> = RadixIndex::with_capacity(100, 400);
    let guard = epoch::pin();
    
    let count = 20; // Well below page size (64)
    for i in 0..count {
        let key = make_id16(i);
        let sidx = stream.append_with_index(i as u64).expect("append");
        idx.upsert(&key, &sidx, &guard);
    }
    
    // Verify all keys resolve correctly
    for i in 0..count {
        let key = make_id16(i);
        let sidx = idx.get(&key, &guard).expect("key should exist");
        let value = stream.resolve_ref_unchecked(sidx);
        assert_eq!(*value, i as u64,
            "Key {} should resolve to {}", i, i);
    }
}

#[test]
fn radix_across_stream_link_ahead_boundary() {
    // Goal: Verify keys remain accessible when stream crosses LINK_AHEAD (32)
    let stream: SegmentedStream<u64> = SegmentedStream::with_page_size(64);
    let idx: RadixIndex<ID16, StreamIndex<u64>> = RadixIndex::with_capacity(100, 400);
    let guard = epoch::pin();
    
    let count = 40; // Cross LINK_AHEAD=32 (page_size/2), but stay within first page (64)
    for i in 0..count {
        let key = make_id16(i);
        let sidx = stream.append_with_index(i as u64).expect("append");
        idx.upsert(&key, &sidx, &guard);
    }
    
    // Critical: keys 0-31 should still resolve after index 32 triggers stream pre-linking
    for i in 0..count {
        let key = make_id16(i);
        let sidx = idx.get(&key, &guard).expect("key should exist");
        let value = stream.resolve_ref_unchecked(sidx);
        assert_eq!(*value, i as u64,
            "Key {} should resolve even after stream LINK_AHEAD", i);
    }
}

#[test]
fn radix_across_stream_page_boundary() {
    // Goal: Verify keys remain accessible when stream crosses page boundary (64)
    let stream: SegmentedStream<u64> = SegmentedStream::with_page_size(64);
    let idx: RadixIndex<ID16, StreamIndex<u64>> = RadixIndex::with_capacity(200, 800);
    let guard = epoch::pin();
    
    let count = 100; // Cross page boundary at 64, spanning two stream pages
    for i in 0..count {
        let key = make_id16(i);
        let sidx = stream.append_with_index(i as u64).expect("append");
        idx.upsert(&key, &sidx, &guard);
    }
    
    // Critical: ALL keys should resolve, including those from the first stream page
    for i in 0..count {
        let key = make_id16(i);
        let sidx = idx.get(&key, &guard).expect("key should exist");
        let value = stream.resolve_ref_unchecked(sidx);
        assert_eq!(*value, i as u64,
            "Key {} should resolve across stream page boundary", i);
    }
}

#[test]
fn radix_random_access_after_multiple_stream_pages() {
    // Goal: Verify random access to keys whose values are on old stream pages
    let stream: SegmentedStream<u64> = SegmentedStream::with_page_size(64);
    let idx: RadixIndex<ID16, StreamIndex<u64>> = RadixIndex::with_capacity(300, 1200);
    let guard = epoch::pin();
    
    let count = 200; // Span 3+ stream pages (64 * 3 = 192)
    for i in 0..count {
        let key = make_id16(i);
        let sidx = stream.append_with_index(i as u64).expect("append");
        idx.upsert(&key, &sidx, &guard);
    }
    
    // Test random access pattern: backwards, forwards, random
    
    // 1. Backwards traversal
    for i in (0..count).rev() {
        let key = make_id16(i);
        let sidx = idx.get(&key, &guard).expect("key should exist");
        let value = stream.resolve_ref_unchecked(sidx);
        assert_eq!(*value, i as u64,
            "Backwards: Key {} should resolve", i);
    }
    
    // 2. Random access to first stream page after being on third page
    let first_page_indices = [0, 10, 31, 50, 63];
    for &i in &first_page_indices {
        let key = make_id16(i);
        let sidx = idx.get(&key, &guard).expect("key should exist");
        let value = stream.resolve_ref_unchecked(sidx);
        assert_eq!(*value, i as u64,
            "Random access: Key {} should resolve", i);
    }
    
    // 3. Interleaved access across stream pages
    let interleaved = [5, 70, 15, 130, 25, 190, 35];
    for &i in &interleaved {
        let key = make_id16(i);
        let sidx = idx.get(&key, &guard).expect("key should exist");
        let value = stream.resolve_ref_unchecked(sidx);
        assert_eq!(*value, i as u64,
            "Interleaved: Key {} should resolve", i);
    }
}

#[test]
fn radix_update_then_access_across_pages() {
    // Goal: Verify updates work correctly when values span multiple stream pages
    let stream: SegmentedStream<u64> = SegmentedStream::with_page_size(64);
    let idx: RadixIndex<ID16, StreamIndex<u64>> = RadixIndex::with_capacity(200, 800);
    let guard = epoch::pin();
    
    // Insert initial values
    let count = 100;
    for i in 0..count {
        let key = make_id16(i);
        let sidx = stream.append_with_index(i as u64).expect("append");
        idx.upsert(&key, &sidx, &guard);
    }
    
    // Update some keys with new values (which will be on later stream pages)
    let update_indices = [5, 35, 65, 95];
    for &i in &update_indices {
        let key = make_id16(i);
        let new_val = (i + 1000) as u64;
        let sidx = stream.append_with_index(new_val).expect("append");
        idx.upsert(&key, &sidx, &guard);
    }
    
    // Verify updated keys return new values
    for &i in &update_indices {
        let key = make_id16(i);
        let sidx = idx.get(&key, &guard).expect("key should exist");
        let value = stream.resolve_ref_unchecked(sidx);
        assert_eq!(*value, (i + 1000) as u64,
            "Updated key {} should resolve to new value", i);
    }
    
    // Verify non-updated keys still return original values
    for i in 0..count {
        if !update_indices.contains(&i) {
            let key = make_id16(i);
            let sidx = idx.get(&key, &guard).expect("key should exist");
            let value = stream.resolve_ref_unchecked(&sidx);
            assert_eq!(*value, i as u64,
                "Non-updated key {} should resolve to original value", i);
        }
    }
}

#[test]
fn radix_delete_then_access_across_pages() {
    // Goal: Verify deletes work correctly when values span multiple stream pages
    let stream: SegmentedStream<u64> = SegmentedStream::with_page_size(64);
    let idx: RadixIndex<ID16, StreamIndex<u64>> = RadixIndex::with_capacity(200, 800);
    let guard = epoch::pin();
    
    // Insert initial values
    let count = 100;
    for i in 0..count {
        let key = make_id16(i);
        let sidx = stream.append_with_index(i as u64).expect("append");
        idx.upsert(&key, &sidx, &guard);
    }
    
    // Delete some keys
    let delete_indices = [5, 35, 65, 95];
    for &i in &delete_indices {
        let key = make_id16(i);
        idx.delete(&key, &guard);
    }
    
    // Verify deleted keys return None
    for &i in &delete_indices {
        let key = make_id16(i);
        let result = idx.get(&key, &guard);
        assert!(result.is_none(),
            "Deleted key {} should return None", i);
    }
    
    // Verify non-deleted keys still return original values
    for i in 0..count {
        if !delete_indices.contains(&i) {
            let key = make_id16(i);
            let sidx = idx.get(&key, &guard).expect("key should exist");
            let value = stream.resolve_ref_unchecked(&sidx);
            assert_eq!(*value, i as u64,
                "Non-deleted key {} should resolve to original value", i);
        }
    }
}

#[test]
fn radix_iteration_across_stream_pages() {
    // Goal: Verify iteration works correctly when values span multiple stream pages
    let stream: SegmentedStream<u64> = SegmentedStream::with_page_size(64);
    let idx: RadixIndex<ID16, StreamIndex<u64>> = RadixIndex::with_capacity(200, 800);
    let guard = epoch::pin();
    
    let count = 100;
    for i in 0..count {
        let key = make_id16(i);
        let sidx = stream.append_with_index(i as u64).expect("append");
        idx.upsert(&key, &sidx, &guard);
    }
    
    // Collect all values via iteration
    let mut collected: Vec<u64> = idx.iter(&guard)
        .map(|sidx| *stream.resolve_ref_unchecked(sidx))
        .collect();
    collected.sort_unstable();
    
    // Verify all values are present
    assert_eq!(collected.len(), count);
    for i in 0..count {
        assert!(collected.contains(&(i as u64)),
            "Iteration should include value {}", i);
    }
}

#[test]
fn radix_stress_many_stream_pages() {
    // Goal: Stress test with many stream pages to ensure no leaks or corruption
    let stream: SegmentedStream<u64> = SegmentedStream::with_page_size(64);
    let idx: RadixIndex<ID16, StreamIndex<u64>> = RadixIndex::with_capacity(1500, 6000);
    let guard = epoch::pin();
    
    let count = 1000; // ~15 stream pages
    for i in 0..count {
        let key = make_id16(i);
        let sidx = stream.append_with_index(i as u64).expect("append");
        idx.upsert(&key, &sidx, &guard);
    }
    
    // Sample every 10th key for performance
    for i in (0..count).step_by(10) {
        let key = make_id16(i);
        let sidx = idx.get(&key, &guard).expect("key should exist");
        let value = stream.resolve_ref_unchecked(sidx);
        assert_eq!(*value, i as u64,
            "Stress test: Key {} should resolve", i);
    }
    
    // Verify first and last keys
    let first_key = make_id16(0);
    let first_sidx = idx.get(&first_key, &guard).expect("first key should exist");
    assert_eq!(*stream.resolve_ref_unchecked(&first_sidx), 0);
    
    let last_key = make_id16(count - 1);
    let last_sidx = idx.get(&last_key, &guard).expect("last key should exist");
    assert_eq!(*stream.resolve_ref_unchecked(&last_sidx), (count - 1) as u64);
}

#[test]
fn radix_mixed_operations_across_pages() {
    // Goal: Verify mixed insert/update/delete operations across stream pages
    let stream: SegmentedStream<u64> = SegmentedStream::with_page_size(64);
    let idx: RadixIndex<ID16, StreamIndex<u64>> = RadixIndex::with_capacity(300, 1200);
    let guard = epoch::pin();
    
    // Phase 1: Insert 50 keys (stream page 0)
    for i in 0..50 {
        let key = make_id16(i);
        let sidx = stream.append_with_index(i as u64).expect("append");
        idx.upsert(&key, &sidx, &guard);
    }
    
    // Phase 2: Insert 50 more keys (stream page 0-1)
    for i in 50..100 {
        let key = make_id16(i);
        let sidx = stream.append_with_index(i as u64).expect("append");
        idx.upsert(&key, &sidx, &guard);
    }
    
    // Phase 3: Update some keys from phase 1 (values now on stream page 1-2)
    for i in (0..50).step_by(5) {
        let key = make_id16(i);
        let new_val = (i + 2000) as u64;
        let sidx = stream.append_with_index(new_val).expect("append");
        idx.upsert(&key, &sidx, &guard);
    }
    
    // Phase 4: Delete some keys from phase 2
    for i in (50..100).step_by(5) {
        let key = make_id16(i);
        idx.delete(&key, &guard);
    }
    
    // Verify phase 1 keys (updated)
    for i in (0..50).step_by(5) {
        let key = make_id16(i);
        let sidx = idx.get(&key, &guard).expect("key should exist");
        let value = stream.resolve_ref_unchecked(sidx);
        assert_eq!(*value, (i + 2000) as u64,
            "Updated key {} should have new value", i);
    }
    
    // Verify phase 1 keys (not updated)
    for i in 0..50 {
        if i % 5 != 0 {
            let key = make_id16(i);
            let sidx = idx.get(&key, &guard).expect("key should exist");
            let value = stream.resolve_ref_unchecked(&sidx);
            assert_eq!(*value, i as u64,
                "Non-updated key {} should have original value", i);
        }
    }
    
    // Verify phase 2 keys (deleted)
    for i in (50..100).step_by(5) {
        let key = make_id16(i);
        let result = idx.get(&key, &guard);
        assert!(result.is_none(),
            "Deleted key {} should return None", i);
    }
    
    // Verify phase 2 keys (not deleted)
    for i in 50..100 {
        if i % 5 != 0 {
            let key = make_id16(i);
            let sidx = idx.get(&key, &guard).expect("key should exist");
            let value = stream.resolve_ref_unchecked(&sidx);
            assert_eq!(*value, i as u64,
                "Non-deleted key {} should have original value", i);
        }
    }
}

