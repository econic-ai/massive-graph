//! Delta processing module for real-time document synchronization.
//! 
//! This module provides immutable delta storage and sequential processing
//! with the following guarantees:
//! - Deltas are stored immutably in chunked heaps for zero-copy access
//! - Per-document sequential processing maintains operation ordering
//! - Work-stealing at document level provides load balancing
//! - Thread-safe concurrent access without blocking

pub mod types;
pub mod processor;

// Re-export main types for convenience
pub use types::{DeltaHeader, DeltaStatus, ChunkAddress};
pub use processor::DeltaProcessor; 