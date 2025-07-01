//! WASM bindings for the RustFin library.

use wasm_bindgen::prelude::*;

mod currency;
mod money;
mod utils;

/// Initialize the WASM module
#[wasm_bindgen(start)]
pub fn init() {
    utils::set_panic_hook();
}
