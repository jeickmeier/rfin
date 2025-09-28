//! Error mapping helpers for Python bindings.
//!
//! This module standardizes how `finstack-core` errors are exposed to Python by
//! translating them into idiomatic Python exceptions. Callers should use these
//! helpers to ensure consistent exception types and human-readable messages
//! across the bindings. The mapping favors `ValueError` for invalid inputs,
//! `KeyError` for missing identifiers, and `RuntimeError` for operational
//! failures.
//!
//! Example mapping (non-exhaustive):
//! - `Error::Input(NotFound)` → `KeyError`
//! - `Error::Input(AdjustmentFailed)` → `ValueError`
//! - `Error::CurrencyMismatch` → `ValueError`
//! - `Error::Calibration { .. }` → `RuntimeError`
//! - `Error::Validation(..)` → `ValueError`
//! - `Error::Internal` → `RuntimeError`
//! - Fallback → `RuntimeError` with the error's display string
//!
//! These helpers return `PyErr` values which callers should propagate with `?`
//! so Python sees the appropriate exception type and message.

use finstack_core::error::{Error, InputError};
use pyo3::exceptions::{PyKeyError, PyRuntimeError, PyValueError};
use pyo3::PyErr;

/// Convert a `finstack-core` `Error` into an idiomatic Python exception.
///
/// Parameters
/// ----------
/// err : Error
///     Error emitted by the Rust core.
///
/// Returns
/// -------
/// PyErr
///     Python exception matching the error category.
///
/// Notes
/// -----
/// Mapping highlights:
/// - `Error::Input(_)` delegates to [`input_to_py`].
/// - `Error::CurrencyMismatch` becomes `ValueError` describing expected vs actual.
/// - `Error::Calibration` becomes `RuntimeError` with category and message.
/// - `Error::Validation` becomes `ValueError`.
/// - `Error::Internal` becomes `RuntimeError`.
/// - Unknown variants fall back to `RuntimeError` with the display string.
pub(crate) fn core_to_py(err: Error) -> PyErr {
    match err {
        Error::Input(input) => input_to_py(input),
        Error::InterpOutOfBounds => PyValueError::new_err("Interpolation input out of bounds"),
        Error::CurrencyMismatch { expected, actual } => PyValueError::new_err(format!(
            "Currency mismatch: expected {expected}, got {actual}"
        )),
        Error::Calibration { message, category } => {
            PyRuntimeError::new_err(format!("Calibration error ({category}): {message}"))
        }
        Error::Validation(msg) => PyValueError::new_err(msg),
        Error::Internal => PyRuntimeError::new_err("Internal finstack error"),
        _ => PyRuntimeError::new_err(err.to_string()),
    }
}

/// Convert a core `InputError` into a specific Python exception.
///
/// Parameters
/// ----------
/// err : InputError
///     Concrete input error variant from the Rust core.
///
/// Returns
/// -------
/// PyErr
///     `KeyError` for missing identifiers, otherwise `ValueError` with details.
///
/// Notes
/// -----
/// - `InputError::NotFound { id }` → `KeyError(id)`.
/// - `InputError::AdjustmentFailed { .. }` → `ValueError` describing the attempt.
/// - All other variants fall back to `ValueError` using the error's display text.
pub(crate) fn input_to_py(err: InputError) -> PyErr {
    match err {
        InputError::NotFound { id } => PyKeyError::new_err(id),
        InputError::AdjustmentFailed {
            date,
            convention,
            max_days,
        } => PyValueError::new_err(format!(
            "Business day adjustment failed for {date} using {convention:?} within {max_days} days"
        )),
        other => PyValueError::new_err(other.to_string()),
    }
}

/// Create a `ValueError` for an unknown ISO currency code.
///
/// Parameters
/// ----------
/// code : &str
///     Three-letter ISO currency code provided by the user.
///
/// Returns
/// -------
/// PyErr
///     `ValueError` describing the unknown currency.
pub(crate) fn unknown_currency(code: &str) -> PyErr {
    PyValueError::new_err(format!("Unknown currency code: {code}"))
}

/// Create a `ValueError` for an unknown rounding mode name.
///
/// Parameters
/// ----------
/// name : &str
///     Rounding mode identifier supplied by the user.
///
/// Returns
/// -------
/// PyErr
///     `ValueError` describing the invalid rounding mode.
pub(crate) fn unknown_rounding_mode(name: &str) -> PyErr {
    PyValueError::new_err(format!("Unknown rounding mode: {name}"))
}

/// Create a `ValueError` for an unknown business-day convention name.
///
/// Parameters
/// ----------
/// name : &str
///     Business-day convention identifier supplied by the user.
///
/// Returns
/// -------
/// PyErr
///     `ValueError` describing the invalid convention.
pub(crate) fn unknown_business_day_convention(name: &str) -> PyErr {
    PyValueError::new_err(format!("Unknown business day convention: {name}"))
}

/// Create a `KeyError` for a missing calendar identifier.
///
/// Parameters
/// ----------
/// id : &str
///     Calendar id/code that could not be resolved.
///
/// Returns
/// -------
/// PyErr
///     `KeyError` describing the missing calendar.
pub(crate) fn calendar_not_found(id: &str) -> PyErr {
    PyKeyError::new_err(format!("Calendar not found: {id}"))
}
