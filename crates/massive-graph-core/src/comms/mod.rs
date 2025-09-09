//! Communication layer for Massive Graph
//! 
//! This module handles all network communication including WebRTC connections.

/// Connection management
pub mod connection_manager;

/// Protocol definitions for control channel
pub mod protocol;

/// WebRTC connection handling
pub mod network;

// Re-export commonly used items
pub use connection_manager::{ConnectionManager, ConnectionState, ConnectionStatus};
pub use protocol::{Command, Event};