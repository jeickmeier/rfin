//! Centralized error re-exports for the valuations crate.
//!
//! Domain-specific errors live near their implementation but are re-exported
//! here for discoverability and convenience.  Callers can import any
//! valuations error from one place:
//!
//! ```rust,ignore
//! use finstack_valuations::error::{PricingError, ValidationError, CorrelationMatrixError};
//! ```

pub use crate::instruments::common::models::correlation::factor_model::CorrelationMatrixError;
pub use crate::instruments::fixed_income::structured_credit::utils::validation::ValidationError;
pub use crate::pricer::{PricingError, PricingErrorContext, PricingResult};
