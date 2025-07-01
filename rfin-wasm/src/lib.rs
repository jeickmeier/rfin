//! WASM bindings for the RustFin library.

use wasm_bindgen::prelude::*;

mod dates;
mod primitives;
mod utils;

// Removed wee_alloc as it's deprecated and not needed for modern WASM

/// Initialize the WASM module
#[wasm_bindgen(start)]
pub fn init() {
    utils::set_panic_hook();
}
