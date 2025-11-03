use crossbeam_epoch as epoch;
use std::sync::Arc;
use massive_graph_core::structures::mph_delta_index::{OptimisedIndex, mph_indexer};
use massive_graph_core::types::ids::ID16;

struct ZeroMph;
impl mph_indexer::MphIndexer<ID16> for ZeroMph {
    fn eval(&self, _key: &ID16) -> usize { 0 }
    fn build(&self, _keys: &[ID16]) -> Arc<dyn mph_indexer::MphIndexer<ID16>> { 
        Arc::new(ZeroMph) 
    }
}

fn main() {
    println!("Testing epoch overflow with many small indexes...\n");
    
    let keys: Vec<ID16> = (0..4).map(|_| ID16::random()).collect();
    let vals: Vec<u64> = vec![1, 2, 3, 4];
    
    // Try to replicate the benchmark behavior
    for i in 0..1_000_000 {
        if i % 10000 == 0 {
            println!("Iteration {}", i);
            
            // Try to get epoch stats
            let guard = epoch::pin();
            guard.flush();
            drop(guard);
        }
        
        // Create index
        let idx = OptimisedIndex::new_with_indexer_and_capacity(
            Arc::new(ZeroMph),
            4,
            8
        );
        
        // Insert 4 items
        for j in 0..4 {
            idx.upsert(keys[j].clone(), vals[j]);
        }
        
        // Drop index (should defer cleanup)
        drop(idx);
    }
    
    println!("\nCompleted without crash!");
}

