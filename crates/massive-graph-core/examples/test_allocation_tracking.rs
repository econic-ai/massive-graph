use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use massive_graph_core::structures::mph_delta_index::{OptimisedIndex, mph_indexer};
use massive_graph_core::types::ids::ID16;

// Custom allocator that tracks large allocations
struct TrackingAllocator;

static LARGE_ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static TOTAL_ALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        
        // Track large allocations
        if size > 10_000_000 { // > 10MB
            let count = LARGE_ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
            eprintln!("\n!!! LARGE ALLOCATION #{}: {} bytes ({} MB) at {:?}",
                     count + 1,
                     size,
                     size / (1024 * 1024),
                     std::backtrace::Backtrace::force_capture());
        }
        
        TOTAL_ALLOCATED.fetch_add(size, Ordering::Relaxed);
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        TOTAL_ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

struct ZeroMph;
impl mph_indexer::MphIndexer<ID16> for ZeroMph {
    fn eval(&self, _key: &ID16) -> usize { 0 }
    fn build(&self, _keys: &[ID16]) -> Arc<dyn mph_indexer::MphIndexer<ID16>> { 
        Arc::new(ZeroMph) 
    }
}

fn main() {
    println!("Testing with allocation tracking...\n");
    println!("Will print stack trace for any allocation > 10MB\n");
    
    let keys: Vec<ID16> = (0..4).map(|_| ID16::random()).collect();
    let vals: Vec<u64> = vec![1, 2, 3, 4];
    
    for i in 0..1_000_000 {
        if i % 10000 == 0 {
            let total = TOTAL_ALLOCATED.load(Ordering::Relaxed);
            let large = LARGE_ALLOC_COUNT.load(Ordering::Relaxed);
            println!("Iteration {}: Total allocated: {} MB, Large allocs: {}",
                    i,
                    total / (1024 * 1024),
                    large);
        }
        
        let idx = OptimisedIndex::new_with_indexer_and_capacity(
            Arc::new(ZeroMph),
            4,
            8
        );
        
        for j in 0..4 {
            idx.upsert(keys[j].clone(), vals[j]);
        }
        
        drop(idx);
    }
    
    println!("\nCompleted!");
}

