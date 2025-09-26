//! Domain-level error enumeration returned by most `rfin-core` APIs.
//!
//! The variants are **non-exhaustive**; match defensively on `_` to remain
//! forward-compatible.

use crate::currency::Currency;
use crate::dates::calendar::business_days::BusinessDayConvention;
use time::Date;
use thiserror::Error;

/// Detailed user input validation failures.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum InputError {
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
    /// The provided date range is inverted – the start date is after the end date.
    #[error("Invalid date range: start must be before end")]
    InvalidDateRange,
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
    /// Shape/dimension mismatch in matrix-like input (e.g. vol surface grid).
    #[error("Input dimensions do not match")]
    DimensionMismatch,
    /// Requested item (curve, surface, etc.) not found in a collection.
    #[error("Requested item not found: {id}")]
    NotFound {
        /// The identifier of the requested item that was not found
        id: String,
    },
    /// Unknown or unsupported currency code supplied by the caller.
    #[error("Unknown currency code")]
    UnknownCurrency,
    /// Fallback for miscellaneous validation problems not yet covered by a specific variant.
    #[error("Invalid input data")]
    Invalid,
}

/// Unified error type for all high-level APIs.
///
/// All user-facing validation issues bubble up via the [`Input`](Error::Input)
/// wrapper so callers can pattern-match on [`InputError`] for actionable
/// feedback.  Internal failures remain grouped under [`Internal`].
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum Error {
    /// User input validation error.
    #[error(transparent)]
    Input(#[from] InputError),
    /// Interpolator evaluation exceeded grid bounds.
    #[error("Interpolation input out of bounds")]
    InterpOutOfBounds,
    /// Currency mismatch in a binary [`Money`](crate::money::Money) operation.
    #[error("Currency mismatch: expected {expected}, got {actual}")]
    CurrencyMismatch {
        /// The expected (left-hand) currency.
        expected: Currency,
        /// The actual (right-hand) currency encountered.
        actual: Currency,
    },
    /// Calibration process failure.
    #[error("Calibration error: {message}")]
    Calibration {
        /// Human-readable error description.
        message: String,
        /// Error category for programmatic handling.
        category: String,
    },
    /// Market data validation failure (no-arbitrage, monotonicity, bounds).
    #[error("Validation error: {0}")]
    Validation(String),
    /// Catch-all for unexpected internal failures.
    #[error("Internal system error")]
    Internal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let err: Error = InputError::Invalid.into();
        assert_eq!(format!("{}", err), "Invalid input data");

        let not_found_err: Error = InputError::NotFound {
            id: "test_curve".to_string(),
        }
        .into();
        assert_eq!(
            format!("{}", not_found_err),
            "Requested item not found: test_curve"
        );
    }
}
