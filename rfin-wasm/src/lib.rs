//! WASM bindings for the RustFin library.

use wasm_bindgen::prelude::*;

mod currency;
mod dates;
mod money;
mod utils;
mod calendar;

/// Initialize the WASM module
#[wasm_bindgen(start)]
pub fn init() {
    utils::set_panic_hook();
}

// Re-export key types for ergonomic JS imports (`import { Date, Money, Currency } …`).
pub use currency::Currency;
pub use dates::Date;
pub use money::Money;
pub use calendar::{Calendar, BusDayConvention};
