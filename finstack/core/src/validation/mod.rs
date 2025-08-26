//! Validation framework for composable validation logic
//!
//! This module provides a trait-based validation system that enables composable,
//! reusable validation logic across the finstack ecosystem. The framework supports
//! both pass/fail validation and warnings, allowing for flexible validation policies.

mod result;
mod traits;

pub use result::{ValidationResult, ValidationWarning, ValidationStatus};
pub use traits::{Validator, ValidatorExt, RangeValidator, LengthValidator};
