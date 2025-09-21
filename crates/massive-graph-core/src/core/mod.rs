//! Core application components
//! 
//! This module contains core application state and configuration that works
//! across both server and browser environments.

/// Cross-platform application configuration
pub mod config;

/// Cross-platform application state
pub mod app_state;

/// Factory for creating AppState
pub mod factory;

/// Cross-platform utilities
pub mod utils;

/// Cross-platform logging
pub mod logging;

// Re-export commonly used items
pub use config::Config;
pub use app_state::AppState;
// pub use logging::{log_info, log_warn, log_error, log_debug, log_trace};
