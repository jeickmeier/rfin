//! wasm-bindgen-test suite for `finstack_wasm::api::valuations`.
//!
//! Covers list_standard_metrics and price_instrument_with_metrics
//! which use JsValue.

#![cfg(target_arch = "wasm32")]

use finstack_wasm::api::valuations::pricing::{
    list_standard_metrics, price_instrument_with_metrics,
};
use wasm_bindgen_test::*;

fn bond_instrument_json() -> String {
    use finstack_core::currency::Currency;
    use finstack_core::money::Money;
    use finstack_valuations::instruments::fixed_income::bond::Bond;
    use finstack_valuations::instruments::InstrumentJson;

    let bond = Bond::fixed(
        "TEST-BOND",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap(),
        time::Date::from_calendar_date(2034, time::Month::January, 1).unwrap(),
        "USD-OIS",
    )
    .unwrap();
    serde_json::to_string(&InstrumentJson::Bond(bond)).unwrap()
}

fn market_context_json() -> String {
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::DiscountCurve;
    let base = time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap();
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots([(0.5, 0.99), (1.0, 0.98), (5.0, 0.90), (10.0, 0.80)])
        .build()
        .unwrap();
    let ctx = MarketContext::new().insert(disc);
    serde_json::to_string(&ctx).unwrap()
}

#[wasm_bindgen_test]
fn list_standard_metrics_returns_non_empty_array() {
    let result = list_standard_metrics().unwrap();
    let ids: Vec<String> = serde_wasm_bindgen::from_value(result).unwrap();
    assert!(!ids.is_empty());
}

#[wasm_bindgen_test]
fn price_instrument_with_metrics_returns_result() {
    let inst = bond_instrument_json();
    let mkt = market_context_json();
    let metrics = serde_wasm_bindgen::to_value(&vec!["dirty_price".to_string()]).unwrap();
    let result =
        price_instrument_with_metrics(&inst, &mkt, "2024-01-01", "discounting", metrics).unwrap();
    assert!(!result.is_empty());
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.is_object());
}
