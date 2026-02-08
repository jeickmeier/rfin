//! Generic validation helpers for checking invariants.
//!
//! These helpers are convention-agnostic: they enforce structural invariants
//! (conditions, ordering, finiteness) without encoding market-specific defaults.

/// Require a condition to be true, otherwise return a validation error.
#[inline]
pub fn require(condition: bool, message: impl Into<String>) -> crate::Result<()> {
    if condition {
        Ok(())
    } else {
        Err(crate::Error::Validation(message.into()))
    }
}

/// Require a condition to be true, otherwise return the provided error.
#[inline]
pub fn require_or(condition: bool, err: impl Into<crate::Error>) -> crate::Result<()> {
    if condition {
        Ok(())
    } else {
        Err(err.into())
    }
}

/// Require a condition to be true, lazily constructing the error message.
#[inline]
pub fn require_with(condition: bool, message: impl FnOnce() -> String) -> crate::Result<()> {
    if condition {
        Ok(())
    } else {
        Err(crate::Error::Validation(message()))
    }
}
