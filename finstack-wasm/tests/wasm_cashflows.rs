//! wasm-bindgen-test suite for `finstack_wasm::api::cashflows`.

#![cfg(target_arch = "wasm32")]

use finstack_wasm::api::{cashflows, valuations::pricing::price_instrument};
use wasm_bindgen_test::*;

fn cashflow_spec_json() -> String {
    serde_json::json!({
        "notional": {
            "initial": {"amount": "1000000", "currency": "USD"},
            "amort": "None",
        },
        "issue": "2024-08-31",
        "maturity": "2025-08-31",
        "fixed_coupons": [{
            "coupon_type": "Cash",
            "rate": "0.06",
            "freq": {"count": 12, "unit": "months"},
            "dc": "Thirty360",
            "bdc": "following",
            "calendar_id": "weekends_only",
            "stub": "None",
            "end_of_month": false,
            "payment_lag_days": 0,
        }],
    })
    .to_string()
}

fn market_context_json() -> String {
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::DiscountCurve;
    let base = time::Date::from_calendar_date(2024, time::Month::January, 1).unwrap();
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)])
        .build()
        .unwrap();
    let ctx = MarketContext::new().insert(disc);
    serde_json::to_string(&ctx).unwrap()
}

#[wasm_bindgen_test]
fn cashflows_json_bridge_builds_accrues_and_prices_custom_bond() {
    let schedule_json = cashflows::build_cashflow_schedule(&cashflow_spec_json(), None)
        .expect("schedule should build");
    let validated = cashflows::validate_cashflow_schedule(&schedule_json).expect("valid schedule");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&schedule_json).unwrap(),
        serde_json::from_str::<serde_json::Value>(&validated).unwrap()
    );

    let flows_json = cashflows::dated_flows(&schedule_json).expect("dated flows");
    let flows: Vec<serde_json::Value> = serde_json::from_str(&flows_json).unwrap();
    assert!(!flows.is_empty());
    assert!(cashflows::accrued_interest(&schedule_json, "2025-02-28", None).unwrap() > 0.0);

    let instrument_json =
        cashflows::bond_from_cashflows("CUSTOM-CF", &schedule_json, "USD-OIS", Some(99.0))
            .expect("bond JSON");
    let result_json = price_instrument(
        &instrument_json,
        &market_context_json(),
        "2024-09-03",
        "discounting",
    )
    .expect("price custom bond");
    let result: serde_json::Value = serde_json::from_str(&result_json).unwrap();
    assert_eq!(result["instrument_id"], "CUSTOM-CF");
}
