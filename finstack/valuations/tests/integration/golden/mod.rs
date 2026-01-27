//! Golden Test Framework
//!
//! This module provides golden test data loading for validating finstack implementations
//! against known reference values from QuantLib, Bloomberg, and analytical formulas.
//!
//! # Migration Note
//!
//! This module now re-exports types from `finstack_core::golden` for consistency
//! across the codebase. Local loader functions are retained for backward compatibility
//! with existing option pricing test cases.
//!
//! # Components
//!
//! ## Core Framework (from `finstack_core::golden`)
//!
//! - `GoldenSuite<T>`: Canonical suite format with metadata
//! - `SuiteMeta`: Suite-level provenance information
//! - `Tolerance`: Comparison tolerance types (Abs, Rel, Bps, Pct)
//! - `Expectation`: Exact or range expectations
//! - `GoldenAssert`: Fluent assertion builder
//!
//! ## Local Types (for option pricing tests)
//!
//! - `GoldenTestCase`: European option test case
//! - `BarrierTestCase`: Barrier option test case
//! - `AsianTestCase`: Asian option test case
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::integration::golden::{load_golden_tests, golden_data_dir};
//!
//! let path = golden_data_dir().join("european_options.json");
//! let cases = load_golden_tests(&path)?;
//!
//! for case in cases {
//!     let price = black_scholes(case.spot, case.strike, ...);
//!     assert_within_tolerance(&case, price, 0.0);
//! }
//! ```

// Re-export core golden framework types
#[allow(unused_imports)]
pub use finstack_core::golden::{
    assert_abs, assert_bp, assert_expected_f64, assert_pct, assert_range,
    assert_within_tolerance as core_assert_within_tolerance, is_suite_ready, load_suite_from_path,
    load_suite_from_str, Expectation, ExpectedValue, GoldenAssert, GoldenSuite, SuiteMeta,
    Tolerance,
};

// Local loader for backward compatibility with option pricing tests
#[allow(dead_code)]
mod loader;

// Re-export loader types for option pricing tests
#[allow(unused_imports)]
pub use loader::{
    golden_data_dir, load_asian_tests, load_barrier_tests, load_golden_tests, AsianTestCase,
    AveragingType, BarrierTestCase, BarrierType, GoldenFile, GoldenTestCase, OptionType,
};

/// Path to market compliance fixtures directory.
///
/// Returns the path to `tests/integration/golden/data/market_compliance/`.
#[allow(dead_code)]
pub fn market_compliance_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("integration")
        .join("golden")
        .join("data")
        .join("market_compliance")
}
