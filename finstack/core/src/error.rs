//! Error types for financial computation and validation failures.
//!
//! This module defines the unified error hierarchy used throughout Finstack's
//! core library. All errors bubble up through the [`Error`] enum, which wraps
//! domain-specific failures like input validation, interpolation bounds,
//! currency mismatches, and calibration failures.
//!
//! # Design Philosophy
//!
//! - **Actionable errors**: Each variant includes enough context for callers to
//!   diagnose and potentially recover from failures
//! - **Non-exhaustive**: Error variants may expand in minor releases; always
//!   match with a catch-all `_` pattern for forward compatibility
//! - **Fuzzy suggestions**: Missing curve errors include similar IDs based on
//!   edit distance to guide users toward corrections
//! - **Serializable**: All error types support `serde` when the feature is enabled
//!
//! # Error Categories
//!
//! - **Input validation** ([`InputError`]): User-supplied data fails constraints
//!   (e.g., non-monotonic knots, invalid dates, missing curves)
//! - **Currency safety** ([`Error::CurrencyMismatch`]): Attempted cross-currency
//!   arithmetic without explicit conversion
//! - **Interpolation** ([`Error::InterpOutOfBounds`]): Query point falls outside
//!   curve bounds
//! - **Calibration** ([`Error::Calibration`]): Numerical solver or fitting
//!   procedure failed to converge
//! - **Validation** ([`Error::Validation`]): Market data fails no-arbitrage or
//!   structural checks
//!
//! # Examples
//!
//! ## Handling common input errors
//!
//! ```rust
//! use finstack_core::error::{Error, InputError};
//!
//! fn parse_knots(data: &[(f64, f64)]) -> Result<(), Error> {
//!     if data.len() < 2 {
//!         return Err(InputError::TooFewPoints.into());
//!     }
//!     
//!     // Check monotonicity
//!     for window in data.windows(2) {
//!         if window[1].0 <= window[0].0 {
//!             return Err(InputError::NonMonotonicKnots.into());
//!         }
//!     }
//!     
//!     Ok(())
//! }
//!
//! let invalid_data = vec![(1.0, 0.95), (0.5, 0.90)]; // Non-monotonic
//! assert!(parse_knots(&invalid_data).is_err());
//! ```
//!
//! ## Currency mismatch detection
//!
//! ```rust
//! use finstack_core::error::Error;
//! use finstack_core::money::Money;
//! use finstack_core::currency::Currency;
//!
//! let usd = Money::new(100.0, Currency::USD);
//! let eur = Money::new(85.0, Currency::EUR);
//!
//! // Attempting to add different currencies returns CurrencyMismatch
//! let result = usd + eur;
//! assert!(result.is_err());
//!
//! match result {
//!     Err(Error::CurrencyMismatch { expected, actual }) => {
//!         assert_eq!(expected, Currency::USD);
//!         assert_eq!(actual, Currency::EUR);
//!     }
//!     _ => panic!("Expected currency mismatch"),
//! }
//! ```
//!
//! ## Using error suggestions for missing curves
//!
//! ```rust
//! use finstack_core::error::Error;
//!
//! let available = vec![
//!     "USD_OIS".to_string(),
//!     "EUR_OIS".to_string(),
//!     "GBP_GILT".to_string(),
//! ];
//!
//! // Typo in curve name
//! let err = Error::missing_curve_with_suggestions("USD_OS", &available);
//! let msg = format!("{}", err);
//!
//! // Error message includes suggestions
//! assert!(msg.contains("USD_OIS") || msg.contains("Did you mean"));
//! ```
//!
//! # See Also
//!
//! - [`crate::Result`] - Type alias for `Result<T, Error>`
//! - [`InputError`] - Specific validation failure modes
//!
//! # References
//!
//! The fuzzy matching algorithm uses Levenshtein edit distance:
//! - Levenshtein, V. I. (1966). "Binary codes capable of correcting deletions,
//!   insertions, and reversals." *Soviet Physics Doklady*, 10(8), 707-710.

use crate::currency::Currency;
use crate::dates::calendar::business_days::BusinessDayConvention;
use thiserror::Error;
use time::Date;

/// Format suggestions for error messages.
fn format_suggestions(suggestions: &[String]) -> String {
    if suggestions.is_empty() {
        String::new()
    } else if suggestions.len() == 1 {
        format!(". Did you mean '{}'?", suggestions[0])
    } else {
        format!(". Did you mean one of: {}?", suggestions.join(", "))
    }
}

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
    /// Shape/dimension mismatch in matrix-like input (e.g. vol surface grid).
    #[error("Input dimensions do not match")]
    DimensionMismatch,
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
    /// Decimal value cannot be represented as f64 (overflow or loss of precision).
    ///
    /// This error occurs when attempting to convert an internal `Decimal` amount
    /// to `f64` and the value is outside the representable range.
    #[error("Decimal conversion overflow: value cannot be represented as f64")]
    ConversionOverflow,
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
    /// Non-finite numeric value (NaN or infinity) encountered where finite required.
    ///
    /// This error occurs when constructing monetary amounts or performing
    /// calculations that require finite values.
    #[error("Non-finite value: expected finite number, got {kind}")]
    NonFiniteValue {
        /// Description of the non-finite value (e.g., "NaN", "infinity", "-infinity").
        kind: String,
    },
    /// Fallback for miscellaneous validation problems not yet covered by a specific variant.
    #[error("Invalid input data")]
    Invalid,

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
    /// Use [`VolatilityConvention::ShiftedLognormal`](crate::volatility::VolatilityConvention::ShiftedLognormal)
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

/// Unified error type for all high-level APIs.
///
/// All user-facing validation issues bubble up via the [`Input`](Error::Input)
/// wrapper so callers can pattern-match on [`InputError`] for actionable
/// feedback. Internal failures remain grouped under [`Internal`].
///
/// # Variants
///
/// - **Input**: Wraps [`InputError`] for all validation failures
/// - **InterpOutOfBounds**: Query point outside interpolator domain
/// - **CurrencyMismatch**: Binary operation on incompatible currencies
/// - **Calibration**: Numerical fitting or solver convergence failure
/// - **Validation**: Market data structural checks failed
/// - **Internal**: Unexpected system-level failures
///
/// # Examples
///
/// ```rust
/// use finstack_core::error::{Error, InputError};
///
/// // Convert InputError to Error
/// let input_err: Error = InputError::TooFewPoints.into();
/// assert!(matches!(input_err, Error::Input(_)));
///
/// // Pattern match on error variants
/// fn handle_error(err: Error) -> String {
///     match err {
///         Error::Input(e) => format!("Invalid input: {}", e),
///         Error::CurrencyMismatch { expected, actual } => {
///             format!("Cannot mix {} and {}", expected, actual)
///         }
///         Error::InterpOutOfBounds => "Query outside curve range".to_string(),
///         Error::Calibration { message, .. } => format!("Calibration failed: {}", message),
///         Error::Validation(msg) => format!("Validation error: {}", msg),
///         Error::Internal => "Internal error".to_string(),
///         _ => "Unknown error".to_string(), // Non-exhaustive enum
///     }
/// }
///
/// let msg = handle_error(input_err);
/// assert!(msg.contains("Invalid input"));
/// ```
#[derive(Debug, Clone, PartialEq, Error)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

impl Error {
    /// Create a MissingCurve error with suggestions based on available curves.
    ///
    /// Performs fuzzy matching to find similar curve IDs.
    ///
    /// # Example
    ///
    /// ```
    /// use finstack_core::error::Error;
    ///
    /// let available = vec!["USD_OIS".to_string(), "EUR_OIS".to_string(), "GBP_OIS".to_string()];
    /// let err = Error::missing_curve_with_suggestions("USD_OS", &available);
    ///
    /// let msg = format!("{}", err);
    /// assert!(msg.contains("Did you mean"));
    /// ```
    pub fn missing_curve_with_suggestions(
        requested: impl Into<String>,
        available: &[String],
    ) -> Self {
        let requested_str = requested.into();
        let suggestions = fuzzy_suggestions(&requested_str, available.iter().map(String::as_str));
        Self::Input(InputError::MissingCurve {
            requested: requested_str,
            suggestions,
        })
    }

    /// Create a CalendarNotFound error with suggestions based on available calendars.
    ///
    /// Performs fuzzy matching to find similar calendar IDs.
    ///
    /// # Example
    ///
    /// ```
    /// use finstack_core::error::Error;
    ///
    /// let available = &["nyse", "target2", "gblo"];
    /// let err = Error::calendar_not_found_with_suggestions("nyes", available);
    ///
    /// let msg = format!("{}", err);
    /// assert!(msg.contains("Did you mean"));
    /// ```
    pub fn calendar_not_found_with_suggestions(
        requested: impl Into<String>,
        available: &[&str],
    ) -> Self {
        let requested_str = requested.into();
        let suggestions = fuzzy_suggestions(&requested_str, available.iter().copied());
        Self::Input(InputError::CalendarNotFound {
            requested: requested_str,
            suggestions,
        })
    }
}

/// Find fuzzy matches for a requested identifier among available options.
///
/// Returns up to 3 suggestions based on:
/// 1. Substring containment (case-insensitive)
/// 2. Edit distance ≤ 2
fn fuzzy_suggestions<'a>(
    requested: &str,
    available: impl Iterator<Item = &'a str>,
) -> Vec<String> {
    let requested_lower = requested.to_lowercase();
    let requested_chars: Vec<char> = requested_lower.chars().collect();

    let mut suggestions: Vec<String> = available
        .filter(|id| {
            let id_lower = id.to_lowercase();
            // Match if:
            // 1. Contains the requested string
            // 2. Requested contains this ID
            // 3. Edit distance is small
            id_lower.contains(&requested_lower)
                || requested_lower.contains(&id_lower)
                || edit_distance(&requested_chars, &id_lower) <= 2
        })
        .map(|s| s.to_string())
        .collect();

    // Limit to top 3 suggestions
    suggestions.truncate(3);
    suggestions
}

/// Simple Levenshtein edit distance for fuzzy matching.
fn edit_distance(a_chars: &[char], b: &str) -> usize {
    let b_len = b.chars().count();
    let a_len = a_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut prev_row: Vec<usize> = (0..=b_len).collect();
    let mut curr_row = vec![0; b_len + 1];

    for (i, &a_char) in a_chars.iter().enumerate() {
        curr_row[0] = i + 1;
        for (j, b_char) in b.chars().enumerate() {
            let cost = if a_char == b_char { 0 } else { 1 };
            curr_row[j + 1] = (curr_row[j] + 1)
                .min(prev_row[j + 1] + 1)
                .min(prev_row[j] + cost);
        }
        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[b_len]
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

    #[test]
    fn test_missing_curve_with_suggestions() {
        let available = vec![
            "USD_OIS".to_string(),
            "USD_GOVT".to_string(),
            "EUR_OIS".to_string(),
            "GBP_GILT".to_string(),
        ];

        // Test exact fuzzy match
        let err = Error::missing_curve_with_suggestions("USD_OS", &available);
        let msg = format!("{}", err);
        assert!(msg.contains("USD_OIS") || msg.contains("Did you mean"));

        // Test prefix match
        let err2 = Error::missing_curve_with_suggestions("USD", &available);
        let msg2 = format!("{}", err2);
        assert!(msg2.contains("USD_OIS") || msg2.contains("USD_GOVT"));

        // Test no match
        let err3 = Error::missing_curve_with_suggestions("JPY_UNKNOWN", &available);
        let msg3 = format!("{}", err3);
        assert!(msg3.contains("Curve not found"));
    }

    #[test]
    fn test_edit_distance() {
        let empty: Vec<char> = vec![];
        let abc: Vec<char> = vec!['a', 'b', 'c'];

        assert_eq!(edit_distance(&empty, ""), 0);
        assert_eq!(edit_distance(&abc, "abc"), 0);
        assert_eq!(edit_distance(&abc, "abd"), 1);
        assert_eq!(edit_distance(&abc, "ab"), 1);
        assert_eq!(edit_distance(&empty, "abc"), 3);
    }
}
