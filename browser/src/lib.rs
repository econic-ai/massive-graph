//! # Massive Graph WASM
//!
//! Minimal WebAssembly bindings for the Massive Graph database (hello world only).

use wasm_bindgen::prelude::*;
use massive_graph_core as mgcore;

/// Utility function to log to browser console
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// Macro for console logging
macro_rules! console_log {
    ($($t:tt)*) => (crate::log(&format_args!($($t)*).to_string()))
}

// Initialize WASM environment
#[wasm_bindgen(start)]
pub fn init() {
    // Set panic hook for better error messages in console
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
    
    // Log hello world message from core library
    let message = mgcore::system::utils::hello_world();
    console_log!("WASM initialized: {}", message);
}

/// Get version information
#[wasm_bindgen]
pub fn version() -> String {
    format!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

// Re-export only what's needed
pub use massive_graph_core::system;