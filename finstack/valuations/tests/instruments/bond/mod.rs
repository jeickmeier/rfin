//! Comprehensive bond instrument test suite.
//!
//! # Test Organization
//!
//! - `construction`: Bond builder, validation, edge cases
//! - `cashflows`: Cashflow generation (fixed, floating, custom, amortizing)
//! - `pricing`: Basic pricing engine, settlement conventions, theta
//! - `metrics`: Individual metric calculator tests
//! - `validation`: Market benchmark validation (Fabozzi, Hull)
//! - `integration`: Complex bond structures (callable, putable, amortizing)

mod cashflows;
mod construction;
mod helpers_tests;
mod integration;
mod metrics;
mod pricing;
mod validation;
