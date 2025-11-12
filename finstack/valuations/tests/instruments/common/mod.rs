//! Comprehensive unit tests for common instruments module.
//!
//! Organized by:
//! - metrics: Risk metrics and calculations
//! - parameters: Parameter types and conventions
//! - helpers: Utility functions and test fixtures
//! - test_helpers: Shared test utilities and fixtures

pub mod helpers;
pub mod metrics;
#[cfg(feature = "mc")]
pub mod mc;
pub mod parameters;
pub mod test_discountable;
pub mod test_helpers;
pub mod test_pricing;
