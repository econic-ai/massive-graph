//! Comprehensive unit tests for BBHashIndexer
//! Tests determinism, correctness, and performance across various key types and scales.

#[cfg(test)]
mod tests {
    use crate::structures::mph_delta_index::mph_indexer::{BBHashIndexer, MphIndexer, BbhConfig};
    use crate::types::{ID8, ID16, ID32};
    use std::collections::HashSet;

    /// Test scales: 10, 100, 1K, 10K, 100K, 1M
    const TEST_SCALES: &[usize] = &[10, 100, 1_000, 10_000, 100_000, 1_000_000];

    // Helper: Generate sequential usize keys
    fn make_sequential_keys(n: usize) -> Vec<usize> {
        (0..n).collect()
    }

    // Helper: Generate ID8 keys (8-byte random-like values)
    fn make_id8_keys(n: usize) -> Vec<ID8> {
        (0..n).map(|i| {
            let mut bytes = [0u8; 8];
            // Use a deterministic "random" pattern based on index
            let mut x = i as u64;
            x = x.wrapping_mul(0x9E3779B97F4A7C15u64);
            x ^= x >> 30;
            x = x.wrapping_mul(0xBF58476D1CE4E5B9u64);
            x ^= x >> 27;
            bytes.copy_from_slice(&x.to_le_bytes());
            ID8::new(bytes)
        }).collect()
    }

    // Helper: Generate ID16 keys (16-byte random-like values)
    fn make_id16_keys(n: usize) -> Vec<ID16> {
        (0..n).map(|i| {
            let mut bytes = [0u8; 16];
            // Use a deterministic "random" pattern based on index
            let mut x = i as u64;
            x = x.wrapping_mul(0x9E3779B97F4A7C15u64);
            x ^= x >> 30;
            x = x.wrapping_mul(0xBF58476D1CE4E5B9u64);
            x ^= x >> 27;
            bytes[0..8].copy_from_slice(&x.to_le_bytes());
            
            let mut y = x.wrapping_add(0xDEADBEEFCAFEBABEu64);
            y = y.wrapping_mul(0x9E3779B97F4A7C15u64);
            y ^= y >> 30;
            bytes[8..16].copy_from_slice(&y.to_le_bytes());
            ID16::from_bytes(bytes)
        }).collect()
    }

    // Helper: Generate ID32 keys (32-byte random-like values)
    fn make_id32_keys(n: usize) -> Vec<ID32> {
        (0..n).map(|i| {
            let mut bytes = [0u8; 32];
            // Use a deterministic "random" pattern based on index
            for chunk in 0..4 {
                let mut x = (i as u64).wrapping_add((chunk as u64) * 0x123456789ABCDEFu64);
                x = x.wrapping_mul(0x9E3779B97F4A7C15u64);
                x ^= x >> 30;
                x = x.wrapping_mul(0xBF58476D1CE4E5B9u64);
                x ^= x >> 27;
                bytes[chunk*8..(chunk+1)*8].copy_from_slice(&x.to_le_bytes());
            }
            ID32::new(bytes)
        }).collect()
    }

    /// Test 1: Determinism - same keys produce same indexer with same eval results
    #[test]
    fn test_determinism_sequential() {
        for &n in &[10, 100, 1_000] {
            let keys = make_sequential_keys(n);
            
            // Build two indexers from the same keys
            let mph1 = BBHashIndexer::build(&keys, Default::default());
            let mph2 = BBHashIndexer::build(&keys, Default::default());
            
            // Verify all keys map to the same slots in both indexers
            for (i, key) in keys.iter().enumerate() {
                let slot1 = mph1.eval(key);
                let slot2 = mph2.eval(key);
                assert_eq!(slot1, slot2, 
                    "Non-deterministic: key[{}]={} maps to slot {} in mph1 but {} in mph2", 
                    i, key, slot1, slot2);
            }
        }
    }

    #[test]
    fn test_determinism_id16() {
        for &n in &[10, 100, 1_000] {
            let keys = make_id16_keys(n);
            
            let mph1 = BBHashIndexer::build(&keys, Default::default());
            let mph2 = BBHashIndexer::build(&keys, Default::default());
            
            for (i, key) in keys.iter().enumerate() {
                let slot1 = mph1.eval(key);
                let slot2 = mph2.eval(key);
                assert_eq!(slot1, slot2, 
                    "Non-deterministic: ID16 key[{}] maps to slot {} in mph1 but {} in mph2", 
                    i, slot1, slot2);
            }
        }
    }

    /// Test 2: Injectivity - all keys map to unique slots
    #[test]
    fn test_injectivity_sequential() {
        for &n in &[10, 100, 1_000, 10_000] {
            let keys = make_sequential_keys(n);
            let mph = BBHashIndexer::build(&keys, Default::default());
            
            let mut seen_slots = HashSet::new();
            for (i, key) in keys.iter().enumerate() {
                let slot = mph.eval(key);
                assert!(slot < n, 
                    "Out of bounds: key[{}]={} maps to slot {} (n={})", 
                    i, key, slot, n);
                assert!(seen_slots.insert(slot), 
                    "Collision: key[{}]={} maps to slot {} which was already assigned", 
                    i, key, slot);
            }
            
            assert_eq!(seen_slots.len(), n, 
                "Not all slots used: {} unique slots for {} keys", 
                seen_slots.len(), n);
        }
    }

    #[test]
    fn test_injectivity_id8() {
        for &n in &[10, 100, 1_000, 10_000] {
            let keys = make_id8_keys(n);
            let mph = BBHashIndexer::build(&keys, Default::default());
            
            let mut seen_slots = HashSet::new();
            for (i, key) in keys.iter().enumerate() {
                let slot = mph.eval(key);
                assert!(slot < n, 
                    "Out of bounds: ID8 key[{}] maps to slot {} (n={})", 
                    i, slot, n);
                assert!(seen_slots.insert(slot), 
                    "Collision: ID8 key[{}] maps to slot {} which was already assigned", 
                    i, slot);
            }
            
            assert_eq!(seen_slots.len(), n);
        }
    }

    #[test]
    fn test_injectivity_id16() {
        for &n in &[10, 100, 1_000, 10_000] {
            let keys = make_id16_keys(n);
            let mph = BBHashIndexer::build(&keys, Default::default());
            
            let mut seen_slots = HashSet::new();
            for (i, key) in keys.iter().enumerate() {
                let slot = mph.eval(key);
                assert!(slot < n, 
                    "Out of bounds: ID16 key[{}] maps to slot {} (n={})", 
                    i, slot, n);
                assert!(seen_slots.insert(slot), 
                    "Collision: ID16 key[{}] maps to slot {} which was already assigned", 
                    i, slot);
            }
            
            assert_eq!(seen_slots.len(), n);
        }
    }

    #[test]
    fn test_injectivity_id32() {
        for &n in &[10, 100, 1_000, 10_000] {
            let keys = make_id32_keys(n);
            let mph = BBHashIndexer::build(&keys, Default::default());
            
            let mut seen_slots = HashSet::new();
            for (i, key) in keys.iter().enumerate() {
                let slot = mph.eval(key);
                assert!(slot < n, 
                    "Out of bounds: ID32 key[{}] maps to slot {} (n={})", 
                    i, slot, n);
                assert!(seen_slots.insert(slot), 
                    "Collision: ID32 key[{}] maps to slot {} which was already assigned", 
                    i, slot);
            }
            
            assert_eq!(seen_slots.len(), n);
        }
    }

    /// Test 3: Stability - eval returns same slot across multiple calls
    #[test]
    fn test_stability_sequential() {
        for &n in &[10, 100, 1_000] {
            let keys = make_sequential_keys(n);
            let mph = BBHashIndexer::build(&keys, Default::default());
            
            for (i, key) in keys.iter().enumerate() {
                let slot1 = mph.eval(key);
                let slot2 = mph.eval(key);
                let slot3 = mph.eval(key);
                assert_eq!(slot1, slot2, 
                    "Unstable: key[{}]={} returned slot {} then {}", 
                    i, key, slot1, slot2);
                assert_eq!(slot2, slot3, 
                    "Unstable: key[{}]={} returned slot {} then {}", 
                    i, key, slot2, slot3);
            }
        }
    }

    #[test]
    fn test_stability_id16() {
        for &n in &[10, 100, 1_000] {
            let keys = make_id16_keys(n);
            let mph = BBHashIndexer::build(&keys, Default::default());
            
            for (i, key) in keys.iter().enumerate() {
                let slot1 = mph.eval(key);
                let slot2 = mph.eval(key);
                let slot3 = mph.eval(key);
                assert_eq!(slot1, slot2, 
                    "Unstable: ID16 key[{}] returned slot {} then {}", 
                    i, slot1, slot2);
                assert_eq!(slot2, slot3, 
                    "Unstable: ID16 key[{}] returned slot {} then {}", 
                    i, slot2, slot3);
            }
        }
    }

    /// Test 4: Non-member handling - keys not in build set map deterministically
    #[test]
    fn test_non_member_determinism() {
        let build_keys = make_sequential_keys(100);
        let mph = BBHashIndexer::build(&build_keys, Default::default());
        
        // Test non-member keys (1000..1100)
        let non_member_keys: Vec<usize> = (1000..1100).collect();
        
        for key in &non_member_keys {
            let slot1 = mph.eval(key);
            let slot2 = mph.eval(key);
            assert_eq!(slot1, slot2, 
                "Non-member key {} maps to different slots: {} vs {}", 
                key, slot1, slot2);
            assert!(slot1 < 100, 
                "Non-member key {} maps to out-of-bounds slot {}", 
                key, slot1);
        }
    }

    /// Test 5: Large scale tests (100K, 1M)
    #[test]
    #[ignore] // Run with --ignored for full test suite
    fn test_large_scale_sequential() {
        for &n in &[100_000, 1_000_000] {
            println!("Testing sequential keys at scale n={}", n);
            let keys = make_sequential_keys(n);
            let mph = BBHashIndexer::build(&keys, Default::default());
            
            // Sample test: check first 1000, middle 1000, last 1000
            let samples = [
                (0, 1000),
                (n/2 - 500, n/2 + 500),
                (n - 1000, n),
            ];
            
            for (start, end) in samples {
                let mut seen_slots = HashSet::new();
                for i in start..end {
                    let slot = mph.eval(&keys[i]);
                    assert!(slot < n, "Out of bounds at scale {}: slot {}", n, slot);
                    assert!(seen_slots.insert(slot), 
                        "Collision at scale {}: key[{}] maps to slot {} (already used)", 
                        n, i, slot);
                }
            }
            println!("✓ Scale n={} passed", n);
        }
    }

    #[test]
    #[ignore] // Run with --ignored for full test suite
    fn test_large_scale_id16() {
        for &n in &[100_000, 1_000_000] {
            println!("Testing ID16 keys at scale n={}", n);
            let keys = make_id16_keys(n);
            let mph = BBHashIndexer::build(&keys, Default::default());
            
            // Sample test: check first 1000, middle 1000, last 1000
            let samples = [
                (0, 1000),
                (n/2 - 500, n/2 + 500),
                (n - 1000, n),
            ];
            
            for (start, end) in samples {
                let mut seen_slots = HashSet::new();
                for i in start..end {
                    let slot = mph.eval(&keys[i]);
                    assert!(slot < n, "Out of bounds at scale {}: slot {}", n, slot);
                    assert!(seen_slots.insert(slot), 
                        "Collision at scale {}: ID16 key[{}] maps to slot {} (already used)", 
                        n, i, slot);
                }
            }
            println!("✓ Scale n={} passed", n);
        }
    }

    /// Test 6: Config variations
    #[test]
    fn test_config_variations() {
        let keys = make_sequential_keys(1000);
        
        // Test different gamma values
        for gamma in [1.1, 1.3, 1.5, 2.0] {
            let cfg = BbhConfig { gamma, max_levels: 32 };
            let mph = BBHashIndexer::build(&keys, cfg);
            
            let mut seen_slots = HashSet::new();
            for (i, key) in keys.iter().enumerate() {
                let slot = mph.eval(key);
                assert!(slot < 1000, "Out of bounds with gamma={}: slot {}", gamma, slot);
                assert!(seen_slots.insert(slot), 
                    "Collision with gamma={}: key[{}] maps to slot {}", 
                    gamma, i, slot);
            }
        }
    }

    /// Test 7: Edge cases
    #[test]
    fn test_single_key() {
        let keys = vec![42usize];
        let mph = BBHashIndexer::build(&keys, Default::default());
        
        assert_eq!(mph.eval(&42), 0, "Single key should map to slot 0");
        
        // Non-member should also map to slot 0 (only slot available)
        let non_member_slot = mph.eval(&999);
        assert_eq!(non_member_slot, 0, "Non-member with n=1 should map to slot 0");
    }

    #[test]
    fn test_two_keys() {
        let keys = vec![10usize, 20usize];
        let mph = BBHashIndexer::build(&keys, Default::default());
        
        let slot_10 = mph.eval(&10);
        let slot_20 = mph.eval(&20);
        
        assert!(slot_10 < 2, "Key 10 out of bounds");
        assert!(slot_20 < 2, "Key 20 out of bounds");
        assert_ne!(slot_10, slot_20, "Keys 10 and 20 collide");
    }

    /// Test 8: Repeated eval consistency
    #[test]
    fn test_repeated_eval_consistency() {
        let keys = make_id16_keys(100);
        let mph = BBHashIndexer::build(&keys, Default::default());
        
        // Call eval 100 times for each key and verify consistency
        for (i, key) in keys.iter().enumerate() {
            let first_slot = mph.eval(key);
            for _ in 0..100 {
                let slot = mph.eval(key);
                assert_eq!(slot, first_slot, 
                    "Inconsistent eval for key[{}]: expected {}, got {}", 
                    i, first_slot, slot);
            }
        }
    }

    /// Test 9: Build and eval in different order
    #[test]
    fn test_eval_order_independence() {
        let keys = make_sequential_keys(100);
        let mph = BBHashIndexer::build(&keys, Default::default());
        
        // Eval in forward order
        let forward_slots: Vec<usize> = keys.iter().map(|k| mph.eval(k)).collect();
        
        // Eval in reverse order
        let reverse_slots: Vec<usize> = keys.iter().rev().map(|k| mph.eval(k)).collect();
        
        // Verify forward and reverse produce the same mapping
        for (i, (&fwd, &rev)) in forward_slots.iter().zip(reverse_slots.iter().rev()).enumerate() {
            assert_eq!(fwd, rev, 
                "Order-dependent: key[{}] maps to {} in forward, {} in reverse", 
                i, fwd, rev);
        }
    }

    /// Test 10: Verify perfect hash property - bijection
    #[test]
    fn test_perfect_hash_bijection() {
        for &n in &[10, 100, 1_000] {
            let keys = make_id16_keys(n);
            let mph = BBHashIndexer::build(&keys, Default::default());
            
            // Build forward map: key_index -> slot
            let mut key_to_slot: Vec<usize> = Vec::with_capacity(n);
            for key in &keys {
                key_to_slot.push(mph.eval(key));
            }
            
            // Verify bijection: each slot appears exactly once
            let mut slot_counts = vec![0usize; n];
            for &slot in &key_to_slot {
                assert!(slot < n, "Slot {} out of bounds (n={})", slot, n);
                slot_counts[slot] += 1;
            }
            
            for (slot, &count) in slot_counts.iter().enumerate() {
                assert_eq!(count, 1, 
                    "Slot {} used {} times (expected 1) at n={}", 
                    slot, count, n);
            }
        }
    }
}

