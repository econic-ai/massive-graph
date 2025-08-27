//! Core application logic and configuration

/// Application configuration
pub mod config;

/// Application state management
pub mod app_state;

/// Factory pattern for app creation
pub mod factory;

/// Multithreading utilities
pub mod multithreading;

/// Core utilities
pub mod utils;

// Re-export commonly used items
pub use config::{Config, load_config_or_default};
pub use app_state::AppState;
pub use factory::{ConfiguredAppState, create_app_state};

