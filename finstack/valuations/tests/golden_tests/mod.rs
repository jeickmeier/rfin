//! Golden test suite for pricing validation.
//!
//! Provides CSV-based test vectors for validating pricing implementations
//! against known reference values (QuantLib, Bloomberg, analytical formulas).
//!
//! # Structure
//!
//! - `data/european_options.csv`: European call/put test vectors
//! - `data/barrier_options.csv`: Barrier option test vectors
//! - `data/asian_options.csv`: Asian option test vectors
//!
//! # Usage
//!
//! ```rust,ignore
//! use finstack_valuations::tests::golden_tests::{load_golden_tests, golden_data_dir};
//!
//! let path = golden_data_dir().join("european_options.csv");
//! let cases = load_golden_tests(&path)?;
//!
//! for case in cases {
//!     let price = your_pricing_function(case.spot, case.strike, ...);
//!     assert_within_tolerance(&case, price, 0.0);
//! }
//! ```

pub mod loader;

pub use loader::{
    // Utilities
    golden_data_dir,
    // Asian options
    load_asian_tests,
    // Barrier options
    load_barrier_tests,
    // European options
    load_golden_tests,
    AsianTestCase,
    AveragingType,
    BarrierTestCase,
    BarrierType,
    GoldenTestCase,
    OptionType,
};
