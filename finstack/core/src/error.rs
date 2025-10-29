//! Domain-level error enumeration returned by most `rfin-core` APIs.
//!
//! The variants are **non-exhaustive**; match defensively on `_` to remain
//! forward-compatible.

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

        // Find curves that contain the requested string (case-insensitive fuzzy match)
        let requested_lower = requested_str.to_lowercase();
        let mut suggestions: Vec<String> = available
            .iter()
            .filter(|id| {
                let id_lower = id.to_lowercase();
                // Match if:
                // 1. Contains the requested string
                // 2. Starts with similar prefix
                // 3. Edit distance is small
                id_lower.contains(&requested_lower)
                    || requested_lower.contains(&id_lower)
                    || edit_distance(&requested_lower, &id_lower) <= 2
            })
            .cloned()
            .collect();

        // Limit to top 3 suggestions
        suggestions.truncate(3);

        Self::Input(InputError::MissingCurve {
            requested: requested_str,
            suggestions,
        })
    }
}

/// Simple Levenshtein edit distance for fuzzy matching.
fn edit_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut prev_row: Vec<usize> = (0..=b_len).collect();
    let mut curr_row = vec![0; b_len + 1];

    for i in 1..=a_len {
        curr_row[0] = i;
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            curr_row[j] = (curr_row[j - 1] + 1)
                .min(prev_row[j] + 1)
                .min(prev_row[j - 1] + cost);
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
        assert_eq!(edit_distance("", ""), 0);
        assert_eq!(edit_distance("abc", "abc"), 0);
        assert_eq!(edit_distance("abc", "abd"), 1);
        assert_eq!(edit_distance("abc", "ab"), 1);
        assert_eq!(edit_distance("", "abc"), 3);
    }
}
