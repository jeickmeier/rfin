//! Golden test suite for Monte Carlo pricing validation.
//!
//! Provides CSV-based test vectors for validating MC implementations
//! against known reference values (QuantLib, Bloomberg, analytical formulas).

pub mod loader;

pub use loader::{load_golden_tests, assert_within_tolerance};

