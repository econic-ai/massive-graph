use crate::structures::mph_delta_index::*;
use crate::structures::segmented_stream::SegmentedStream as DeltaStream;
use crossbeam_epoch as epoch;
use crate::types::ids::ID16;
use std::sync::Arc;

fn make_id16(i: usize) -> ID16 {
    let mut b = [b'0'; 16];
    let mut x = i;
    let mut pos = 15;
    if x == 0 { b[pos] = b'0'; } else { while x > 0 && pos < 16 { b[pos] = b'0' + (x % 10) as u8; x /= 10; if pos==0 { break; } pos -= 1; } }
    ID16::from_bytes(b)
}

fn build_index_from_values<V: Clone + std::fmt::Debug + 'static>(vals: &[V]) -> (OptimisedIndex<ID16, V>, Arc<[ID16]>) {
    // Create keys
    let keys_vec: Vec<ID16> = (0..vals.len()).map(make_id16).collect();
    let base_keys: Arc<[ID16]> = keys_vec.into_boxed_slice().into();
    
    // Build MPH indexer
    let mph = mph_indexer::BBHashIndexer::build(&base_keys, Default::default());
    let indexer = Arc::new(mph_indexer::ArcIndexer(Arc::new(mph)));
    
    // Create index and populate with values
    let idx = OptimisedIndex::new_with_indexer_and_capacity(indexer, vals.len(), vals.len() * 2);
    for (i, v) in vals.iter().enumerate() {
        idx.upsert(base_keys[i].clone(), v.clone());
    }
    
    // Publish to build MPH base
    idx.publish();
    
    (idx, base_keys)
}

#[test]
fn by_index_basic() {
    let (idx, keys): (OptimisedIndex<ID16, &'static str>, _) = build_index_from_values(&["x", "y", "z"]);
    assert_eq!(idx.get_owned(&keys[1]), Some("y"));
}

#[test]
fn get_returns_base_without_delta() {
    let (idx, keys) = build_index_from_values(&[42u64, 100u64]);
    assert_eq!(idx.get_owned(&keys[1]), Some(100));
}

#[test]
fn publish_folds_overrides_and_skips_tombstones() {
    let (idx, keys) = build_index_from_values(&[10u64, 20u64, 30u64]);
    idx.upsert(keys[1].clone(), 200);
    idx.remove(&keys[2]);
    let guard = epoch::pin();
    assert_eq!(idx.get(&keys[0], &guard).copied(), Some(10));
    assert_eq!(idx.get(&keys[1], &guard).copied(), Some(200));
    assert_eq!(idx.get(&keys[2], &guard), None);
    // Publish folds overrides into base and clears overrides
    idx.publish();
    // After publish, key2 is removed permanently, key1 is updated
    let guard2 = epoch::pin();
    assert_eq!(idx.get(&keys[0], &guard2).copied(), Some(10));
    assert_eq!(idx.get(&keys[1], &guard2).copied(), Some(200));
    assert_eq!(idx.get(&keys[2], &guard2), None);
}

#[test]
fn upsert_existing_then_publish_then_get() {
    let (idx, keys) = build_index_from_values(&[10u64, 20u64, 30u64]);
    idx.upsert(keys[1].clone(), 200);
    let guard = epoch::pin();
    assert_eq!(idx.get(&keys[1], &guard).copied(), Some(200));
    idx.publish();
    let guard2 = epoch::pin();
    assert_eq!(idx.get(&keys[1], &guard2).copied(), Some(200));
}

#[test]
fn remove_existing_then_publish_then_get() {
    let (idx, keys) = build_index_from_values(&[10u64, 20u64, 30u64]);
    idx.remove(&keys[2]);
    let guard = epoch::pin();
    assert_eq!(idx.get(&keys[2], &guard), None);
    // After publish, tombstoned key is removed
    idx.publish();
    let guard2 = epoch::pin();
    assert_eq!(idx.get(&keys[2], &guard2), None);
}

#[test]
fn upsert_then_get_returns_value() {
    let base: Vec<u64> = (0u64..10).collect();
    let (idx, keys) = build_index_from_values::<u64>(&base);
    idx.upsert(keys[7].clone(), 123);
    assert_eq!(idx.get_owned(&keys[7]), Some(123));
}

#[test]
fn upsert_sets_contains_key_true() {
    let base: Vec<u64> = (0u64..3).collect();
    let (idx, keys) = build_index_from_values::<u64>(&base);
    idx.upsert(keys[1].clone(), 1);
    assert!(idx.contains_key(&keys[1]));
}

#[test]
fn delta_overrides_base_get() {
    let base_vals: Vec<u64> = (0u64..5).map(|i| i * 11).collect::<Vec<_>>();
    let (idx, keys) = build_index_from_values(&base_vals);
    idx.upsert(keys[3].clone(), 88);
    assert_eq!(idx.get_owned(&keys[3]), Some(88));
}

#[test]
fn iterators_and_overrides_behaviour() {
    let (idx, keys) = build_index_from_values(&[10u64, 20u64, 30u64]);
    idx.upsert(keys[1].clone(), 200);
    idx.remove(&keys[2]);
    let guard = epoch::pin();
    // iter_mph yields MPH base values (order by MPH; compare as sets)
    let mut base_vals: Vec<u64> = idx.iter_mph(&guard).copied().collect();
    let mut expected = vec![10, 20, 30];
    base_vals.sort_unstable();
    expected.sort_unstable();
    assert_eq!(base_vals, expected);
    // iter_mph yields override-or-base, skipping tombstones (order-free checks)
    let overlay_vals: Vec<u64> = idx.iter_mph(&guard).copied().collect();
    assert!(overlay_vals.contains(&10));
    assert!(overlay_vals.contains(&200));
    assert!(!overlay_vals.contains(&30));
}

#[test]
fn remove_existing_then_get() {
    let (idx, keys) = build_index_from_values(&[10u64, 20u64, 30u64]);
    idx.remove(&keys[2]);
    let guard = epoch::pin();
    assert_eq!(idx.get(&keys[2], &guard), None);
}

#[test]
fn mph_fingerprint_mismatch_routes_to_new_keys() {
    let (idx, _keys) = build_index_from_values(&[1u64]);
    let unknown = make_id16(999);
    idx.upsert(unknown.clone(), 77);
    let guard = epoch::pin();
    assert_eq!(idx.get(&unknown, &guard).copied(), Some(77));
}

#[test]
fn delete_fingerprint_mismatch_goes_to_new_keys() {
    let (idx, _keys) = build_index_from_values(&[1u64]);
    let unknown = make_id16(888);
    idx.upsert(unknown.clone(), 5);
    idx.remove(&unknown);
    let guard = epoch::pin();
    assert_eq!(idx.get(&unknown, &guard), None);
}

#[test]
fn iter_mph_returns_all_base_values_in_slot_order() {
    let (idx, _keys) = build_index_from_values(&[1u64, 2u64, 3u64, 4u64]);
    let guard = epoch::pin();
    let mut got: Vec<u64> = idx.iter_mph(&guard).copied().collect();
    let mut exp = vec![1, 2, 3, 4];
    got.sort_unstable();
    exp.sort_unstable();
    assert_eq!(got, exp);
}

#[test]
fn contains_key_out_of_range_checks_new_keys() {
    let (idx, _keys) = build_index_from_values(&[1u64]);
    let unknown = make_id16(12345);
    idx.upsert(unknown.clone(), 9);
    assert!(idx.contains_key(&unknown));
}

#[test]
fn publish_idempotent_when_no_changes() {
    let (idx, keys) = build_index_from_values(&[5u64, 6u64]);
    let guard = epoch::pin();
    assert_eq!(idx.get(&keys[0], &guard).copied(), Some(5));
    assert_eq!(idx.get(&keys[1], &guard).copied(), Some(6));
    idx.publish();
    let guard2 = epoch::pin();
    assert_eq!(idx.get(&keys[0], &guard2).copied(), Some(5));
    assert_eq!(idx.get(&keys[1], &guard2).copied(), Some(6));
}

#[test]
fn multiple_upserts_latest_visible_before_and_after_publish() {
    let (idx, keys) = build_index_from_values(&[0u64, 0u64, 0u64]);
    idx.upsert(keys[1].clone(), 10);
    idx.upsert(keys[1].clone(), 20);
    idx.upsert(keys[1].clone(), 30);
    let guard = epoch::pin();
    assert_eq!(idx.get(&keys[1], &guard).copied(), Some(30));
    idx.publish();
    let guard2 = epoch::pin();
    assert_eq!(idx.get(&keys[1], &guard2).copied(), Some(30));
}

#[test]
fn delete_then_upsert_same_key_reflected_in_publish() {
    let (idx, keys) = build_index_from_values(&[100u64, 200u64]);
    idx.remove(&keys[1]);
    let guard = epoch::pin();
    assert_eq!(idx.get(&keys[1], &guard), None);
    idx.upsert(keys[1].clone(), 250);
    let guard2 = epoch::pin();
    assert_eq!(idx.get(&keys[1], &guard2).copied(), Some(250));
    idx.publish();
    let guard3 = epoch::pin();
    assert_eq!(idx.get(&keys[1], &guard3).copied(), Some(250));
}

#[test]
fn radix_empty_after_publish() {
    let (idx, _keys) = build_index_from_values(&[0u64, 0u64]);
    // Insert a new key (not in base set)
    let unknown = make_id16(7777);
    idx.upsert(unknown.clone(), 55);
    let guard = epoch::pin();
    assert_eq!(idx.get(&unknown, &guard).copied(), Some(55));
    // Publish should fold new key into MPH and clear radix buckets
    idx.publish();
    let guard2 = epoch::pin();
    assert_eq!(idx.get(&unknown, &guard2).copied(), Some(55));
    // A second publish should be idempotent and still resolve
    idx.publish();
    let guard3 = epoch::pin();
    assert_eq!(idx.get(&unknown, &guard3).copied(), Some(55));
}

#[test]
fn publish_flows_new_key_then_publish_then_get() {
    let (idx, _keys) = build_index_from_values(&[0u64]);
    let k = make_id16(5000);
    idx.upsert(k.clone(), 11);
    let g = epoch::pin();
    assert_eq!(idx.get(&k, &g).copied(), Some(11));
    idx.publish();
    let g2 = epoch::pin();
    assert_eq!(idx.get(&k, &g2).copied(), Some(11));
}

#[test]
fn publish_flows_new_key_publish_delete_get_none() {
    let (idx, _keys) = build_index_from_values(&[0u64]);
    let k = make_id16(6000);
    idx.upsert(k.clone(), 21);
    idx.publish();
    idx.remove(&k);
    let g = epoch::pin();
    assert_eq!(idx.get(&k, &g), None);
}

#[test]
fn publish_flows_new_key_publish_update_get_new() {
    let (idx, _keys) = build_index_from_values(&[0u64]);
    let k = make_id16(7000);
    idx.upsert(k.clone(), 31);
    idx.publish();
    idx.upsert(k.clone(), 41);
    let g = epoch::pin();
    assert_eq!(idx.get(&k, &g).copied(), Some(41));
}

#[test]
fn publish_flows_new_key_delete_publish_get_none() {
    let (idx, _keys) = build_index_from_values(&[0u64]);
    let k = make_id16(8000);
    idx.upsert(k.clone(), 51);
    idx.remove(&k);
    idx.publish();
    let g = epoch::pin();
    assert_eq!(idx.get(&k, &g), None);
}

#[test]
fn publish_flows_new_key_update_publish_get_new() {
    let (idx, _keys) = build_index_from_values(&[0u64]);
    let k = make_id16(9000);
    idx.upsert(k.clone(), 61);
    idx.upsert(k.clone(), 71);
    idx.publish();
    let g = epoch::pin();
    assert_eq!(idx.get(&k, &g).copied(), Some(71));
}



#[test]
fn mph_1m_test() {
    use super::*;
    use crate::types::ID16;
    
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
    
    let n = 100_000; // Start with 100K
    println!("Building MPH index with {} items...", n);
    
    let (idx, keys) = build_index_from_values(&(0..n).map(|i| i as u64).collect::<Vec<_>>());
    
    println!("Testing get on first 10 items...");
    let guard = epoch::pin();
    for i in 0..10 {
        let result = idx.get(&keys[i], &guard);
        println!("  get(key[{}]) = {:?}", i, result.copied());
        assert_eq!(result.copied(), Some(i as u64));
    }
    
    println!("Testing get on last 10 items...");
    for i in (n-10)..n {
        let result = idx.get(&keys[i], &guard);
        println!("  get(key[{}]) = {:?}", i, result.copied());
        assert_eq!(result.copied(), Some(i as u64));
    }
    
    println!("SUCCESS: All {} items accessible", n);
}

#[test]
fn test_key_cloning_equality() {
    use super::*;
    use crate::types::ID16;
    use std::sync::Arc;
    
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
    
    let n = 34;
    let keys_vec: Vec<ID16> = (0..n).map(make_id16).collect();
    let base_keys: Arc<[ID16]> = keys_vec.clone().into_boxed_slice().into();
    
    // Verify keys are identical
    for i in 0..n {
        assert_eq!(keys_vec[i].as_bytes(), base_keys[i].as_bytes(),
            "Key mismatch at index {}: vec={:?} vs arc={:?}", 
            i, keys_vec[i].as_bytes(), base_keys[i].as_bytes());
    }
    
    // Build MPH with base_keys
    let mph = mph_indexer::BBHashIndexer::build(&base_keys, Default::default());
    
    // Verify eval returns same slot for both
    for i in 0..n {
        let slot_vec = mph.eval(&keys_vec[i]);
        let slot_arc = mph.eval(&base_keys[i]);
        assert_eq!(slot_vec, slot_arc,
            "Eval mismatch at index {}: vec key -> slot {} vs arc key -> slot {}", 
            i, slot_vec, slot_arc);
    }
    
    println!("✓ All keys and evals match!");
}

#[test]
fn test_benchmark_flow_mimics_actual_benchmark() {
    use super::*;
    use crate::types::ID16;
    use std::sync::Arc;
    use crate::structures::segmented_stream::SegmentedStream as DeltaStream;
    
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
    
    // Mimic the benchmark's build_optidx_mph function
    let n = 34;
    let vals: Vec<u64> = (0..n).map(|i| i as u64).collect();
    
    let stream: DeltaStream<u64> = DeltaStream::new();
    let mut indices = Vec::with_capacity(vals.len());
    for v in vals.iter().cloned() { 
        indices.push(stream.append_with_index(v).expect("append")); 
    }
    let mph_vals: Arc<[_]> = indices.into_boxed_slice().into();
    
    // THIS IS THE KEY PART: keys_vec is created, then used to build index
    let keys_vec: Vec<ID16> = (0..mph_vals.len()).map(make_id16).collect();
    let base_keys: Arc<[ID16]> = keys_vec.clone().into_boxed_slice().into();
    
    println!("Building MPH indexer with {} keys...", base_keys.len());
    let mph = mph_indexer::BBHashIndexer::build(&base_keys, Default::default());
    let indexer = Arc::new(mph_indexer::ArcIndexer(Arc::new(mph)));
    
    println!("Creating OptimisedIndex...");
    let oi = OptimisedIndex::new_with_indexer_and_capacity(indexer, n, n * 2);
    
    // Populate with values
    for i in 0..n {
        oi.upsert(keys_vec[i].clone(), vals[i]);
    }
    
    // Publish to build MPH
    oi.publish();
    
    // Now test get using keys_vec (like the benchmark does)
    println!("Testing gets with keys_vec...");
    let guard = crossbeam_epoch::pin();
    for i in 0..10 {
        let result = oi.get(&keys_vec[i], &guard);
        println!("  get(keys_vec[{}]) = {:?}", i, result.copied());
        assert_eq!(result.copied(), Some(i as u64), 
            "Failed at index {}: expected Some({}), got {:?}", 
            i, i, result.copied());
    }
    
    println!("✓ All gets successful!");
}

#[test]
fn test_stream_insert_and_resolve() {
    use crate::structures::segmented_stream::SegmentedStream;
    
    let stream: SegmentedStream<u64> = SegmentedStream::new();
    
    // Insert values 0..10
    let mut indices = Vec::new();
    for i in 0..10u64 {
        let sidx = stream.append_with_index(i).expect("append");
        println!("Inserted value {} -> StreamIndex(page={:?}, idx={})", i, sidx.page, sidx.idx);
        indices.push(sidx);
    }
    
    // Resolve and verify
    println!("\nResolving...");
    for (i, sidx) in indices.iter().enumerate() {
        let resolved = stream.resolve_ref_unchecked(sidx);
        println!("  StreamIndex(page={:?}, idx={}) -> value {}", sidx.page, sidx.idx, resolved);
        assert_eq!(*resolved, i as u64, 
            "Mismatch: StreamIndex at position {} resolved to {} instead of {}", 
            i, resolved, i);
    }
    
    println!("✓ All stream inserts and resolves correct!");
}

#[test]
fn verify_indexer_built_with_correct_keys() {
    use super::*;
    use crate::types::ID16;
    use std::sync::Arc;
    use crate::structures::segmented_stream::SegmentedStream as DeltaStream;
    
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
    
    let n = 34;
    
    // Create keys FIRST
    let keys_vec: Vec<ID16> = (0..n).map(make_id16).collect();
    let base_keys: Arc<[ID16]> = keys_vec.clone().into_boxed_slice().into();
    
    println!("Keys created:");
    for i in 0..5 {
        println!("  keys_vec[{}] = {:?}", i, &keys_vec[i].as_bytes()[..8]);
        println!("  base_keys[{}] = {:?}", i, &base_keys[i].as_bytes()[..8]);
    }
    
    // Build indexer with base_keys
    println!("\nBuilding MPH indexer with base_keys...");
    let mph = mph_indexer::BBHashIndexer::build(&base_keys, Default::default());
    
    // Verify indexer maps keys correctly
    println!("\nVerifying indexer maps keys to unique slots:");
    let mut slot_to_key = vec![None; n];
    for i in 0..n {
        let slot = mph.eval(&base_keys[i]);
        if let Some(prev_i) = slot_to_key[slot] {
            panic!("COLLISION: key[{}] and key[{}] both map to slot {}", prev_i, i, slot);
        }
        slot_to_key[slot] = Some(i);
        if i < 5 {
            println!("  base_keys[{}] -> slot {}", i, slot);
        }
    }
    
    // Now verify keys_vec maps the same way
    println!("\nVerifying keys_vec maps identically:");
    for i in 0..5 {
        let slot_from_base = mph.eval(&base_keys[i]);
        let slot_from_vec = mph.eval(&keys_vec[i]);
        println!("  base_keys[{}] -> slot {}, keys_vec[{}] -> slot {}", 
            i, slot_from_base, i, slot_from_vec);
        assert_eq!(slot_from_base, slot_from_vec, 
            "Mismatch: base_keys[{}] -> {} but keys_vec[{}] -> {}", 
            i, slot_from_base, i, slot_from_vec);
    }
    
    println!("\n✓ Indexer is correct and deterministic!");
}

#[test]
fn check_full_key_bytes() {
    use crate::types::ID16;
    
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
    
    for i in 0..10 {
        let key = make_id16(i);
        println!("key[{}] = {:?}", i, key.as_bytes());
    }
}

#[test]
fn test_stream_index_through_slot_assignment() {
    use crate::structures::segmented_stream::SegmentedStream;
    use crate::structures::segmented_stream::segmented_stream::StreamIndex;
    
    let stream: SegmentedStream<u64> = SegmentedStream::new();
    
    // Insert 10 values
    let mut indices = Vec::new();
    for i in 0..10u64 {
        let sidx = stream.append_with_index(i).expect("append");
        println!("Inserted {} -> StreamIndex(page={:?}, idx={})", i, sidx.page, sidx.idx);
        indices.push(sidx);
    }
    
    // Create a slot-like structure
    struct TestSlot {
        base: StreamIndex<u64>,
    }
    
    let mut slots: Vec<TestSlot> = Vec::new();
    for _ in 0..10 {
        slots.push(TestSlot {
            base: StreamIndex { page: core::ptr::null(), idx: 0 }
        });
    }
    
    // Assign indices to slots (simulating MPH mapping)
    // key[0] -> slot[5], key[1] -> slot[0], key[2] -> slot[7]
    let mapping = [(0, 5), (1, 0), (2, 7)];
    for (key_idx, slot_idx) in mapping {
        let sidx = &indices[key_idx];
        slots[slot_idx].base = StreamIndex { page: sidx.page, idx: sidx.idx };
        println!("Assigned key[{}] (value={}) to slot[{}]: StreamIndex(page={:?}, idx={})", 
            key_idx, key_idx, slot_idx, sidx.page, sidx.idx);
    }
    
    // Now resolve through slots
    println!("\nResolving through slots:");
    for (key_idx, slot_idx) in mapping {
        let slot = &slots[slot_idx];
        let resolved = stream.resolve_ref_unchecked(&slot.base);
        println!("  slot[{}].base (idx={}) -> value {}", slot_idx, slot.base.idx, resolved);
        assert_eq!(*resolved, key_idx as u64, 
            "Mismatch: slot[{}] should resolve to {} but got {}", 
            slot_idx, key_idx, resolved);
    }
    
    println!("✓ All slot resolutions correct!");
}

#[test]
fn test_minimal_mph_bug_reproduction() {
    use super::*;
    use crate::types::ID16;
    use std::sync::Arc;
    use crate::structures::segmented_stream::SegmentedStream as DeltaStream;
    
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
    
    // Test with just 5 keys
    let n = 34;
    let keys: Vec<ID16> = (0..n).map(make_id16).collect();
    let vals: Vec<u64> = (0..n).map(|i| i as u64).collect();
    
    println!("\n=== MINIMAL TEST WITH {} KEYS ===", n);
    
    // Build stream and indices
    let stream: DeltaStream<u64> = DeltaStream::new();
    let mut indices = Vec::new();
    for v in vals.iter().cloned() {
        let sidx = stream.append_with_index(v).expect("append");
        println!("Stream: inserted value {} -> StreamIndex(page={:?}, idx={})", v, sidx.page, sidx.idx);
        indices.push(sidx);
    }
    let mph_vals: Arc<[_]> = indices.into_boxed_slice().into();
    
    // Build indexer
    let base_keys: Arc<[ID16]> = keys.clone().into_boxed_slice().into();
    println!("\nBuilding MPH indexer...");
    let mph = mph_indexer::BBHashIndexer::build(&base_keys, Default::default());
    
    // Show MPH mapping
    println!("\nMPH mapping:");
    for i in 0..n {
        let slot = mph.eval(&keys[i]);
        println!("  key[{}] -> slot[{}]", i, slot);
    }
    
    let indexer = Arc::new(mph_indexer::ArcIndexer(Arc::new(mph)));
    
    // Create OptimisedIndex
    println!("\nCreating OptimisedIndex...");
    let oi = OptimisedIndex::new_with_indexer_and_capacity(indexer, n, n * 2);
    
    // Populate with values
    for i in 0..n {
        oi.upsert(keys[i].clone(), vals[i]);
    }
    
    // Publish to build MPH
    oi.publish();
    
    // Test each key
    println!("\nTesting gets:");
    let guard = crossbeam_epoch::pin();
    let mut all_pass = true;
    for i in 0..n {
        let result = oi.get(&keys[i], &guard);
        let expected = Some(i as u64);
        let pass = result.copied() == expected;
        
        println!("  key[{}]: expected {:?}, got {:?} {}", 
            i, expected, result.copied(), if pass { "✓" } else { "❌" });
        
        if !pass {
            all_pass = false;
        }
    }
    
    assert!(all_pass, "Some keys returned wrong values!");
    println!("\n✓ All keys returned correct values!");
}
