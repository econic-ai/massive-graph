//! Cross-platform logging utilities
//! 
//! This module provides zero-overhead logging macros that work across both
//! native and WASM targets. On native targets, it uses the tracing crate.
//! On WASM targets, it uses the browser's console API.

/// Cross-platform logging module with clean API
pub mod logging {

    /// Info level logging - general information messages
    #[macro_export]
    macro_rules! log_info {
        ($($arg:tt)*) => {{
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(&format!($($arg)*).into());
            
            #[cfg(not(target_arch = "wasm32"))]
            tracing::info!($($arg)*);
        }};
    }

    /// Warning level logging - potentially problematic situations
    #[macro_export]
    macro_rules! log_warn {
        ($($arg:tt)*) => {{
            #[cfg(target_arch = "wasm32")]
            web_sys::console::warn_1(&format!($($arg)*).into());
            
            #[cfg(not(target_arch = "wasm32"))]
            tracing::warn!($($arg)*);
        }};
    }

    /// Error level logging - error conditions
    #[macro_export]
    macro_rules! log_error {
        ($($arg:tt)*) => {{
            #[cfg(target_arch = "wasm32")]
            web_sys::console::error_1(&format!($($arg)*).into());
            
            #[cfg(not(target_arch = "wasm32"))]
            tracing::error!($($arg)*);
        }};
    }

    /// Debug level logging - detailed information for debugging
    #[macro_export]
    macro_rules! log_debug {
        ($($arg:tt)*) => {{
            #[cfg(target_arch = "wasm32")]
            web_sys::console::debug_1(&format!($($arg)*).into());
            
            #[cfg(not(target_arch = "wasm32"))]
            tracing::debug!($($arg)*);
        }};
    }

    /// Trace level logging - very detailed tracing information
    #[macro_export]
    macro_rules! log_trace {
        ($($arg:tt)*) => {{
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(&format!("TRACE: {}", format!($($arg)*)).into());
            
            #[cfg(not(target_arch = "wasm32"))]
            tracing::trace!($($arg)*);
        }};
    }

    // Make macros available with clean names
    // pub(crate) use log_info;
    // pub(crate) use log_warn;
    // pub(crate) use log_error;
    // pub(crate) use log_debug;
    // pub(crate) use log_trace;
}