//! Shared conversion helpers for WASM bindings.
//!
//! Utilities for error mapping, JSON serialization, and decimal conversion
//! used across all domain binding modules.

pub mod date;

pub use date::{date_to_iso, parse_iso_date, parse_iso_dates};

use wasm_bindgen::JsValue;

/// Convert any `Display`-able error into a structured `JsValue` error.
///
/// Returns a plain JS `Error` object whose `message` is the error's
/// `Display` text and whose `name` is `"FinstackError"`. Structured
/// errors let JS clients pattern-match on `err.name` and reliably read
/// `err.message` rather than parsing ad-hoc strings.
pub fn to_js_err(e: impl std::fmt::Display) -> JsValue {
    let msg = e.to_string();
    #[cfg(target_arch = "wasm32")]
    {
        let err = js_sys::Error::new(&msg);
        err.set_name("FinstackError");
        err.into()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = msg;
        JsValue::NULL
    }
}

// Native unit tests for `to_js_err` are limited because `js_sys::Error` only
// behaves normally under wasm32. The function is exercised indirectly by
// error-path wasm-bindgen tests.
