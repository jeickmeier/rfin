//! wasm-bindgen-test suite for `api::portfolio`.
//!
//! Covers portfolio_result_get_metric and apply_scenario_and_revalue
//! which return JsValue.

#![cfg(target_arch = "wasm32")]

use finstack_wasm::api::portfolio::*;
use finstack_wasm::api::scenarios::build_scenario_spec;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

fn portfolio_spec_json() -> String {
    r#"{"id":"test_portfolio","name":"Test","base_ccy":"USD","as_of":"2024-01-15","entities":{},"positions":[]}"#.to_string()
}

fn empty_market_json() -> String {
    let ctx = finstack_core::market_data::context::MarketContext::new();
    serde_json::to_string(&ctx).unwrap()
}

#[wasm_bindgen_test]
fn portfolio_result_get_metric_returns_undefined_for_missing() {
    let spec = portfolio_spec_json();
    let market = empty_market_json();
    let valuation_json = value_portfolio(&spec, &market, false).unwrap();
    let result = finstack_portfolio::results::PortfolioResult {
        valuation: serde_json::from_str(&valuation_json).unwrap(),
        metrics: Default::default(),
        meta: Default::default(),
    };
    let result_json = serde_json::to_string(&result).unwrap();
    let v = portfolio_result_get_metric(&result_json, "nonexistent").unwrap();
    assert!(v == JsValue::UNDEFINED);
}

#[wasm_bindgen_test]
fn apply_scenario_and_revalue_empty_portfolio() {
    let spec = portfolio_spec_json();
    let scenario = build_scenario_spec("stress", "[]", None, None, 0).unwrap();
    let market = empty_market_json();
    let result = apply_scenario_and_revalue(&spec, &scenario, &market).unwrap();
    let obj: serde_json::Value = serde_wasm_bindgen::from_value(result).unwrap();
    assert!(obj["valuation"].as_str().is_some());
    assert!(obj["report"].as_str().is_some());
}
