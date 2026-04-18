//! WebAssembly bindings for the Finstack financial computation library.
//!
//! The public API is consumed through a hand-written JS/TS facade (`index.js`)
//! that groups raw `wasm-bindgen` exports into crate-level namespaces mirroring
//! the Rust umbrella crate structure.

#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]

use wasm_bindgen::prelude::*;

pub mod api;
pub mod utils;

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_does_not_panic() {
        start();
    }
}
