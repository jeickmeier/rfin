//! Unified error handling for WASM bindings.
//!
//! Provides standardized error mapping from finstack-core errors to JavaScript
//! Error objects. This mirrors the Python bindings' error handling approach
//! and ensures consistent error messages across both binding layers.
//!
//! # Error Taxonomy
//!
//! Errors thrown from WASM have a `name` property that identifies the error kind,
//! matching Python exception names for cross-binding consistency:
//!
//! - `InputError` - Invalid inputs, missing data, unsupported operations
//! - `ValidationError` - Data validation failures
//! - `CalibrationError` - Calibration process failures
//! - `CurrencyError` - Currency mismatch or invalid currency
//! - `InterpError` - Interpolation out of bounds
//! - `InternalError` - Internal finstack errors
//! - `Error` - Generic errors
//!
//! # Example (JavaScript/TypeScript)
//! ```typescript
//! try {
//!     const result = someWasmFunction();
//! } catch (e) {
//!     if (e.name === 'ValidationError') {
//!         // Handle validation error
//!     } else if (e.name === 'CalibrationError') {
//!         // Handle calibration error
//!     }
//! }
//! ```

use finstack_core::{Error, InputError};
use wasm_bindgen::JsValue;

/// Error kind for taxonomy classification.
///
/// These kinds match Python exception names for cross-binding consistency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Invalid inputs, missing data, unsupported operations
    Input,
    /// Data validation failures
    Validation,
    /// Calibration process failures
    Calibration,
    /// Currency mismatch or invalid currency
    Currency,
    /// Interpolation out of bounds
    Interp,
    /// Internal finstack errors
    Internal,
    /// Generic errors
    Generic,
}

impl ErrorKind {
    /// Get the JavaScript error name for this kind.
    pub fn js_name(&self) -> &'static str {
        match self {
            ErrorKind::Input => "InputError",
            ErrorKind::Validation => "ValidationError",
            ErrorKind::Calibration => "CalibrationError",
            ErrorKind::Currency => "CurrencyError",
            ErrorKind::Interp => "InterpError",
            ErrorKind::Internal => "InternalError",
            ErrorKind::Generic => "Error",
        }
    }
}

/// Create a JavaScript Error with a specific kind (sets the `name` property).
///
/// This enables JavaScript/TypeScript code to discriminate errors by kind:
/// ```typescript
/// try { ... } catch (e) { if (e.name === 'ValidationError') { ... } }
/// ```
pub(crate) fn js_error_with_kind(kind: ErrorKind, message: impl ToString) -> JsValue {
    let error = js_sys::Error::new(&message.to_string());
    let _ = js_sys::Reflect::set(&error, &"name".into(), &kind.js_name().into());
    JsValue::from(error)
}

/// Convert a finstack-core Error into a JavaScript Error value.
///
/// Maps core errors to descriptive JavaScript Error instances with proper error kinds:
/// - `Error::Input(_)` → `InputError` (delegates to `input_to_js`)
/// - `Error::CurrencyMismatch` → `CurrencyError`
/// - `Error::Calibration` → `CalibrationError`
/// - `Error::Validation` → `ValidationError`
/// - `Error::InterpOutOfBounds` → `InterpError`
/// - `Error::Internal` → `InternalError`
/// - Others → `Error` (generic)
pub(crate) fn core_to_js(err: Error) -> JsValue {
    match err {
        Error::Input(input) => input_to_js(input),
        Error::InterpOutOfBounds => {
            js_error_with_kind(ErrorKind::Interp, "Interpolation input out of bounds")
        }
        Error::CurrencyMismatch { expected, actual } => js_error_with_kind(
            ErrorKind::Currency,
            format!("Currency mismatch: expected {expected}, got {actual}"),
        ),
        Error::Calibration { message, category } => js_error_with_kind(
            ErrorKind::Calibration,
            format!("Calibration error ({category}): {message}"),
        ),
        Error::Validation(msg) => {
            js_error_with_kind(ErrorKind::Validation, format!("Validation error: {msg}"))
        }
        Error::Internal(message) => js_error_with_kind(
            ErrorKind::Internal,
            format!("Internal finstack error: {message}"),
        ),
        _ => js_error_with_kind(ErrorKind::Generic, err.to_string()),
    }
}

/// Convert a core InputError into a JavaScript Error value.
///
/// All input errors are mapped to `InputError` kind:
/// - `InputError::NotFound { id }` → "Not found: {id}"
/// - `InputError::AdjustmentFailed` → "Business day adjustment failed for {date}"
/// - Others → Generic error with display string
pub(crate) fn input_to_js(err: InputError) -> JsValue {
    let message = match &err {
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
    js_error_with_kind(ErrorKind::Input, message)
}

/// Create a JavaScript Error for an unknown currency code.
pub(crate) fn unknown_currency(code: &str) -> JsValue {
    js_error_with_kind(
        ErrorKind::Currency,
        format!("Unknown currency code: {code}"),
    )
}

/// Create a JavaScript Error for a missing calendar identifier.
pub(crate) fn calendar_not_found(id: &str) -> JsValue {
    js_error_with_kind(ErrorKind::Input, format!("Calendar not found: {id}"))
}

// Note: unknown_business_day_convention and unknown_rounding_mode removed.
// These were unused - parsing now happens centrally via ParseFromString trait.

/// Unified error creation for JavaScript.
///
/// This is the single source of truth for creating JavaScript errors.
/// Use this instead of duplicating error creation logic.
#[inline]
pub(crate) fn js_error(message: impl ToString) -> JsValue {
    JsValue::from(js_sys::Error::new(&message.to_string()))
}

/// Macro for creating JavaScript errors with formatted messages.
#[macro_export]
macro_rules! js_err {
    ($($arg:tt)*) => {
        $crate::core::error::js_error(format!($($arg)*))
    };
}
