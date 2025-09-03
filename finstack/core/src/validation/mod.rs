//! Validation framework for composable validation logic
//!
//! This module provides a trait-based validation system that enables composable,
//! reusable validation logic across the finstack ecosystem. The framework supports
//! both pass/fail validation and warnings, allowing for flexible validation policies.

mod multi;
mod result;
mod standard;
mod traits;

pub use multi::{
    BatchValidator, FinancialValidators, MultiValidator, ValidationError, ValidationErrors,
    ValidationPipeline,
};
pub use result::{ValidationResult, ValidationStatus, ValidationWarning};
pub use standard::{
    DateRangeValidator, DiscountFactorCurveValidator, DiscountFactorValidator,
    MonotonicTermStructureValidator, RateBoundsValidator, TermStructureKnotsValidator,
};
pub use traits::{LengthValidator, RangeValidator, Validator, ValidatorExt};
