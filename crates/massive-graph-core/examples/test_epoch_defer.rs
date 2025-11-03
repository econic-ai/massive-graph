/// Standalone test: Epoch defer accumulation
use crossbeam_epoch::{self as epoch, Owned};
use std::sync::atomic::Ordering;

fn main() {
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

