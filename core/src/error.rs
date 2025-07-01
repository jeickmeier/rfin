//! Domain-level error enumeration returned by most `rfin-core` APIs.
//!
//! The variants are **non-exhaustive**; match defensively on `_` to remain
//! forward-compatible.

use crate::currency::Currency;
#[cfg(feature = "std")]
use thiserror::Error;

#[cfg(not(feature = "std"))]
extern crate alloc;

/// Main error type for rfin-core operations.
///
/// This enum captures all domain-level failures that may arise in the core
/// crate.  It is marked `#[non_exhaustive]` so new variants can be added
/// without breaking existing code.
///
/// # Example
///
/// ```rust
/// # #[cfg(feature = "std")] {
/// use rfin_core::{Currency, Error};
///
/// // Simulate a failed attempt to add USD and EUR amounts.
/// let err = Error::CurrencyMismatch { expected: Currency::USD, actual: Currency::EUR };
/// assert_eq!(err.to_string(), "Currency mismatch: expected USD, got EUR");
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Error))]
#[non_exhaustive]
pub enum Error {
    /// Invalid or unknown currency code.
    #[cfg_attr(feature = "std", error("Invalid currency code"))]
    InvalidCurrency,
    /// Currency mismatch in a binary [`Money`](crate::money::Money) operation.
    #[cfg_attr(feature = "std", error("Currency mismatch: expected {expected}, got {actual}"))]
    CurrencyMismatch {
        /// The expected (left-hand) currency.
        expected: Currency,
        /// The actual (right-hand) currency encountered.
        actual: Currency,
    },
    /// Arithmetic overflow or underflow of the underlying numeric type.
    #[cfg_attr(feature = "std", error("Numeric overflow"))]
    Overflow,
    /// Attempted division of a monetary value by zero.
    #[cfg_attr(feature = "std", error("Division by zero"))]
    DivisionByZero,
    /// Loss of precision beyond the tolerance accepted by the library.
    #[cfg_attr(feature = "std", error("Precision loss in calculation"))]
    PrecisionLoss,
    /// Result is not a finite number (e.g. `NaN` or ±∞).
    #[cfg_attr(feature = "std", error("Invalid calculation result"))]
    InvalidResult,
    /// A required configuration value is missing or malformed.
    #[cfg_attr(feature = "std", error("Configuration error"))]
    Configuration,
    /// An external resource (file, network service, etc.) was unavailable.
    #[cfg_attr(feature = "std", error("Resource unavailable"))]
    ResourceUnavailable,
    /// An unexpected internal invariant was violated; this likely indicates a bug.
    #[cfg_attr(feature = "std", error("Internal system error"))]
    Internal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "std"))]
    use alloc::format;
    #[cfg(feature = "std")]
    use std::format;

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Error::InvalidCurrency), "Invalid currency code");
    }
}

// --- Manual trait implementations for no_std ---------------------------------

#[cfg(not(feature = "std"))]
impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::InvalidCurrency => write!(f, "Invalid currency code"),
            Error::CurrencyMismatch { expected, actual } => write!(
                f,
                "Currency mismatch: expected {}, got {}",
                expected, actual
            ),
            Error::Overflow => write!(f, "Numeric overflow"),
            Error::DivisionByZero => write!(f, "Division by zero"),
            Error::PrecisionLoss => write!(f, "Precision loss in calculation"),
            Error::InvalidResult => write!(f, "Invalid calculation result"),
            Error::Configuration => write!(f, "Configuration error"),
            Error::ResourceUnavailable => write!(f, "Resource unavailable"),
            Error::Internal => write!(f, "Internal system error"),
        }
    }
}