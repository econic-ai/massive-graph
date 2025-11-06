//! Debug logging macros for OptimisedIndex
//! 
//! These macros provide flexible debug logging controlled via environment variables
//! with zero-overhead when disabled. When the `debug-logging` feature is disabled,
//! all logging compiles to nothing.
//!
//! # Usage
//!
//! Enable logging by setting environment variables before running:
//!
//! ```bash
//! # Enable all debug logging
//! MG_DEBUG=1 cargo run
//!
//! # Enable logging for specific modules
//! MG_DEBUG=mph_delta_index cargo run
//!
//! # Enable logging for specific functions
//! MG_DEBUG=mph_delta_index::upsert cargo run
//!
//! # Enable all index-related logging
//! MG_DEBUG_INDEX=1 cargo run
//! ```
//!
//! Module paths use `::` separator and can match prefixes:
//! - `MG_DEBUG=mph_delta_index` - enables all logging in mph_delta_index module
//! - `MG_DEBUG=index` - enables all logging in any module containing "index"
//! - `MG_DEBUG=1` or `MG_DEBUG=all` - enables all debug logging
//!
//! You can also use the `debug-logging` feature flag for global enable/disable at compile time.

/// Runtime configuration for debug logging based on environment variables
#[cfg(feature = "debug-logging")]
pub mod config {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::OnceLock;

    /// Runtime debug configuration
    pub struct DebugConfig {
        /// Whether debug logging is globally enabled
        enabled: AtomicBool,
        /// Module path patterns to match against
        patterns: Vec<String>,
    }

    static CONFIG: OnceLock<DebugConfig> = OnceLock::new();

    impl DebugConfig {
        fn load() -> &'static Self {
            CONFIG.get_or_init(|| {
                let enabled = std::env::var("MG_DEBUG")
                    .map(|v| v == "1" || v == "all" || v.to_lowercase() == "true")
                    .unwrap_or(false);
                
                let enabled_index = std::env::var("MG_DEBUG_INDEX")
                    .map(|v| v == "1" || v.to_lowercase() == "true")
                    .unwrap_or(false);

                let mut patterns = Vec::new();
                
                // Parse MG_DEBUG for module patterns
                if let Ok(val) = std::env::var("MG_DEBUG") {
                    if val != "1" && val != "all" && val.to_lowercase() != "true" {
                        patterns.push(val);
                    }
                }
                
                // If MG_DEBUG_INDEX is set, add index-related patterns
                if enabled_index {
                    patterns.push("mph_delta_index".to_string());
                    patterns.push("optimised_index".to_string());
                }

                DebugConfig {
                    enabled: AtomicBool::new(enabled || enabled_index),
                    patterns,
                }
            })
        }

        /// Check if debug logging should be enabled for the given module path
        pub fn is_enabled(module_path: &str) -> bool {
            let config = Self::load();
            
            // Global enable
            if config.enabled.load(Ordering::Relaxed) {
                return true;
            }
            
            // Check against patterns
            for pattern in &config.patterns {
                if module_path.contains(pattern) {
                    return true;
                }
            }
            
            false
        }
    }

    /// Check if debug logging is enabled for the given module path
    pub fn is_debug_enabled(module_path: &str) -> bool {
        DebugConfig::is_enabled(module_path)
    }
}

/// Stub configuration module when debug-logging feature is disabled
#[cfg(not(feature = "debug-logging"))]
pub mod config {
    /// Stub function for when debug-logging feature is disabled
    #[allow(dead_code)] // Used by macros
    pub fn is_debug_enabled(_module_path: &str) -> bool {
        false
    }
}

/// Debug print macro controlled by environment variables and module path.
/// 
/// Usage:
/// ```ignore
/// debug_log!("upsert: key={:?}, idx={}", key, idx);
/// debug_log!(module = "mph_delta_index", "key={:?}", key);
/// ```
#[macro_export]
macro_rules! debug_log {
    // With explicit module path
    (module = $module:expr, $($arg:tt)*) => {
        #[cfg(feature = "debug-logging")]
        {
            if $crate::structures::mph_delta_index::debug_macros::config::is_debug_enabled($module) {
                eprintln!($($arg)*);
            }
        }
    };
    // Default: use module_path!() and file!() to get context
    ($($arg:tt)*) => {
        #[cfg(feature = "debug-logging")]
        {
            let module_path = concat!(module_path!(), "::", file!());
            if $crate::structures::mph_delta_index::debug_macros::config::is_debug_enabled(module_path) {
                eprintln!($($arg)*);
            }
        }
    };
}

/// Debug print macro with a label prefix.
/// 
/// Usage:
/// ```ignore
/// debug_log_labeled!("UPSERT", "key={:?}, idx={}", key, idx);
/// debug_log_labeled!(module = "mph_delta_index", "UPSERT", "key={:?}", key, idx);
/// // Output: [UPSERT] key=..., idx=...
/// ```
#[macro_export]
macro_rules! debug_log_labeled {
    // With explicit module path
    (module = $module:expr, $label:expr, $($arg:tt)*) => {
        #[cfg(feature = "debug-logging")]
        {
            if $crate::structures::mph_delta_index::debug_macros::config::is_debug_enabled($module) {
                eprint!("[{}] ", $label);
                eprintln!($($arg)*);
            }
        }
    };
    // Without explicit module path
    ($label:expr, $($arg:tt)*) => {
        #[cfg(feature = "debug-logging")]
        {
            let module_path = concat!(module_path!(), "::", file!());
            if $crate::structures::mph_delta_index::debug_macros::config::is_debug_enabled(module_path) {
                eprint!("[{}] ", $label);
                eprintln!($($arg)*);
            }
        }
    };
}

/// Debug expression evaluation macro - evaluates expression only when debugging is enabled.
/// Useful for expensive computations that are only needed for logging.
/// 
/// Usage:
/// ```ignore
/// let expensive_value = debug_eval!(module = "mph_delta_index", compute_expensive_thing());
/// let expensive_value = debug_eval!(compute_expensive_thing());
/// debug_log!("value={:?}", expensive_value);
/// ```
#[macro_export]
macro_rules! debug_eval {
    // With explicit module path
    (module = $module:expr, $expr:expr) => {
        #[cfg(feature = "debug-logging")]
        {
            if $crate::structures::mph_delta_index::debug_macros::config::is_debug_enabled($module) {
                $expr
            } else {
                ()
            }
        }
        #[cfg(not(feature = "debug-logging"))]
        {
            ()
        }
    };
    // Without explicit module path
    ($expr:expr) => {
        #[cfg(feature = "debug-logging")]
        {
            let module_path = concat!(module_path!(), "::", file!());
            if $crate::structures::mph_delta_index::debug_macros::config::is_debug_enabled(module_path) {
                $expr
            } else {
                ()
            }
        }
        #[cfg(not(feature = "debug-logging"))]
        {
            ()
        }
    };
}

/// Debug block macro - entire block only executes when debugging is enabled.
/// 
/// Usage:
/// ```ignore
/// debug_block!(module = "mph_delta_index") {
///     let x = expensive_computation();
///     let y = another_expensive_thing();
///     eprintln!("x={}, y={}", x, y);
/// }
/// ```
#[macro_export]
macro_rules! debug_block {
    // With explicit module path
    (module = $module:expr, $($body:tt)*) => {
        #[cfg(feature = "debug-logging")]
        {
            if $crate::structures::mph_delta_index::debug_macros::config::is_debug_enabled($module) {
                $($body)*
            }
        }
    };
    // Without explicit module path
    ($($body:tt)*) => {
        #[cfg(feature = "debug-logging")]
        {
            let module_path = concat!(module_path!(), "::", file!());
            if $crate::structures::mph_delta_index::debug_macros::config::is_debug_enabled(module_path) {
                $($body)*
            }
        }
    };
}

