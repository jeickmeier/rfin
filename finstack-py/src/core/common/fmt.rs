//! Shared helpers for Python `__format__` implementations on wrapper types.
//!
//! Python format spec parsing is a small, boring chore that was previously
//! duplicated across `money.rs` and `types.rs`. This module centralises the
//! logic so that every wrapper uses the same rules for precision extraction
//! and the same error message when a spec is unsupported.

use pyo3::exceptions::PyValueError;
use pyo3::PyErr;

/// Parse the precision field out of a Python format spec with a known suffix.
///
/// Accepts specs of the form `".{N}{suffix}"` or `"{N}{suffix}"`, with an
/// optional `"."` prefix. Returns `Some(N)` if a valid unsigned integer
/// precedes `suffix`; returns `Some(default)` if the numeric portion is empty
/// but the suffix matches; returns `None` if `spec` does not end with the
/// requested suffix.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(precision_for(".2f", 'f', 6), Some(2));
/// assert_eq!(precision_for("4f", 'f', 6), Some(4));
/// assert_eq!(precision_for("f", 'f', 6), Some(6));
/// assert_eq!(precision_for("2d", 'f', 6), None);
/// ```
pub(crate) fn precision_for(spec: &str, suffix: char, default: usize) -> Option<usize> {
    let tail = spec.strip_suffix(suffix)?;
    let digits = tail.strip_prefix('.').unwrap_or(tail);
    if digits.is_empty() {
        Some(default)
    } else {
        digits.parse::<usize>().ok().or(Some(default))
    }
}

/// Parse a bare precision spec of the form `".{N}"` or `"{N}"` (no suffix).
///
/// Returns `Some(default)` if the input is `"."` or empty after stripping the
/// leading dot. Returns `None` only if `spec` is empty (callers typically
/// short-circuit on that before calling this helper).
pub(crate) fn bare_precision(spec: &str, default: usize) -> Option<usize> {
    let digits = spec.strip_prefix('.').unwrap_or(spec);
    if digits.is_empty() {
        Some(default)
    } else {
        digits.parse::<usize>().ok().or(Some(default))
    }
}

/// Create a standard "Unsupported format spec" `PyValueError` for a wrapper
/// type that does not recognise the supplied spec.
pub(crate) fn unsupported_spec(type_name: &str, spec: &str) -> PyErr {
    PyValueError::new_err(format!("Unsupported format spec for {type_name}: '{spec}'"))
}
