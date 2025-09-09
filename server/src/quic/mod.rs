//! QUIC ingress service for high-performance delta streaming
//! 
//! This module implements a QUIC-based ingress service with:
//! - Single-copy from stream to storage
//! - Doc-id level sharding
//! - Lock-free reads with minimal atomics
//! - K=12 unidirectional swim lanes per connection

mod config;
mod server;
mod connection_manager;
mod shard_runtime;
mod types;

pub use config::QuicConfig;
pub use server::QuicService;
pub use types::{ShardId, LaneId};

// Re-export for main.rs integration
pub use server::run_quic_service;
