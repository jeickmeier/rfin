//! wasm-bindgen-test suite for `api::core::market_data` FX bindings.

#![cfg(target_arch = "wasm32")]

use finstack_wasm::api::core::market_data::{FxConversionPolicy, FxMatrix};
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn fx_matrix_rate_returns_structured_result() {
    let matrix = FxMatrix::new();
    matrix.set_quote("EUR", "USD", 1.10).unwrap();

    let result = matrix
        .rate(
            "EUR",
            "USD",
            "2024-01-02",
            Some(FxConversionPolicy::cashflow_date()),
        )
        .unwrap();

    assert!((result.get_rate() - 1.10).abs() < 1e-12);
    assert!(!result.get_triangulated());
    assert_eq!(result.get_policy().get_name(), "cashflow_date");
}

#[wasm_bindgen_test]
fn fx_matrix_rate_defaults_policy_to_cashflow_date() {
    let matrix = FxMatrix::new();
    matrix.set_quote("GBP", "USD", 1.25).unwrap();

    let result = matrix.rate("GBP", "USD", "2024-01-02", None).unwrap();

    assert!((result.get_rate() - 1.25).abs() < 1e-12);
    assert_eq!(result.get_policy().get_name(), "cashflow_date");
}
