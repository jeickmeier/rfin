//! Shared conversion helpers for WASM bindings.
//!
//! Utilities for error mapping, JSON serialization, and decimal conversion
//! used across all domain binding modules.

use wasm_bindgen::JsValue;

/// Convert any `Display`-able error into a `JsValue` error string.
pub fn to_js_err(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}
