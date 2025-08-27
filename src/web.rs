#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
	// Better panic messages in the browser console
	console_error_panic_hook::set_once();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn version() -> String {
	format!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

// Native stub to satisfy non-wasm builds of this bin target
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("Native build of massive-graph-web");
}


