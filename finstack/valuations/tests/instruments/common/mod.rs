//! Comprehensive unit tests for common instruments module.
//!
//! Organized by:
//! - models: Option pricing models (binomial, trinomial, SABR, etc.)
//! - metrics: Risk metrics and calculations
//! - parameters: Parameter types and conventions
//! - helpers: Utility functions and test fixtures
//! - test_traits: Core trait behavior tests (CashflowProvider, Priceable)

pub mod helpers;
pub mod metrics;
// pub mod models; // Disabled - tests private APIs
#[cfg(feature = "mc")]
pub mod mc;
pub mod parameters;
pub mod test_discountable;
pub mod test_helpers;
pub mod test_pricing;
// pub mod test_traits; // Disabled - tests removed traits
