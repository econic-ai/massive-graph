//! Core system types and foundations
//! 
//! This module contains the fundamental building blocks of the Massive Graph system,
//! including error handling, configuration, and memory management.

// Error types moved to types module
/// Configuration management
pub mod config;
/// Application state management
pub mod app_state;
/// Application factory
pub mod factory;
// IDs moved to types module
// Types moved to top-level types module
/// Utility functions for common operations
pub mod utils;

// Re-export commonly used items
pub use config::Config;
pub use app_state::AppState;
pub use factory::{create_app_state, ConfiguredAppState, AppStateFactoryError};
