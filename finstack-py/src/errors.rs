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
//! ```rust,ignore
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

use finstack_core::error::{Error as CoreError, InputError};
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

    // Internal errors
    m.add("InternalError", py.get_type::<InternalError>())?;

    Ok(())
}

// =============================================================================
// Error Mapping
// =============================================================================

/// Map a Rust core error to an appropriate Python exception.
///
/// This function provides a centralized mapping from `finstack_core::Error`
/// to the custom Python exception hierarchy, with helpful error messages.
///
/// # Examples
///
/// ```rust,ignore
/// let result = compute_something().map_err(map_error)?;
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

/// Legacy mapping for backward compatibility with existing code.
///
/// New code should use `map_error` instead.
#[allow(dead_code)]
pub(crate) fn core_to_py(err: CoreError) -> PyErr {
    map_error(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exception_hierarchy() {
        Python::with_gil(|py| {
            // Create a test module to register exceptions
            let m = PyModule::new(py, "test_module").unwrap();
            register_exceptions(py, &m).unwrap();

            // Verify exceptions are registered
            assert!(m.getattr("FinstackError").is_ok());
            assert!(m.getattr("ConfigurationError").is_ok());
            assert!(m.getattr("MissingCurveError").is_ok());
            assert!(m.getattr("ConvergenceError").is_ok());
            assert!(m.getattr("CurrencyMismatchError").is_ok());
        });
    }

    #[test]
    fn test_currency_mismatch_mapping() {
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
