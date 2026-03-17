//! Python exception hierarchy for Finstack.
//!
//! This module defines a structured exception hierarchy that maps Rust errors
//! to idiomatic Python exceptions with clear semantics and helpful error messages.
//!
//! # Exception Hierarchy
//!
//! ```text
//! FinstackError (base exception for all Finstack errors)
//! ├── ConfigurationError (setup/configuration errors)
//! │   ├── MissingCurveError (required curve not found in market)
//! │   ├── MissingFxRateError (required FX rate not available)
//! │   └── InvalidConfigError (invalid configuration values)
//! ├── ComputationError (runtime computation failures)
//! │   ├── ConvergenceError (solver failed to converge)
//! │   ├── CalibrationError (calibration failed)
//! │   └── PricingError (pricing computation failed)
//! ├── ValidationError (input validation failures)
//! │   ├── CurrencyMismatchError (incompatible currencies)
//! │   ├── DateError (invalid dates or date ranges)
//! │   └── ParameterError (invalid parameter values)
//! └── InternalError (unexpected internal errors - bugs)
//! ```
//!
//! # Usage
//!
//! Register exceptions in module init and use the mapping function to convert
//! Rust errors to appropriate Python exceptions:
//!
//! ```rust,no_run
//! use crate::errors::{register_exceptions, map_error};
//!
//! #[pymodule]
//! fn finstack(py: Python, m: &PyModule) -> PyResult<()> {
//!     register_exceptions(py, m)?;
//!     // ... rest of module setup
//! }
//!
//! #[pyfunction]
//! fn some_function() -> PyResult<Something> {
//!     let result = rust_function().map_err(map_error)?;
//!     Ok(result.into())
//! }
//! ```

use finstack_core::{Error as CoreError, InputError};
use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

// =============================================================================
// Exception Hierarchy Definition
// =============================================================================

// Base exception for all Finstack errors.
create_exception!(finstack, FinstackError, PyException);

// Configuration errors
create_exception!(finstack, ConfigurationError, FinstackError);
create_exception!(finstack, MissingCurveError, ConfigurationError);
create_exception!(finstack, MissingFxRateError, ConfigurationError);
create_exception!(finstack, InvalidConfigError, ConfigurationError);

// Computation errors
create_exception!(finstack, ComputationError, FinstackError);
create_exception!(finstack, ConvergenceError, ComputationError);
create_exception!(finstack, CalibrationError, ComputationError);
create_exception!(finstack, PricingError, ComputationError);

// Validation errors
create_exception!(finstack, ValidationError, FinstackError);
create_exception!(finstack, CurrencyMismatchError, ValidationError);
create_exception!(finstack, DateError, ValidationError);
create_exception!(finstack, ParameterError, ValidationError);
create_exception!(finstack, ConstraintValidationError, ParameterError);
create_exception!(finstack, CholeskyError, ParameterError);

// Internal errors
create_exception!(finstack, InternalError, FinstackError);

// =============================================================================
// Exception Registration
// =============================================================================

/// Register all custom exceptions in the Python module.
///
/// This must be called during module initialization to make exceptions
/// available to Python code.
pub fn register_exceptions(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Base exception
    m.add("FinstackError", py.get_type::<FinstackError>())?;

    // Configuration errors
    m.add("ConfigurationError", py.get_type::<ConfigurationError>())?;
    m.add("MissingCurveError", py.get_type::<MissingCurveError>())?;
    m.add("MissingFxRateError", py.get_type::<MissingFxRateError>())?;
    m.add("InvalidConfigError", py.get_type::<InvalidConfigError>())?;

    // Computation errors
    m.add("ComputationError", py.get_type::<ComputationError>())?;
    m.add("ConvergenceError", py.get_type::<ConvergenceError>())?;
    m.add("CalibrationError", py.get_type::<CalibrationError>())?;
    m.add("PricingError", py.get_type::<PricingError>())?;

    // Validation errors
    m.add("ValidationError", py.get_type::<ValidationError>())?;
    m.add(
        "CurrencyMismatchError",
        py.get_type::<CurrencyMismatchError>(),
    )?;
    m.add("DateError", py.get_type::<DateError>())?;
    m.add("ParameterError", py.get_type::<ParameterError>())?;
    m.add(
        "ConstraintValidationError",
        py.get_type::<ConstraintValidationError>(),
    )?;
    m.add("CholeskyError", py.get_type::<CholeskyError>())?;

    // Internal errors
    m.add("InternalError", py.get_type::<InternalError>())?;

    Ok(())
}

// =============================================================================
// Error Mapping
// =============================================================================

/// Map a Rust core error to an appropriate Python exception.
///
/// This is the underlying implementation. Binding code should use `core_to_py`
/// which is the stable public alias.
///
/// # Examples
///
/// ```rust,no_run
/// let result = compute_something().map_err(core_to_py)?;
/// ```
pub fn map_error(err: CoreError) -> PyErr {
    match err {
        // Input errors
        CoreError::Input(input_err) => map_input_error(input_err),

        // Configuration errors
        CoreError::InterpOutOfBounds => {
            ValidationError::new_err("Interpolation input out of bounds")
        }

        // Currency mismatch
        CoreError::CurrencyMismatch { expected, actual } => CurrencyMismatchError::new_err(
            format!("Currency mismatch: expected {}, got {}", expected, actual),
        ),

        // Calibration errors
        CoreError::Calibration { message, category } => {
            CalibrationError::new_err(format!("Calibration failed ({}): {}", category, message))
        }

        // Validation errors
        CoreError::Validation(msg) => ValidationError::new_err(msg),

        // Internal errors
        CoreError::Internal => InternalError::new_err(
            "Internal finstack error - this is likely a bug. Please report it.",
        ),

        // Fallback for any other error types
        other => FinstackError::new_err(other.to_string()),
    }
}

/// Map input-specific errors to Python exceptions.
fn map_input_error(err: InputError) -> PyErr {
    match err {
        // Specific curve not found error with suggestions
        InputError::MissingCurve {
            requested,
            suggestions,
        } => {
            let mut msg = format!("Curve not found: {}", requested);
            if !suggestions.is_empty() {
                msg.push_str(&format!(". Did you mean: {}?", suggestions.join(", ")));
            }
            MissingCurveError::new_err(msg)
        }

        // Generic not found error (non-curve resources)
        InputError::NotFound { id } => {
            ConfigurationError::new_err(format!("Resource not found: {}", id))
        }

        // Business day adjustment failures
        InputError::AdjustmentFailed {
            date,
            convention,
            max_days,
        } => DateError::new_err(format!(
            "Business day adjustment failed for {} using {:?} within {} days",
            date, convention, max_days
        )),

        // Unknown currency
        InputError::UnknownCurrency => ParameterError::new_err("Unknown currency code"),

        // Invalid date construction
        InputError::InvalidDate { year, month, day } => {
            DateError::new_err(format!("Invalid date: {}-{:02}-{:02}", year, month, day))
        }

        // Invalid date range
        InputError::InvalidDateRange => {
            DateError::new_err("Invalid date range: start must be before end")
        }

        // Validation errors
        InputError::TooFewPoints => {
            ParameterError::new_err("At least two data points are required")
        }
        InputError::NonMonotonicKnots => {
            ParameterError::new_err("Times (knots) must be strictly increasing")
        }
        InputError::NonPositiveValue => ParameterError::new_err("Values must be positive"),
        InputError::NegativeValue => ParameterError::new_err("Values must be non-negative"),
        InputError::DimensionMismatch => ParameterError::new_err("Input dimensions do not match"),

        // Fallback for any remaining input errors
        InputError::Invalid => ParameterError::new_err("Invalid input data"),

        // Catch-all for non-exhaustive enum (future variants)
        _ => ParameterError::new_err("Invalid input parameter"),
    }
}

// =============================================================================
// Helper Functions (previously in core/error.rs)
// =============================================================================

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
    ParameterError::new_err(format!("Unknown currency code: {code}"))
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
    ParameterError::new_err(format!("Unknown rounding mode: {name}"))
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
    ParameterError::new_err(format!("Unknown business day convention: {name}"))
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
    use pyo3::exceptions::PyKeyError;
    PyKeyError::new_err(format!("Calendar not found: {id}"))
}

/// Map a Rust core error to an appropriate Python exception.
///
/// This is the canonical binding-layer function for converting `finstack_core::Error`
/// to a Python exception. Prefer this over `map_error` in binding code.
pub(crate) fn core_to_py(err: CoreError) -> PyErr {
    map_error(err)
}

// =============================================================================
// Error Context Trait
// =============================================================================

/// Trait to add context to `PyResult` errors.
///
/// This allows adding field-specific information to generic PyO3 errors.
pub trait PyContext<T> {
    /// Wrap the error with additional context message.
    fn context(self, msg: &str) -> PyResult<T>;
}

impl<T> PyContext<T> for PyResult<T> {
    fn context(self, msg: &str) -> PyResult<T> {
        self.map_err(|e| {
            use pyo3::exceptions::PyValueError;
            PyValueError::new_err(format!("Invalid input for field '{}': {}", msg, e))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_python() {
        Python::initialize();
    }

    #[test]
    fn test_exception_hierarchy() -> PyResult<()> {
        init_python();
        Python::attach(|py| -> PyResult<()> {
            // Create a test module to register exceptions
            let m = PyModule::new(py, "test_module")?;
            register_exceptions(py, &m)?;

            // Verify exceptions are registered
            assert!(m.getattr("FinstackError").is_ok());
            assert!(m.getattr("ConfigurationError").is_ok());
            assert!(m.getattr("MissingCurveError").is_ok());
            assert!(m.getattr("ConvergenceError").is_ok());
            assert!(m.getattr("CurrencyMismatchError").is_ok());
            Ok(())
        })?;
        Ok(())
    }

    #[test]
    fn test_currency_mismatch_mapping() {
        init_python();
        use finstack_core::currency::Currency;

        let err = CoreError::CurrencyMismatch {
            expected: Currency::USD,
            actual: Currency::EUR,
        };

        let py_err = map_error(err);
        let err_msg = format!("{}", py_err);
        assert!(err_msg.contains("USD"));
        assert!(err_msg.contains("EUR"));
    }

    #[test]
    fn test_calibration_error_mapping() {
        init_python();
        let err = CoreError::Calibration {
            message: "Failed to fit quotes".to_string(),
            category: "yield_curve".to_string(),
        };

        let py_err = map_error(err);
        let err_msg = format!("{}", py_err);
        assert!(err_msg.contains("Calibration"));
        assert!(err_msg.contains("yield_curve"));
    }
}
