//! # Massive Graph Server
//! 
//! Native server implementation for the Massive Graph database.
//! This crate contains HTTP/WebSocket APIs and native-only optimizations.

#![warn(missing_docs)]

/// HTTP API handlers and routing
pub mod api;

/// Application constants
pub mod constants;

/// WebRTC implementation
pub mod webrtc;

/// QUIC ingress service
pub mod quic;

// Re-export core functionality
pub use massive_graph_core::*;
/// Backwards-compat re-export: expose core storage as `crate::storage`
pub use massive_graph_core::storage as storage;
