//! Massive Graph - A Real-Time Graph Database for Collaborative Intelligence
//!
//! Massive Graph is a high-performance, real-time graph database designed for 
//! collaborative scenarios where multiple parties need to share and synchronize
//! data across trust boundaries with cryptographic guarantees.
#![warn(missing_docs)]

// Configure global allocator for maximum performance
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

// Core foundational modules
pub mod core;

/// Type definitions for all data structures
pub mod types;

// Main functional modules
pub mod storage;
pub mod delta;
pub mod api;
pub mod comms;
pub mod system;

// Re-export commonly used items
pub use core::Config;
pub use types::{Error, Result};

/// Crate version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Crate name
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Type alias for results using our custom error type
pub type MassiveResult<T> = std::result::Result<T, Error>;

/// Initialize the database system with tracing and metrics
pub fn init() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Initializing {} v{}", NAME, VERSION);
    
    // Initialize metrics registry
    system::metrics::init_registry();
    
    Ok(())
}

/// Global constants used throughout the codebase
pub mod constants;

// Re-export key types for external users
pub use types::{ID16, ID8, ID32, Value, Document};
pub use storage::{ZeroCopyStore, SimpleStore, DocumentStorage};
pub use constants::{BASE62_CHARS, CHUNK_SIZE}; 