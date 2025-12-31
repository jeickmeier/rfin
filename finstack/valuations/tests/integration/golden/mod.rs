//! Golden Test Framework
//!
//! This module provides golden test data loading for validating finstack implementations
//! against known reference values from QuantLib, Bloomberg, and analytical formulas.
//!
//! # Components
//!
//! ## Golden Test Data (`loader`)
//!
//! JSON-based test fixtures for regression and validation testing:
//! - Market compliance fixtures (`data/market_compliance/`)
//! - Option pricing vectors (`data/options/`)
//!
//! ## Parity Testing
//!
//! For parity testing (tolerance-based comparison), see `instruments/common/parity.rs`.
//! Use `use crate::parity::*;` in instrument tests.
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

// Golden test data loader
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
