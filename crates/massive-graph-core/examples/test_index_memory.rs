/// Standalone test: Full OptimisedIndex memory footprint
use massive_graph_core::structures::mph_delta_index::{OptimisedIndex, mph_indexer};
use massive_graph_core::types::ID16;
use std::sync::Arc;
use crossbeam_epoch as epoch;

#[derive(Clone, Copy, Debug)]
struct V16([u8; 16]);

fn make_v16(i: usize) -> V16 {
    let mut b = [0u8; 16];
    b[0] = (i & 0xFF) as u8;
    b[15] = ((i >> 8) & 0xFF) as u8;
    V16(b)
}

struct ZeroMph;
impl mph_indexer::MphIndexer<ID16> for ZeroMph {
    fn eval(&self, _key: &ID16) -> usize { 0 }
    fn build(&self, _keys: &[ID16]) -> Arc<dyn mph_indexer::MphIndexer<ID16>> { 
        Arc::new(ZeroMph) 
    }
}

fn main() {
    let n = 64;
    let iterations = 100;
    
    eprintln!("\n=== Testing {} iterations with n={} ===\n", iterations, n);
    
    let keys: Vec<ID16> = (0..n).map(|_| ID16::random()).collect();
    let vals: Vec<V16> = (0..n).map(make_v16).collect();
    
    for iter in 0..iterations {
        let idx = OptimisedIndex::new_with_indexer_and_capacity(
            Arc::new(ZeroMph),
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

