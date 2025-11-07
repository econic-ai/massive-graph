//! Test to verify MPH index building and lookup is working correctly.
//! This test is in the module root (not tests/) to debug the current issue.

#[cfg(test)]
mod tests {
    use crate::structures::mph_delta_index::{OptimisedIndex, mph_indexer::{BBHashIndexer, MphIndexer}};
    use crate::types::ids::ID16;

    #[test]
    fn test_mph_index_basic_lookup() {
        println!("\n=== Testing MPH Index Basic Lookup ===\n");
        
        // Create a small set of keys
        let keys: Vec<ID16> = vec![
            ID16::random(),
            ID16::random(),
            ID16::random(),
            ID16::random(),
        ];
        
        let vals: Vec<u32> = vec![100, 200, 300, 400];
        
        // Build an OptimisedIndex with BBHashIndexer (default)
        let idx = OptimisedIndex::new_with_capacity(16, 64);
        
        // Insert all key-value pairs
        for (k, v) in keys.iter().zip(vals.iter()) {
            println!("Inserting key: {:?} -> value: {}", k, v);
            idx.upsert(k.clone(), *v);
        }
        
        println!("\nPublishing index...");
        idx.publish();
        
        println!("\n=== Testing Lookups ===\n");
        
        // Test lookups
        let guard = crossbeam_epoch::pin();
        for (k, expected_val) in keys.iter().zip(vals.iter()) {
            println!("Looking up key: {:?}", k);
            
            match idx.get(k, &guard) {
                Some(actual_val) => {
                    println!("  ✓ Found value: {} (expected: {})", actual_val, expected_val);
                    assert_eq!(actual_val, expected_val, "Value mismatch for key {:?}", k);
                }
                None => {
                    println!("  ✗ NOT FOUND (expected: {})", expected_val);
                    panic!("Key {:?} not found in index after publish!", k);
                }
            }
        }
        
        println!("\n=== All lookups successful ===\n");
    }
    
    #[test]
    fn test_mph_index_slot_placement() {
        println!("\n=== Testing MPH Index Slot Placement ===\n");
        
        // Create keys
        let keys: Vec<ID16> = (0..10).map(|_| ID16::random()).collect();
        
        // Build BBHashIndexer directly
        let indexer = BBHashIndexer::build(&keys, Default::default());
        
        println!("Built indexer for {} keys", keys.len());
        
        // For each key, print what index the indexer assigns
        for (i, key) in keys.iter().enumerate() {
            let idx = indexer.eval(key);
            println!("Key[{}] {:?} -> slot index {}", i, key, idx);
        }
        
        // Now build an MPHIndex and verify slot placement
        use crate::structures::mph_delta_index::mph_index::{MPHIndex, Slot};
        use crate::structures::segmented_stream::segmented_stream::StreamIndex;
        use crate::structures::mph_delta_index::util::{hash64, tag16_from_hash};
        
        let mut slots = Vec::new();
        for (i, key) in keys.iter().enumerate() {
            let h = hash64::<ID16>(key);
            let tag16 = tag16_from_hash(h);
            let sidx = StreamIndex { page: std::ptr::null(), idx: i as u32 };
            slots.push(Slot::new(tag16, h, key.clone(), sidx));
        }
        
        println!("\nBuilding MPHIndex from {} slots", slots.len());
        let mph = MPHIndex::from_slots(slots, indexer.clone());
        
        println!("\n=== Verifying Lookups ===\n");
        
        // Try to look up each key
        for (i, key) in keys.iter().enumerate() {
            let eval_idx = indexer.eval(key);
            println!("Looking up Key[{}] {:?}", i, key);
            println!("  Indexer.eval() returned: {}", eval_idx);
            
            match mph.get(key) {
                Some(sidx) => {
                    println!("  ✓ Found StreamIndex.idx: {}", sidx.idx);
                    assert_eq!(sidx.idx as usize, i, "StreamIndex mismatch for key {:?}", key);
                }
                None => {
                    println!("  ✗ NOT FOUND");
                    
                    // Debug: print what's in the slot at this index
                    if eval_idx < mph.slots.len() {
                        let slot = &mph.slots[eval_idx];
                        let expected_hash = hash64::<ID16>(key);
                        println!("  Debug: slot[{}] has:", eval_idx);
                        println!("    - key: {:?}", slot.key);
                        println!("    - hash64: {} (expected: {})", slot.hash64, expected_hash);
                        println!("    - match: {}", slot.hash64 == expected_hash);
                    }
                    
                    panic!("Key {:?} not found in MPH index!", key);
                }
            }
        }
        
        println!("\n=== All slot placements correct ===\n");
    }
}

