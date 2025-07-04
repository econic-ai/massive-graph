//! Core system types and foundations
//! 
//! This module contains the fundamental building blocks of the Massive Graph system,
//! including error handling, configuration, and memory management.

/// Error types and result handling
pub mod error;
/// Configuration management
pub mod config;
/// Core data types including IDs, documents, and deltas
pub mod types;
/// Storage-agnostic delta operation processor
pub mod delta_processor;

// Re-export commonly used items
pub use error::{Error, Result};
pub use config::Config;

// Re-export core types
pub use types::{ID16, ID8};
pub use types::document::{Value, Document, AdaptiveMap};

// Re-export delta processor
pub use delta_processor::apply_delta;