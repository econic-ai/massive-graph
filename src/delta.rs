//! Delta processing module for real-time document synchronization.
//! 
//! This module provides immutable delta storage and sequential processing
//! with the following guarantees:
//! - Deltas are stored immutably in chunked heaps for zero-copy access
//! - Per-document sequential processing maintains operation ordering
//! - Work-stealing at document level provides load balancing
//! - Thread-safe concurrent access without blocking

// Types moved to top-level types module
/// Delta operation processor
pub mod delta_processor;

// Re-export main types for convenience
// pub use crate::types::{DeltaPacket, DeltaStatus, ValidationResult};

// Re-export delta processor
// TODO: Fix delta processor for POC
// pub use delta_processor::apply_delta;