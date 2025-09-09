//! WebRTC abstractions for platform-agnostic peer connections
//! 
//! This module provides shared types and traits for WebRTC connections
//! that work across both native (server) and WASM (browser) environments.

mod connection;
mod payload;
mod signaling;

pub use connection::*;
pub use payload::*;
pub use signaling::*;
