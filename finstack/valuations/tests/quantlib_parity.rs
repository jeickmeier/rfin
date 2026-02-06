//! QuantLib parity tests — validate pricing against known analytical values.
//!
//! These tests verify that our instrument pricing matches well-known analytical
//! reference values (consistent with QuantLib outputs) for the most important
//! P1 instruments: IRS, bonds, equity options, FX options, and CDS.
//!
//! Each test sets up minimal flat market data, prices the instrument, and compares
//! against analytically known values with appropriate tolerances.

#[allow(clippy::expect_used, clippy::unwrap_used, dead_code, unused_imports)]
mod finstack_test_utils {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/test_utils.rs"
    ));
}

#[path = "quantlib_parity/mod.rs"]
mod quantlib_parity_tests;
