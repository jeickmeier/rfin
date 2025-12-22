//! Input validation error types.
//!
//! This module defines [`InputError`], which captures all validation errors
//! related to user-supplied data. This includes structural issues, value
//! constraints, date/calendar problems, and missing references.

use crate::dates::calendar::business_days::BusinessDayConvention;
use thiserror::Error;
use time::Date;

use super::suggestions::format_suggestions;

/// Detailed user input validation failures.
///
/// This enum captures all validation errors related to user-supplied data,
/// including structural issues (too few points, non-monotonic sequences),
/// value constraints (negative/non-positive values), date/calendar problems,
/// and missing references in market data contexts.
///
/// # Variants
///
/// Each variant provides specific context about the validation failure to
/// enable actionable error messages and recovery logic.
///
/// # Examples
///
/// ```rust
/// use finstack_core::error::InputError;
///
/// // Too few data points
/// let err = InputError::TooFewPoints;
/// assert_eq!(err.to_string(), "At least two data points are required");
///
/// // Non-monotonic sequence
/// let err = InputError::NonMonotonicKnots;
/// assert_eq!(err.to_string(), "Times (knots) must be strictly increasing");
/// ```
#[derive(Debug, Clone, PartialEq, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum InputError {
    // ─────────────────────────────────────────────────────────────────────────
    // Basic Validation
    // ─────────────────────────────────────────────────────────────────────────
    /// Input must contain at least two distinct points (e.g. knots on a curve).
    #[error("At least two data points are required")]
    TooFewPoints,

    /// Knot/point times are not strictly increasing.
    #[error("Times (knots) must be strictly increasing")]
    NonMonotonicKnots,

    /// Encountered a value that must be strictly positive (e.g. discount factor).
    #[error("Values must be positive")]
    NonPositiveValue,

    /// Encountered a negative value where only non-negative values are allowed (e.g. hazard rate).
    #[error("Values must be non-negative")]
    NegativeValue,

    /// Shape/dimension mismatch in matrix-like input (e.g. vol surface grid).
    #[error("Input dimensions do not match")]
    DimensionMismatch,

    /// Fallback for miscellaneous validation problems not yet covered by a specific variant.
    #[error("Invalid input data")]
    Invalid,

    // ─────────────────────────────────────────────────────────────────────────
    // Date and Calendar
    // ─────────────────────────────────────────────────────────────────────────
    /// The provided date range is inverted – the start date is after the end date.
    #[error("Invalid date range: start must be before end")]
    InvalidDateRange,

    /// Invalid calendar date creation (e.g., February 30th).
    #[error("Invalid calendar date: {year}-{month:02}-{day:02}")]
    InvalidDate {
        /// Year component
        year: i32,
        /// Month component (1-12)
        month: u8,
        /// Day component (1-31)
        day: u8,
    },

    /// No business day found within maximum allowed adjustment period.
    #[error("Business day adjustment failed: no business day found within {max_days} days from {date} using {convention:?} convention")]
    AdjustmentFailed {
        /// The original date that couldn't be adjusted.
        date: Date,
        /// The business day convention that was attempted.
        convention: BusinessDayConvention,
        /// Maximum number of days searched.
        max_days: i32,
    },

    /// Unknown or unsupported calendar identifier.
    ///
    /// This error occurs when attempting to look up a calendar by ID that doesn't
    /// exist in the registry.
    #[error("Calendar not found: '{requested}'{}", format_suggestions(.suggestions))]
    CalendarNotFound {
        /// The requested calendar ID that was not found.
        requested: String,
        /// Similar calendar IDs that might be what the user meant.
        suggestions: Vec<String>,
    },

    // ─────────────────────────────────────────────────────────────────────────
    // Market Data and Lookups
    // ─────────────────────────────────────────────────────────────────────────
    /// Requested item (curve, surface, etc.) not found in a collection.
    #[error("Requested item not found: {id}")]
    NotFound {
        /// The identifier of the requested item that was not found
        id: String,
    },

    /// Requested curve not found in market context (with suggestions).
    #[error("Curve not found: {requested}{}", format_suggestions(.suggestions))]
    MissingCurve {
        /// The requested curve ID
        requested: String,
        /// Similar curve IDs that might be what the user meant
        suggestions: Vec<String>,
    },

    /// Unknown or unsupported currency code supplied by the caller.
    #[error("Unknown currency code")]
    UnknownCurrency,

    /// Invalid tenor string format.
    #[error("Invalid tenor '{tenor}': {reason}")]
    InvalidTenor {
        /// The tenor string that failed to parse.
        tenor: String,
        /// Reason for the parsing failure.
        reason: String,
    },

    /// Invalid credit rating string.
    #[error("Invalid credit rating: '{value}'")]
    InvalidRating {
        /// The rating string that failed to parse.
        value: String,
    },

    // ─────────────────────────────────────────────────────────────────────────
    // Numeric Conversion
    // ─────────────────────────────────────────────────────────────────────────
    /// Decimal value cannot be represented as f64 (overflow or loss of precision).
    ///
    /// This error occurs when attempting to convert an internal `Decimal` amount
    /// to `f64` and the value is outside the representable range.
    #[error("Decimal conversion overflow: value cannot be represented as f64")]
    ConversionOverflow,

    /// Non-finite numeric value (NaN or infinity) encountered where finite required.
    ///
    /// This error occurs when constructing monetary amounts or performing
    /// calculations that require finite values.
    #[error("Non-finite value: expected finite number, got {kind}")]
    NonFiniteValue {
        /// Description of the non-finite value (e.g., "NaN", "infinity", "-infinity").
        kind: String,
    },

    // ─────────────────────────────────────────────────────────────────────────
    // Volatility
    // ─────────────────────────────────────────────────────────────────────────
    /// Volatility conversion failed due to solver not converging.
    ///
    /// This error occurs when the numerical solver cannot find a volatility
    /// that produces the target price within tolerance.
    #[error("Volatility conversion failed: solver did not converge within tolerance {tolerance} (residual: {residual:.2e})")]
    VolatilityConversionFailed {
        /// The solver tolerance that was used.
        tolerance: f64,
        /// The residual at the best guess.
        residual: f64,
    },

    /// Lognormal volatility requires positive forward rate.
    ///
    /// The Black (lognormal) model is undefined for non-positive forward rates.
    /// Use [`VolatilityConvention::ShiftedLognormal`](crate::math::volatility::VolatilityConvention::ShiftedLognormal)
    /// with an appropriate shift for negative rate environments.
    #[error("Lognormal volatility requires positive forward rate (got {forward:.6}); use ShiftedLognormal with shift >= {required_shift:.6}")]
    NonPositiveForwardForLognormal {
        /// The forward rate that was provided.
        forward: f64,
        /// The minimum shift required to make the shifted forward positive.
        required_shift: f64,
    },

    /// Shifted lognormal conversion has non-positive shifted forward.
    ///
    /// The shifted forward (F + shift) must be positive for the model to be valid.
    #[error("Shifted forward must be positive: forward ({forward:.6}) + shift ({shift:.6}) = {shifted:.6}")]
    NonPositiveShiftedForward {
        /// The forward rate.
        forward: f64,
        /// The shift amount.
        shift: f64,
        /// The resulting shifted forward.
        shifted: f64,
    },

    /// Invalid volatility value (must be positive and finite).
    #[error("Invalid volatility: expected positive finite value, got {value}")]
    InvalidVolatility {
        /// The invalid volatility value.
        value: f64,
    },

    /// Invalid time to expiry (must be non-negative).
    #[error("Invalid time to expiry: expected non-negative, got {value:.6}")]
    InvalidTimeToExpiry {
        /// The invalid time to expiry value.
        value: f64,
    },

    // ─────────────────────────────────────────────────────────────────────────
    // Bumps and Scenarios
    // ─────────────────────────────────────────────────────────────────────────
    /// Unsupported bump operation on a market data type.
    ///
    /// This error occurs when attempting to apply a bump with an unsupported
    /// combination of mode, units, or bump type for a given curve/surface.
    #[error("Unsupported bump operation: {reason}")]
    UnsupportedBump {
        /// Description of why the bump is not supported.
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_too_few_points_display() {
        let err = InputError::TooFewPoints;
        assert_eq!(err.to_string(), "At least two data points are required");
    }

    #[test]
    fn test_non_monotonic_knots_display() {
        let err = InputError::NonMonotonicKnots;
        assert_eq!(err.to_string(), "Times (knots) must be strictly increasing");
    }

    #[test]
    fn test_invalid_date_display() {
        let err = InputError::InvalidDate {
            year: 2024,
            month: 2,
            day: 30,
        };
        assert_eq!(err.to_string(), "Invalid calendar date: 2024-02-30");
    }

    #[test]
    fn test_missing_curve_display() {
        let err = InputError::MissingCurve {
            requested: "USD_OS".to_string(),
            suggestions: vec!["USD_OIS".to_string()],
        };
        let msg = err.to_string();
        assert!(msg.contains("USD_OS"));
        assert!(msg.contains("Did you mean"));
        assert!(msg.contains("USD_OIS"));
    }

    #[test]
    fn test_volatility_conversion_failed_display() {
        let err = InputError::VolatilityConversionFailed {
            tolerance: 1e-8,
            residual: 0.001,
        };
        let msg = err.to_string();
        assert!(msg.contains("Volatility conversion failed"));
        assert!(msg.contains("1e-8") || msg.contains("0.00000001"));
    }

    #[test]
    fn test_non_positive_forward_display() {
        let err = InputError::NonPositiveForwardForLognormal {
            forward: -0.01,
            required_shift: 0.02,
        };
        let msg = err.to_string();
        assert!(msg.contains("Lognormal volatility"));
        assert!(msg.contains("-0.01"));
    }
}
