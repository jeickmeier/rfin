//! Unified error hierarchy for the valuations crate.
//!
//! This module defines a single [`Error`] enum that wraps all domain-specific
//! error types, following the same pattern as [`finstack_core::Error`] wrapping
//! [`finstack_core::InputError`].
//!
//! # Design
//!
//! ```text
//! valuations::Error
//! ├── Pricing(PricingError)              ← pricer registry, model failures
//! ├── Correlation(CorrelationMatrixError) ← factor model validation
//! └── Validation(ValidationError)        ← structured credit waterfall
//! ```
//!
//! All variants convert one-way into [`finstack_core::Error`] via [`From`] for
//! seamless integration with the core error hierarchy.  The reverse direction
//! requires explicit context (see [`PricingError::from_core`]) to prevent
//! silent loss of error information.
//!
//! # Naming Convention
//!
//! Sub-errors use `{Domain}Error` prefixes (`PricingError`,
//! `CorrelationMatrixError`, `ValidationError`) so they can be imported
//! alongside `finstack_core::Error` without ambiguity. The unified wrapper
//! is re-exported at crate root as `ValuationsError`. See
//! `docs/CONVENTIONS_ERROR_NAMING.md` for the cross-crate naming rationale.
//!
//! # Module Layout Convention
//!
//! The error module layout across Finstack crates follows this convention:
//!
//! - **`error/mod.rs`** (directory with submodules): Use when the crate defines
//!   its own rich error hierarchy with multiple enums and helper logic
//!   (e.g., `finstack_core::error` has `Error`, `InputError`, suggestions).
//! - **`error.rs`** (flat file): Use for re-export facades that aggregate errors
//!   defined elsewhere in the crate (this module re-exports `PricingError`,
//!   `CorrelationMatrixError`, `ValidationError` from their source modules).
//!
//! # Examples
//!
//! ```rust,ignore
//! use finstack_valuations::error::{Error, PricingError, ValidationError, CorrelationMatrixError};
//!
//! // Domain errors automatically wrap into the unified type
//! let pricing_err: Error = PricingError::type_mismatch(
//!     InstrumentType::Bond,
//!     InstrumentType::Deposit,
//! ).into();
//!
//! // And the unified type converts into finstack_core::Error
//! let core_err: finstack_core::Error = pricing_err.into();
//! ```

pub use crate::instruments::common::models::correlation::factor_model::CorrelationMatrixError;
pub use crate::instruments::fixed_income::structured_credit::utils::validation::ValidationError;
pub use crate::pricer::{PricingError, PricingErrorContext, PricingResult};

/// Unified error type for the valuations crate.
///
/// Wraps domain-specific error types so callers can handle any valuations
/// error through a single type, consistent with [`finstack_core::Error`]
/// wrapping [`finstack_core::InputError`].
///
/// Each variant uses `#[error(transparent)]` to delegate `Display` to the
/// inner error and `#[from]` for ergonomic `?` conversion.
#[derive(Debug, Clone, PartialEq, thiserror::Error, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Error {
    /// Pricing model or registry error.
    #[error(transparent)]
    Pricing(#[from] PricingError),

    /// Correlation matrix validation error (factor model).
    #[error(transparent)]
    Correlation(#[from] CorrelationMatrixError),

    /// Structured credit waterfall validation error.
    #[error(transparent)]
    WaterfallValidation(#[from] ValidationError),
}

/// Convenience result type used throughout the valuations crate.
pub type Result<T> = std::result::Result<T, Error>;

/// One-way conversion from [`Error`] into [`finstack_core::Error`].
///
/// | `valuations::Error`       | `finstack_core::Error`          |
/// |---------------------------|---------------------------------|
/// | `Pricing(e)`              | Delegates to `From<PricingError>`|
/// | `Correlation(e)`          | `Validation(e.to_string())`     |
/// | `WaterfallValidation(e)`  | `Validation(e.to_string())`     |
impl From<Error> for finstack_core::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::Pricing(e) => e.into(),
            Error::Correlation(e) => finstack_core::Error::Validation(e.to_string()),
            Error::WaterfallValidation(e) => finstack_core::Error::Validation(e.to_string()),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn pricing_error_wraps_into_unified() {
        let pricing = PricingError::type_mismatch(
            crate::pricer::InstrumentType::Bond,
            crate::pricer::InstrumentType::Deposit,
        );
        let unified: Error = pricing.into();
        assert!(matches!(
            unified,
            Error::Pricing(PricingError::TypeMismatch { .. })
        ));
    }

    #[test]
    fn correlation_error_wraps_into_unified() {
        let corr = CorrelationMatrixError::InvalidSize {
            expected: 3,
            actual: 5,
        };
        let unified: Error = corr.into();
        assert!(matches!(
            unified,
            Error::Correlation(CorrelationMatrixError::InvalidSize { .. })
        ));
    }

    #[test]
    fn validation_error_wraps_into_unified() {
        let val = ValidationError::DuplicateTierId {
            tier_id: "A".into(),
        };
        let unified: Error = val.into();
        assert!(matches!(
            unified,
            Error::WaterfallValidation(ValidationError::DuplicateTierId { .. })
        ));
    }

    #[test]
    fn unified_converts_to_core_error() {
        // Pricing -> core
        let pricing = PricingError::model_failure_with_context(
            "test failure",
            PricingErrorContext::default(),
        );
        let core_err: finstack_core::Error = Error::Pricing(pricing).into();
        assert!(matches!(core_err, finstack_core::Error::Calibration { .. }));

        // Correlation -> core
        let corr = CorrelationMatrixError::NotSymmetric {
            i: 0,
            j: 1,
            diff: 0.01,
        };
        let core_err: finstack_core::Error = Error::Correlation(corr).into();
        assert!(matches!(core_err, finstack_core::Error::Validation(_)));

        // Validation -> core
        let val = ValidationError::EmptyTier {
            tier_id: "B".into(),
        };
        let core_err: finstack_core::Error = Error::WaterfallValidation(val).into();
        assert!(matches!(core_err, finstack_core::Error::Validation(_)));
    }
}
