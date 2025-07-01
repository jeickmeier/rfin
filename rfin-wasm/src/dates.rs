//! WASM bindings for dates module.

use wasm_bindgen::prelude::*;

/// Example Date wrapper for WASM
#[wasm_bindgen]
pub struct Date {
    // TODO: Wrap rfin_core::dates::date::Date
}

#[wasm_bindgen]
impl Date {
    /// Create a new date
    #[wasm_bindgen(constructor)]
    pub fn new(_year: i32, _month: u8, _day: u8) -> Self {
        // TODO: Implement when core Date type is available
        Date {}
    }

    /// Convert date to string
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        // TODO: Implement
        "Date".to_string()
    }
}
