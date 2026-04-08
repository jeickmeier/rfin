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

        // Unknown metric
        CoreError::UnknownMetric {
            metric_id,
            available,
        } => {
            let mut msg = format!("Unknown metric: '{metric_id}'");
            if !available.is_empty() {
                msg.push_str(&format!(". Available: {}", available.join(", ")));
            }
            ComputationError::new_err(msg)
        }

        // Metric not applicable
        CoreError::MetricNotApplicable {
            metric_id,
            instrument_type,
        } => ComputationError::new_err(format!(
            "Metric '{metric_id}' is not applicable to instrument type '{instrument_type}'"
        )),

        // Metric calculation failed
        CoreError::MetricCalculationFailed { metric_id, cause } => {
            ComputationError::new_err(format!("Metric '{metric_id}' calculation failed: {cause}"))
        }

        // Circular dependency
        CoreError::CircularDependency { path } => ComputationError::new_err(format!(
            "Circular dependency detected in metrics: {}",
            path.join(" -> ")
        )),

        // Internal errors
        CoreError::Internal(message) => InternalError::new_err(format!(
            "Internal finstack error - this is likely a bug. Please report it. Context: {message}"
        )),

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
        InputError::KnotSpacingTooSmall => ParameterError::new_err(
            "Consecutive knots are too close together for stable interpolation",
        ),

        // Date and calendar errors
        InputError::DateOutOfRange { date, range } => DateError::new_err(format!(
            "Date {} is outside the allowed range [{} to {}]",
            date, range.0, range.1
        )),
        InputError::CalendarNotFound {
            requested,
            suggestions,
        } => {
            let mut msg = format!("Calendar not found: '{requested}'");
            if !suggestions.is_empty() {
                msg.push_str(&format!(". Did you mean: {}?", suggestions.join(", ")));
            }
            ConfigurationError::new_err(msg)
        }
        InputError::MissingCalendarForBus252 => {
            ParameterError::new_err("DayCount::Bus252 requires a holiday calendar in DayCountCtx")
        }
        InputError::MissingFrequencyForActActIsma => ParameterError::new_err(
            "DayCount::ActActIsma requires a coupon frequency in DayCountCtx",
        ),
        InputError::ActActIsmaUnsupportedFrequency { frequency } => ParameterError::new_err(
            format!("DayCount::ActActIsma requires a Months/Years frequency, got {frequency}"),
        ),
        InputError::InvalidTenor { tenor, reason } => {
            ParameterError::new_err(format!("Invalid tenor '{tenor}': {reason}"))
        }
        InputError::InvalidRating { value } => {
            ParameterError::new_err(format!("Invalid credit rating: '{value}'"))
        }

        // Numeric conversion errors
        InputError::ConversionOverflow => {
            ParameterError::new_err("Decimal conversion overflow: value cannot be represented as f64")
        }
        InputError::NonFiniteValue { kind } => {
            ParameterError::new_err(format!("Non-finite value: expected finite number, got {kind}"))
        }

        // FX / rate errors
        InputError::InvalidFxRate { from, to, rate } => MissingFxRateError::new_err(format!(
            "Invalid FX rate for {from}->{to}: {rate}"
        )),
        InputError::InvalidBusBasis { basis } => {
            ParameterError::new_err(format!("Invalid Bus/252 basis: expected positive, got {basis}"))
        }
        InputError::RateConversionInvalidParams { function, reason } => {
            ParameterError::new_err(format!("Invalid rate conversion inputs for {function}: {reason}"))
        }
        InputError::FxTriangulationFailed {
            from,
            to,
            pivot,
            missing_leg,
        } => MissingFxRateError::new_err(format!(
            "FX triangulation failed for {from}->{to} via {pivot}: {missing_leg}"
        )),

        // Volatility errors
        InputError::VolatilityConversionFailed {
            tolerance,
            residual,
        } => ConvergenceError::new_err(format!(
            "Volatility conversion failed: solver did not converge within tolerance {tolerance} (residual: {residual:.2e})"
        )),
        InputError::NonPositiveForwardForLognormal {
            forward,
            required_shift,
        } => ParameterError::new_err(format!(
            "Lognormal volatility requires positive forward rate (got {forward:.6}); use ShiftedLognormal with shift >= {required_shift:.6}"
        )),
        InputError::NonPositiveShiftedForward {
            forward,
            shift,
            shifted,
        } => ParameterError::new_err(format!(
            "Shifted forward must be positive: forward ({forward:.6}) + shift ({shift:.6}) = {shifted:.6}"
        )),
        InputError::InvalidVolatility { value } => {
            ParameterError::new_err(format!("Invalid volatility: expected positive finite value, got {value}"))
        }
        InputError::InvalidTimeToExpiry { value } => {
            ParameterError::new_err(format!("Invalid time to expiry: expected non-negative, got {value:.6}"))
        }

        // Bump errors
        InputError::UnsupportedBump { reason } => {
            ParameterError::new_err(format!("Unsupported bump operation: {reason}"))
        }

        // Solver convergence
        InputError::SolverConvergenceFailed {
            iterations,
            residual,
            last_x,
            reason,
        } => ConvergenceError::new_err(format!(
            "Solver failed after {iterations} iterations: {reason} (residual: {residual:.6e}, last x: {last_x:.6e})"
        )),

        // Curve type mismatch
        InputError::WrongCurveType {
            id,
            expected,
            actual,
        } => ConfigurationError::new_err(format!(
            "Curve type mismatch for '{id}': expected '{expected}', got '{actual}'"
        )),

        // Allocation limit
        InputError::TooLarge {
            what,
            requested_bytes,
            limit_bytes,
        } => ParameterError::new_err(format!(
            "Allocation too large for {what}: requested {requested_bytes} bytes, limit {limit_bytes} bytes"
        )),

        // Fallback for any remaining input errors
        InputError::Invalid => ParameterError::new_err("Invalid input data"),

        // Catch-all for non-exhaustive enum (future variants)
        _ => ParameterError::new_err(format!("Invalid input parameter: {}", err)),
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
