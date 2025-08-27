//! # Massive Graph Core
//! 
//! Core types and abstractions for the Massive Graph database.
//! This crate is designed to be WASM-compatible and contains minimal dependencies.

#![warn(missing_docs)]

/// System utilities and metrics
pub mod system;

/// Type definitions for all data structures
pub mod types;

/// System constants
pub mod constants;

/// Storage layer for document operations (native only)
#[cfg(not(target_arch = "wasm32"))]
pub mod storage;

// Re-export commonly used items
pub use types::{DocId, UserId, VersionId, StreamId, DeltaId};
pub use types::{Document, DocumentType};

// Storage re-exports (native only)
#[cfg(not(target_arch = "wasm32"))]
pub use storage::DocumentStorage;
