//! Error conversion utilities for WASM bindings.
//!
//! Provides standardized error mapping from finstack-core errors to JavaScript
//! Error objects. This mirrors the Python bindings' error handling approach
//! and ensures consistent error messages across both binding layers.

use finstack_core::error::{Error, InputError};
use wasm_bindgen::JsValue;

/// Convert a finstack-core Error into a JavaScript Error value.
///
/// Maps core errors to descriptive JavaScript Error instances:
/// - `Error::Input(_)` → delegates to `input_to_js`
/// - `Error::CurrencyMismatch` → "Currency mismatch: expected X, got Y"
/// - `Error::Calibration` → "Calibration error (category): message"
/// - `Error::Validation` → "Validation error: message"
/// - `Error::Internal` → "Internal finstack error"
/// - Others → Generic error with display string
pub(crate) fn core_to_js(err: Error) -> JsValue {
    let message = match err {
        Error::Input(input) => return input_to_js(input),
        Error::InterpOutOfBounds => "Interpolation input out of bounds".to_string(),
        Error::CurrencyMismatch { expected, actual } => {
            format!("Currency mismatch: expected {expected}, got {actual}")
        }
        Error::Calibration { message, category } => {
            format!("Calibration error ({category}): {message}")
        }
        Error::Validation(msg) => format!("Validation error: {msg}"),
        Error::Internal => "Internal finstack error".to_string(),
        _ => err.to_string(),
    };
    js_error(message)
}

/// Convert a core InputError into a JavaScript Error value.
///
/// - `InputError::NotFound { id }` → "Not found: {id}"
/// - `InputError::AdjustmentFailed` → "Business day adjustment failed for {date}"
/// - Others → Generic error with display string
pub(crate) fn input_to_js(err: InputError) -> JsValue {
    let message = match err {
        InputError::NotFound { id } => format!("Not found: {id}"),
        InputError::AdjustmentFailed {
            date,
            convention,
            max_days,
        } => format!(
            "Business day adjustment failed for {date} using {convention:?} within {max_days} days"
        ),
        other => other.to_string(),
    };
    js_error(message)
}

/// Create a JavaScript Error for an unknown currency code.
pub(crate) fn unknown_currency(code: &str) -> JsValue {
    js_error(format!("Unknown currency code: {code}"))
}

/// Create a JavaScript Error for a missing calendar identifier.
pub(crate) fn calendar_not_found(id: &str) -> JsValue {
    js_error(format!("Calendar not found: {id}"))
}

/// Create a JavaScript Error for an unknown business day convention.
pub(crate) fn unknown_business_day_convention(name: &str) -> JsValue {
    js_error(format!("Unknown business day convention: {name}"))
}

/// Create a JavaScript Error for an unknown rounding mode.
pub(crate) fn unknown_rounding_mode(name: &str) -> JsValue {
    js_error(format!("Unknown rounding mode: {name}"))
}

/// Helper to create a JavaScript Error from any message.
#[inline]
fn js_error(message: impl Into<String>) -> JsValue {
    JsValue::from(js_sys::Error::new(&message.into()))
}

