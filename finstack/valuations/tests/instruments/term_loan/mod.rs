//! Comprehensive term loan instrument test suite.
//!
//! # Test Organization
//!
//! - `construction`: Builder tests, validation, field assignment
//! - `cashflows`: Cashflow generation (fixed, floating, PIK, amortizing)
//! - `pricing`: Core pricing engine tests
//! - `metrics`: Individual metric calculator tests
//! - `validation`: Edge cases and boundary conditions
//! - `integration`: Multi-metric integration tests (formerly metrics.rs)

mod cashflows;
mod construction;
mod integration;
pub mod metrics;
mod pricing;
pub mod validation;

