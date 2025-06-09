//! Core system types and foundations
//! 
//! This module contains the fundamental building blocks of the Massive Graph system,
//! including type definitions, error handling, configuration, and memory management.

pub mod types;
pub mod error;
pub mod config;
pub mod allocator;

// Re-export commonly used items
pub use types::{NodeId, EdgeId, Value, PropertyKey, Timestamp};
pub use error::{Error, Result};
pub use config::Config; 