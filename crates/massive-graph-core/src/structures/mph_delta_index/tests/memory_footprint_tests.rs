/// Memory footprint test to identify 268MB allocation source
use crate::structures::mph_delta_index::{OptimisedIndex, mph_indexer};
use crate::types::ID16;
use std::sync::Arc;
use crossbeam_epoch::{self as epoch, Owned};
use std::sync::atomic::Ordering;

#[derive(Clone, Copy, Debug)]
struct V16([u8; 16]);

fn make_v16(i: usize) -> V16 {
    let mut b = [0u8; 16];
    b[0] = (i & 0xFF) as u8;
    b[15] = ((i >> 8) & 0xFF) as u8;
    V16(b)
}

#[derive(Clone)]
struct ZeroMph;
impl mph_indexer::MphIndexer<ID16> for ZeroMph {
    fn eval(&self, _key: &ID16) -> usize { 0 }
    fn build(_keys: &[ID16]) -> Self { ZeroMph }
}

#[test]
#[ignore] // Run manually with: cargo test memory_footprint_test -- --ignored --nocapture
fn test_memory_components() {
    let n = 64;
    let iterations = 100;
    
    eprintln!("\n=== Testing {} iterations with n={} ===\n", iterations, n);
    
    let keys: Vec<ID16> = (0..n).map(|_| ID16::random()).collect();
    let vals: Vec<V16> = (0..n).map(make_v16).collect();
    
    for iter in 0..iterations {
        let idx = OptimisedIndex::new_with_indexer_and_capacity(
            ZeroMph,
            n,
            n * 2
        );
        
        // Insert all keys
        for i in 0..n {
            idx.upsert(keys[i].clone(), vals[i]);
        }
        
        if iter % 10 == 0 {
            let guard = epoch::pin();
            let stats = idx.radix_stats(&guard);
            eprintln!(
                "[Iter {}] Arena: {} bytes, {} regions, {} large | Buckets: {}",
                iter,
                stats.hotpath_arena.total_bytes(),
                stats.hotpath_arena.num_regions,
                stats.hotpath_arena.num_large,
                stats.active_buckets
            );
        }
        
        // Explicit drop
        drop(idx);
        
        // Force epoch cleanup periodically
        if iter % 20 == 0 {
            epoch::pin().flush();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    
    eprintln!("\n=== Test completed successfully ===");
}

#[test]
#[ignore]
fn test_epoch_defer_accumulation() {
    eprintln!("\n=== Testing Epoch Defer Accumulation ===\n");
    
    // Simulate what happens with active list swaps
    let active: epoch::Atomic<Vec<u16>> = epoch::Atomic::null();
    
    // Simulate 10,000 active list swaps (similar to 10k bucket registrations)
    for i in 0..10000 {
        let guard = epoch::pin();
        let snap = active.load(Ordering::Acquire, &guard);
        let cur_slice: &[u16] = if snap.is_null() {
            &[]
        } else {
            unsafe { snap.deref().as_slice() }
        };
        
        let mut v: Vec<u16> = Vec::with_capacity(cur_slice.len() + 1);
        v.extend_from_slice(cur_slice);
        v.push((i % 64) as u16);
        
        let prev = active.swap(Owned::new(v), Ordering::AcqRel, &guard);
        if !prev.is_null() {
            unsafe { guard.defer_unchecked(move || drop(prev.into_owned())); }
        }
        
        if i % 1000 == 0 {
            eprintln!("[{}] Swapped vec, now {} items | {} defers accumulated", 
                     i, cur_slice.len() + 1, i);
        }
    }
    
    eprintln!("\n=== Epoch Defer Test Complete ===");
    eprintln!("Total defers queued: 10,000");
    eprintln!("Forcing cleanup...");
    
    epoch::pin().flush();
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    eprintln!("Cleanup complete");
}

#[test]
#[ignore]
fn test_large_n_single_iteration() {
    eprintln!("\n=== Testing Single Iteration with n=65536 ===\n");
    
    let n = 65536;
    let keys: Vec<ID16> = (0..n).map(|_| ID16::random()).collect();
    let vals: Vec<V16> = (0..n).map(make_v16).collect();
    
    let idx = OptimisedIndex::new_with_indexer_and_capacity(
        ZeroMph,
        n,
        n * 2
    );
    
    eprintln!("Inserting {} keys...", n);
    for i in 0..n {
        idx.upsert(keys[i].clone(), vals[i]);
        
        if i % 10000 == 0 && i > 0 {
            let guard = epoch::pin();
            let stats = idx.radix_stats(&guard);
            eprintln!(
                "[{} keys] Arena: {} bytes ({:.1} MB), {} regions | Active buckets: {}",
                i,
                stats.hotpath_arena.total_bytes(),
                stats.hotpath_arena.total_bytes() as f64 / (1024.0 * 1024.0),
                stats.hotpath_arena.num_regions,
                stats.active_buckets
            );
        }
    }
    
    let guard = epoch::pin();
    let stats = idx.radix_stats(&guard);
    eprintln!("\n=== Final Stats ===");
    eprintln!("{}", stats.summary_report());
    
    eprintln!("\nDropping index...");
    drop(idx);
    eprintln!("Index dropped successfully");
    
    eprintln!("\n=== Test completed successfully ===");
}
