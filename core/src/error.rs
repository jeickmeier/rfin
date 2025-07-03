//! Domain-level error enumeration returned by most `rfin-core` APIs.
//!
//! The variants are **non-exhaustive**; match defensively on `_` to remain
//! forward-compatible.

use crate::currency::Currency;
use thiserror::Error;

/// Unified error type for all high-level APIs.
///
/// The variant set is intentionally minimal—everything outside the two common
/// failure modes bubbles up as `Internal` so downstream code has a single
/// catch-all branch.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum Error {
    /// Invalid user input or arguments (parse failure, inverted dates, …).
    #[error("Invalid input data")]
    InvalidInput,
    /// Currency mismatch in a binary [`Money`](crate::money::Money) operation.
    #[error("Currency mismatch: expected {expected}, got {actual}")]
    CurrencyMismatch {
        /// The expected (left-hand) currency.
        expected: Currency,
        /// The actual (right-hand) currency encountered.
        actual: Currency,
    },
    /// Catch-all for unexpected internal failures.
    #[error("Internal system error")]
    Internal,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", Error::InvalidInput), "Invalid input data");
    }
}
