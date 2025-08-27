//! System utilities and metrics

/// Metrics collection
pub mod metrics;

/// Performance profiling  
pub mod profiling;

/// System utilities
pub mod utils;

// Re-export system components
// Re-export system modules when needed
// pub use metrics::*;
// pub use profiling::*;
pub use utils::*;

