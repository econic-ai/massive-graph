//! Test to verify BBHashIndexer determinism across multiple builds.
//! This test currently FAILS because AHasher::default() uses random seeds.

#[cfg(test)]
mod tests {
    use crate::structures::mph_delta_index::mph_indexer::{BBHashIndexer, MphIndexer, BbhConfig};
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

    /// Test that BBHashIndexer produces the SAME slot assignments across multiple builds.
    /// This test FAILS with the current implementation because AHasher::default() is randomized.
    #[test]
    fn test_bbhash_determinism_across_builds() {
        let n = 100;
        let keys: Vec<ID16> = (0..n).map(make_id16).collect();
        
        // Build the indexer 3 times with the same keys
        println!("Building indexer 3 times with the same {} keys...", n);
        let mph1 = BBHashIndexer::build(&keys, Default::default());
        let mph2 = BBHashIndexer::build(&keys, Default::default());
        let mph3 = BBHashIndexer::build(&keys, Default::default());
        
        // Collect slot assignments from each build
        let mut slots1 = Vec::new();
        let mut slots2 = Vec::new();
        let mut slots3 = Vec::new();
        
        for (i, key) in keys.iter().enumerate() {
            let slot1 = mph1.eval(key);
            let slot2 = mph2.eval(key);
            let slot3 = mph3.eval(key);
            
            slots1.push(slot1);
            slots2.push(slot2);
            slots3.push(slot3);
            
            // Log first 5 for debugging
            if i < 5 {
                println!("  key[{}]: mph1 -> slot {}, mph2 -> slot {}, mph3 -> slot {}", 
                    i, slot1, slot2, slot3);
            }
        }
        
        // Verify all three builds produce the same slot assignments
        let mut mismatches = 0;
        for i in 0..n {
            if slots1[i] != slots2[i] || slots1[i] != slots3[i] {
                if mismatches < 5 {
                    println!("MISMATCH at key[{}]: mph1={}, mph2={}, mph3={}", 
                        i, slots1[i], slots2[i], slots3[i]);
                }
                mismatches += 1;
            }
        }
        
        if mismatches > 0 {
            println!("\n❌ FAILED: {} out of {} keys mapped to different slots across builds", 
                mismatches, n);
            println!("This is because AHasher::default() uses random seeds.");
            panic!("BBHashIndexer is non-deterministic across builds!");
        }
        
        println!("\n✓ All {} keys map to the same slots across all 3 builds", n);
    }

    /// Test that demonstrates the practical impact: construction vs get mismatch
    #[test]
    fn test_construction_vs_get_mismatch() {
        use std::sync::Arc;
        use crate::structures::segmented_stream::SegmentedStream;
        use crate::structures::mph_delta_index::OptimisedIndex;
        use crate::structures::mph_delta_index::mph_indexer::BBHashIndexer;
        
        let n = 100;
        let keys: Vec<ID16> = (0..n).map(make_id16).collect();
        let vals: Vec<u64> = (0..n).map(|i| i as u64).collect();
        
        // Build stream and indices
        let stream: SegmentedStream<u64> = SegmentedStream::new();
        let mut indices = Vec::new();
        for v in vals.iter().cloned() {
            indices.push(stream.append_with_index(v).expect("append"));
        }
        let mph_vals: Arc<[_]> = indices.into_boxed_slice().into();
        
        // Build indexer
        let base_keys: Arc<[ID16]> = keys.clone().into_boxed_slice().into();
        println!("Building MPH indexer...");
        let mph = BBHashIndexer::build(&base_keys, Default::default());
        
        // Log first 3 slot assignments during construction
        println!("Slot assignments during construction:");
        for i in 0..3 {
            let slot = mph.eval(&keys[i]);
            println!("  key[{}] -> slot {}", i, slot);
        }
        
        let indexer = mph;
        
        // Create OptimisedIndex (this uses the indexer during construction)
        println!("\nCreating OptimisedIndex...");
        let oi = OptimisedIndex::new_with_indexer_and_capacity(indexer, n, n * 2);
        
        // Populate with values
        for i in 0..n {
            oi.upsert(keys[i].clone(), vals[i]);
        }
        
        // Publish to build MPH
        oi.publish();
        
        // Now test get (this uses the SAME indexer)
        println!("\nTesting gets:");
        let guard = crossbeam_epoch::pin();
        let mut failures = 0;
        for i in 0..n {
            let result = oi.get(&keys[i], &guard);
            let expected = Some(i as u64);
            
            if result.copied() != expected {
                if failures < 5 {
                    println!("  ❌ key[{}]: expected {:?}, got {:?}", i, expected, result.copied());
                }
                failures += 1;
            } else if i < 3 {
                println!("  ✓ key[{}]: got {:?}", i, result.copied());
            }
        }
        
        if failures > 0 {
            println!("\n❌ FAILED: {} out of {} gets returned wrong values", failures, n);
            println!("This happens because the indexer used during construction is different");
            println!("from the one stored in the struct (due to AHasher randomization).");
            panic!("{} gets failed!", failures);
        }
        
        println!("\n✓ All {} gets successful", n);
    }
}

