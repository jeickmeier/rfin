//! Shared test utilities for risk tests.
//!
//! This module provides:
//! - Tolerance constants for numerical comparisons
//! - Assertion helpers with better error messages
//! - Market context builders for common test scenarios
//! - Option builders for consistent test instrument creation

pub mod assertions;
pub mod builders;
pub mod tolerances;

pub use assertions::*;
pub use builders::*;
pub use tolerances::*;
