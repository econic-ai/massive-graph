//! # Massive Graph Browser WASM
//!
//! WebAssembly bindings for the Massive Graph database WebRTC POC.

use wasm_bindgen::prelude::*;


// Re-export logging macros for use in worker modules and main lib
pub(crate) use massive_graph_core::log_info;

// // Use log_info in this module to avoid unused import warning
// use crate::log_info;

// Module declarations
mod workers;
mod utils;
mod webrtc;

// Re-export worker structs for WASM bindings
pub use workers::{DedicatedWorker, BrowserApp, ServiceWorkerContext};

// Re-export WebRTC manager for WASM bindings
pub use webrtc::WebRtcWorkerManager;



/// Get version information
#[wasm_bindgen]
pub fn version() -> String {
    format!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

// Initialize WASM environment
#[wasm_bindgen(start)]
pub fn init() {
    log_info!("⭕ Massive Graph WASM initialised...");
}