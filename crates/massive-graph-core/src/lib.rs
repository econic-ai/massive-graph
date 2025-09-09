//! # Massive Graph Core
//! 
//! Core types and abstractions for the Massive Graph database.
//! This crate is designed to be WASM-compatible and contains minimal dependencies.

#![warn(missing_docs)]

/// Comms layer for realtime communication
pub mod comms;

/// Core application components
pub mod core;

/// System utilities and metrics
pub mod system;
    
/// Type definitions for all data structures
pub mod types;

/// System constants
pub mod constants;

/// Storage layer for document operations
pub mod storage;

/// Delta processing and propagation
pub mod delta;

/// WebRTC connection abstractions
pub mod webrtc;

// Re-export commonly used items
pub use types::{DocId, UserId, VersionId, StreamId, DeltaId, ConnectionId};
// pub use types::{Document, DocumentType};
pub use core::{AppState, Config};

// Storage re-exports
pub use storage::DocumentStorage;
