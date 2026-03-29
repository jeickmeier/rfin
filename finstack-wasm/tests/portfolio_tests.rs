//! Tests for portfolio WASM bindings.

use finstack_core::currency::Currency as CoreCurrency;
use finstack_core::dates::PeriodId;
use finstack_core::money::Money;
use finstack_core::HashMap;
use finstack_margin::{ImMethodology, NettingSetId, SimmSensitivities};
use finstack_portfolio::cashflows::{
    PortfolioCashflowBuckets as CorePortfolioCashflowBuckets,
    PortfolioCashflows as CorePortfolioCashflows,
};
use finstack_portfolio::margin::{
    NettingSetMargin as CoreNettingSetMargin, PortfolioMarginResult as CorePortfolioMarginResult,
};
use finstack_portfolio::PositionId;
use indexmap::IndexMap;
use time::macros::date;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn test_placeholder() {
    // Placeholder test to ensure the test file compiles
    // Actual tests would require browser environment and WASM setup
    let result = 1 + 1;
    assert_eq!(result, 2);
}

fn js_stringify(value: &JsValue) -> String {
    js_sys::JSON::stringify(value)
        .unwrap()
        .as_string()
        .unwrap_or_default()
}

#[wasm_bindgen_test]
fn test_portfolio_cashflows_json_roundtrip() {
    use finstack_wasm::PortfolioCashflows;

    let mut by_date = IndexMap::new();
    let mut totals = IndexMap::new();
    totals.insert(CoreCurrency::USD, Money::new(12_500.0, CoreCurrency::USD));
    by_date.insert(date!(2025 - 01 - 15), totals);

    let mut by_position = IndexMap::new();
    by_position.insert(
        PositionId::new("POS_1"),
        vec![(
            date!(2025 - 01 - 15),
            Money::new(12_500.0, CoreCurrency::USD),
        )],
    );

    let payload = serde_wasm_bindgen::to_value(&CorePortfolioCashflows {
        by_date,
        by_position,
        position_summaries: IndexMap::new(),
        warnings: Vec::new(),
    })
    .unwrap();

    let wrapped = PortfolioCashflows::from_json(payload.clone()).unwrap();
    assert_eq!(
        js_stringify(&payload),
        js_stringify(&wrapped.to_json().unwrap())
    );
}

#[wasm_bindgen_test]
fn test_portfolio_cashflow_buckets_json_roundtrip() {
    use finstack_wasm::PortfolioCashflowBuckets;

    let mut by_period = IndexMap::new();
    by_period.insert(
        PeriodId::month(2025, 1),
        Money::new(50_000.0, CoreCurrency::USD),
    );

    let payload =
        serde_wasm_bindgen::to_value(&CorePortfolioCashflowBuckets { by_period }).unwrap();
    let wrapped = PortfolioCashflowBuckets::from_json(payload.clone()).unwrap();
    assert_eq!(
        js_stringify(&payload),
        js_stringify(&wrapped.to_json().unwrap())
    );
}

#[wasm_bindgen_test]
fn test_netting_set_margin_json_roundtrip() {
    use finstack_wasm::NettingSetMargin;

    let mut breakdown = HashMap::default();
    breakdown.insert(
        "InterestRate".to_string(),
        Money::new(875_000.0, CoreCurrency::USD),
    );
    let margin = NettingSetMargin::from_json(
        serde_wasm_bindgen::to_value(
            &CoreNettingSetMargin::new(
                NettingSetId::bilateral("BANK_A", "CSA_01"),
                date!(2025 - 01 - 15),
                Money::new(1_250_000.0, CoreCurrency::USD),
                Money::new(150_000.0, CoreCurrency::USD),
                4,
                ImMethodology::Simm,
            )
            .with_simm_breakdown(SimmSensitivities::new(CoreCurrency::USD), breakdown),
        )
        .unwrap(),
    )
    .unwrap();

    let json = margin.to_json().unwrap();
    let restored = NettingSetMargin::from_json(json.clone()).unwrap();
    assert_eq!(
        js_stringify(&json),
        js_stringify(&restored.to_json().unwrap())
    );
}

#[wasm_bindgen_test]
fn test_portfolio_margin_result_json_roundtrip() {
    use finstack_wasm::PortfolioMarginResult;

    let mut result = CorePortfolioMarginResult::new(date!(2025 - 01 - 15), CoreCurrency::USD);
    let margin = CoreNettingSetMargin::new(
        NettingSetId::cleared("LCH"),
        date!(2025 - 01 - 15),
        Money::new(900_000.0, CoreCurrency::USD),
        Money::new(100_000.0, CoreCurrency::USD),
        5,
        ImMethodology::ClearingHouse,
    );
    result.add_netting_set(margin).unwrap();
    result.positions_without_margin = 2;
    result.add_degraded_position(PositionId::new("POS_9"), "missing VM source");

    let payload = serde_wasm_bindgen::to_value(&result).unwrap();
    let wrapped = PortfolioMarginResult::from_json(payload.clone()).unwrap();
    assert_eq!(
        js_stringify(&payload),
        js_stringify(&wrapped.to_json().unwrap())
    );
}

// Note: Comprehensive tests for WASM bindings are best done in JavaScript/TypeScript
// using the actual browser environment or Node.js with proper WASM setup.
// The Rust tests here primarily ensure compilation and basic structure.
//
// For full integration testing, see:
// - finstack-wasm/examples/src/ for TypeScript examples
// - Browser-based test suites using the compiled WASM package
