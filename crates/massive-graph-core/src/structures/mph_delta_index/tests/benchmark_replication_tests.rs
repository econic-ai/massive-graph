//! Tests to replicate the benchmark failure and identify root cause
//! 
//! The benchmark fails with "memory allocation of 268435456 bytes failed" (256 MB)
//! during Criterion's warmup phase with n=64 keys.
//! 
//! Goal: Determine if the issue is:
//! 1. Overload: Many inserts to a single long-lived index
//! 2. Churn: Rapid creation/destruction of many index instances

use crate::structures::mph_delta_index::{OptimisedIndex, mph_indexer};
use crate::types::ID16;
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
struct V16([u8; 16]);

fn make_v16(i: usize) -> V16 {
    let mut b = [0u8; 16];
    b[0] = (i & 0xFF) as u8;
    b[15] = ((i >> 8) & 0xFF) as u8;
    V16(b)
}

/// Zero MPH indexer that always returns slot 0 (mimics benchmark)
#[derive(Clone)]
struct ZeroMph;
impl mph_indexer::MphIndexer<ID16> for ZeroMph {
    fn eval(&self, _key: &ID16) -> usize { 0 }
    fn build(_keys: &[ID16]) -> Self { ZeroMph }
}

/// Hypothesis 1: Overload - Many inserts to a single long-lived index
/// 
/// Test: Create ONE index and perform 100,000 insert operations on it
/// Expected: If this fails, the issue is with sustained operations on one instance
#[test]
#[ignore = "Run manually: cargo test hypothesis_overload -- --ignored --nocapture"]
fn hypothesis_overload_single_index_many_inserts() {
    eprintln!("\n=== Hypothesis 1: Overload (Single Index, Many Inserts) ===\n");
    
    let n = 64;
    let keys: Vec<ID16> = (0..n).map(|_| ID16::random()).collect();
    let vals: Vec<V16> = (0..n).map(make_v16).collect();
    
    eprintln!("Creating single OptimisedIndex with capacity {}", n);
    let idx = OptimisedIndex::new_with_indexer_and_capacity(
        ZeroMph,
        n,
        n * 2
    );
    
    eprintln!("Performing 100,000 insert operations on the SAME index...");
    
    let mut successful_inserts = 0;
    for i in 0..100_000 {
        // Insert all 64 keys
        for j in 0..n {
            idx.upsert(keys[j].clone(), vals[j]);
            successful_inserts += 1;
        }
        
        if i % 10_000 == 0 {
            eprintln!("[{}] Performed {} total inserts", i, successful_inserts);
        }
    }
    
    eprintln!("\n✅ Test completed successfully!");
    eprintln!("Total inserts performed: {}", successful_inserts);
    eprintln!("If this passed, overload is NOT the issue.");
    
    drop(idx);
    eprintln!("Index dropped.");
}

/// Hypothesis 2: Churn - Rapid creation/destruction of many index instances
/// 
/// Test: Create and destroy 100,000 index instances, each with n=64 keys
/// EXACTLY mimics benchmark's iter_batched pattern
/// Expected: If this fails, the issue is with rapid instance churn
#[test]
#[ignore = "Run manually: cargo test hypothesis_churn -- --ignored --nocapture"]
fn hypothesis_churn_many_instances() {
    eprintln!("\n=== Hypothesis 2: Churn (Many Index Instances) ===\n");
    eprintln!("EXACTLY mimicking benchmark's iter_batched pattern:\n");
    eprintln!("  - Setup: Create OptimisedIndex(ZeroMph, n=64, max=128)");
    eprintln!("  - Benchmark: Insert 64 keys with clone()");
    eprintln!("  - Teardown: Drop index");
    eprintln!("  - Repeat 100,000 times\n");
    
    let n = 64;
    let keys: Vec<ID16> = (0..n).map(|_| ID16::random()).collect();
    let vals: Vec<V16> = (0..n).map(make_v16).collect();
    
    for i in 0..100_000 {
        // SETUP phase (matches benchmark's iter_batched setup)
        let max_capacity = n * 2;
        let idx = OptimisedIndex::new_with_indexer_and_capacity(
            ZeroMph,
            n,
            max_capacity
        );
        
        // BENCHMARK phase (matches benchmark's measurement block)
        for j in 0..n {
            // NOTE: benchmark uses black_box() around upsert, but that's just
            // to prevent optimization, not affect behavior
            idx.upsert(keys[j].clone(), vals[j].clone());
        }
        
        // TEARDOWN phase (implicit drop)
        drop(idx);
        
        if i % 10_000 == 0 {
            eprintln!("[{}] {} iterations complete | {} total inserts", 
                     i, i + 1, (i + 1) * n);
        }
    }
    
    eprintln!("\n✅ Test completed successfully!");
    eprintln!("Total iterations: 100,000");
    eprintln!("Total inserts: {} ({} per iteration)", 100_000 * n, n);
    eprintln!("If this passed, churn at this scale is NOT the issue.");
}

/// Hypothesis 2b: Churn with aggressive epoch cleanup
/// 
/// Test: Same as hypothesis_churn but with epoch cleanup every 100 iterations
/// Expected: If this PASSES when hypothesis_churn FAILS, epoch accumulation is the issue
#[test]
#[ignore = "Run manually: cargo test hypothesis_churn_with_cleanup -- --ignored --nocapture"]
fn hypothesis_churn_with_aggressive_cleanup() {
    use crossbeam_epoch as epoch;
    
    eprintln!("\n=== Hypothesis 2b: Churn with Aggressive Epoch Cleanup ===\n");
    
    let n = 64;
    let keys: Vec<ID16> = (0..n).map(|_| ID16::random()).collect();
    let vals: Vec<V16> = (0..n).map(make_v16).collect();
    
    eprintln!("Creating and destroying 100,000 index instances...");
    eprintln!("With epoch cleanup every 100 iterations");
    
    for i in 0..100_000 {
        let idx = OptimisedIndex::new_with_indexer_and_capacity(
            ZeroMph,
            n,
            n * 2
        );
        
        for j in 0..n {
            idx.upsert(keys[j].clone(), vals[j]);
        }
        
        drop(idx);
        
        // Aggressive cleanup
        if i % 100 == 0 {
            epoch::pin().flush();
            std::thread::yield_now();
        }
        
        if i % 10_000 == 0 {
            eprintln!("[{}] Created/destroyed {} instances (with cleanup)", i, i + 1);
        }
    }
    
    eprintln!("\n✅ Test completed successfully!");
    eprintln!("If this passes but hypothesis_churn fails, epoch accumulation is the culprit.");
}

/// Hypothesis 2c: Churn - Mimicking Criterion's exact iteration count
/// 
/// Test: Run until we hit approximately the same iteration count as the benchmark
/// Criterion runs about 3 seconds of warmup where the failure occurs
/// At typical benchmark speed that's roughly 500,000 iterations
/// 
/// CRITICAL: This exactly mimics the benchmark's pattern:
/// - Each iteration: Create index + Insert 64 keys + Drop
/// - Total: 500K iterations = 32 million inserts
#[test]
#[ignore = "Run manually: cargo test hypothesis_criterion_scale -- --ignored --nocapture"]
fn hypothesis_criterion_scale() {
    eprintln!("\n=== Hypothesis 2c: Criterion-Scale Churn (500K iterations) ===\n");
    eprintln!("EXACTLY mimicking benchmark's iter_batched pattern at scale:");
    eprintln!("  - 500,000 iterations (matches Criterion warmup)");
    eprintln!("  - Each: Create index + Insert 64 keys + Drop");
    eprintln!("  - Total: {} inserts\n", 500_000 * 64);
    
    let n = 64;
    let keys: Vec<ID16> = (0..n).map(|_| ID16::random()).collect();
    let vals: Vec<V16> = (0..n).map(make_v16).collect();
    
    let start = std::time::Instant::now();
    
    for i in 0..500_000 {
        // SETUP: Create index (matches benchmark)
        let max_capacity = n * 2;
        let idx = OptimisedIndex::new_with_indexer_and_capacity(
            ZeroMph,
            n,
            max_capacity
        );
        
        // BENCHMARK: Insert n keys with clone (matches benchmark)
        for j in 0..n {
            idx.upsert(keys[j].clone(), vals[j].clone());
        }
        
        // TEARDOWN: Drop index (matches benchmark)
        drop(idx);
        
        if i % 50_000 == 0 {
            let elapsed = start.elapsed();
            let iter_per_sec = (i + 1) as f64 / elapsed.as_secs_f64();
            eprintln!("[{}] {} iterations | {:.0} iter/sec | {} total inserts", 
                     i, i + 1, iter_per_sec, (i + 1) * n);
        }
    }
    
    let elapsed = start.elapsed();
    eprintln!("\n✅ Test completed successfully!");
    eprintln!("Total time: {:.2}s", elapsed.as_secs_f64());
    eprintln!("Average: {:.0} iterations/sec", 500_000.0 / elapsed.as_secs_f64());
    eprintln!("Total inserts: {}", 500_000 * n);
    eprintln!("\n⚠️  If this passed, we did NOT replicate the benchmark failure!");
    eprintln!("    This suggests Criterion's measurement overhead is the trigger.");
}

