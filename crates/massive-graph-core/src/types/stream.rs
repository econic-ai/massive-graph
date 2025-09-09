// use std::sync::atomic::{AtomicBool, AtomicPtr};
// use super::StreamId;

// /// In-memory reference to immutable delta
// /// Used for building traversable chains
// pub struct DeltaRef {
//     pub ptr: *const WireDelta,  // Pointer to immutable delta
//     pub delta_id: ID8,           // Cached for fast ordering
//     pub timestamp: u64,          // Cached for time-based queries
//     pub next: AtomicPtr<DeltaRef>, // Next in chain (lock-free)
// }

// /// Lock-free append-only stream for deltas
// pub struct AppendOnlyDeltaStream {
//     pub head: *const DeltaRef,           // First delta (immutable after creation)
//     pub tail: AtomicPtr<DeltaRef>,      // Last delta for O(1) append
// }

// impl AppendOnlyDeltaStream {
//     /// Create new stream with initial delta
//     pub fn new(first: *const DeltaRef) -> Self {
//         Self {
//             head: first,
//             tail: AtomicPtr::new(first as *mut DeltaRef),
//         }
//     }
    
//     /// Append delta to stream atomically
//     pub fn append(&self, delta_ref: *mut DeltaRef) {
//         // Lock-free append via tail CAS
//     }
// }