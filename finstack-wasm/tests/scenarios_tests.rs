//! Integration tests for scenarios WASM bindings.

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_scenario_engine_creation() {
    use finstack_wasm::ScenarioEngine;

    let _engine = ScenarioEngine::new();
    // If this compiles and runs, the basic binding works
}

#[wasm_bindgen_test]
fn test_curve_kind_enum() {
    use finstack_wasm::ScenarioCurveKind;

    let discount = ScenarioCurveKind::DISCOUNT();
    let forecast = ScenarioCurveKind::FORWARD();

    // Test that we can create the enums
    assert_ne!(discount.to_string_js(), forecast.to_string_js());
}

#[wasm_bindgen_test]
fn test_tenor_match_mode_enum() {
    use finstack_wasm::TenorMatchMode;

    let exact = TenorMatchMode::EXACT();
    let interpolate = TenorMatchMode::INTERPOLATE();

    // Test that we can create the enums
    assert_ne!(exact.to_string_js(), interpolate.to_string_js());
}

#[wasm_bindgen_test]
fn test_vol_surface_kind_enum() {
    use finstack_wasm::VolSurfaceKind;

    let equity = VolSurfaceKind::EQUITY();
    let credit = VolSurfaceKind::CREDIT();
    let swaption = VolSurfaceKind::SWAPTION();

    // Test that we can create the enums
    assert_ne!(equity.to_string_js(), credit.to_string_js());
    assert_ne!(credit.to_string_js(), swaption.to_string_js());
}

#[wasm_bindgen_test]
fn test_operation_spec_creation() {
    use finstack_wasm::OperationSpec;

    // Test creating an equity price shock operation
    let _op = OperationSpec::equity_price_pct(vec!["SPY".to_string()], -10.0);
    // If this compiles and runs, the operation spec binding works
}

#[wasm_bindgen_test]
fn test_operation_spec_curve_shock() {
    use finstack_wasm::{OperationSpec, ScenarioCurveKind};

    let curve_kind = ScenarioCurveKind::DISCOUNT();
    let _op = OperationSpec::curve_parallel_bp(&curve_kind, "USD_SOFR".to_string(), None, 50.0);
    // If this compiles and runs, curve operation binding works
}

#[wasm_bindgen_test]
fn test_operation_spec_statement_shock() {
    use finstack_wasm::OperationSpec;

    let _op1 = OperationSpec::stmt_forecast_percent("Revenue".to_string(), -5.0);
    let _op2 = OperationSpec::stmt_forecast_assign("Cost".to_string(), 1000.0);
    // If this compiles and runs, statement operation bindings work
}

#[wasm_bindgen_test]
fn test_operation_spec_time_roll() {
    use finstack_wasm::OperationSpec;

    let _op = OperationSpec::time_roll_forward("1M".to_string(), Some(true), None);
    // If this compiles and runs, time roll operation binding works
}

#[wasm_bindgen_test]
fn test_operation_spec_structured_credit() {
    use finstack_wasm::OperationSpec;

    let operations = vec![
        OperationSpec::asset_correlation_pts(0.05),
        OperationSpec::prepay_default_correlation_pts(-0.10),
        OperationSpec::recovery_correlation_pts(0.02),
        OperationSpec::prepay_factor_loading_pts(0.15),
    ];

    for op in operations {
        let json = op.to_json().expect("serialize structured credit op");
        let roundtrip = OperationSpec::from_json(&json).expect("deserialize structured credit op");

        let original = js_sys::JSON::stringify(&json).unwrap();
        let reparsed = js_sys::JSON::stringify(&roundtrip.to_json().unwrap()).unwrap();
        assert_eq!(original, reparsed);
    }
}

#[wasm_bindgen_test]
fn test_rate_binding_from_json() {
    use finstack_wasm::{Compounding, RateBindingSpec};

    let binding = RateBindingSpec::new(
        "RateNode".to_string(),
        "USD_SOFR".to_string(),
        "1Y".to_string(),
        Some(Compounding::CONTINUOUS()),
        None,
    );

    let json = binding.to_json().expect("serialize rate binding");
    let parsed = RateBindingSpec::from_json(&json).expect("deserialize rate binding");

    assert_eq!(parsed.node_id(), "RateNode");
    assert_eq!(parsed.curve_id(), "USD_SOFR");
    assert_eq!(parsed.tenor(), "1Y");
}

#[wasm_bindgen_test]
fn test_operation_spec_json_roundtrip() {
    use finstack_wasm::OperationSpec;

    // Create an operation
    let op = OperationSpec::equity_price_pct(vec!["SPY".to_string()], -10.0);

    // Convert to JSON
    let json = op.to_json().expect("Failed to convert to JSON");

    // Convert back from JSON
    let op2 = OperationSpec::from_json(&json).expect("Failed to parse from JSON");

    // Verify roundtrip works
    let json2 = op2.to_json().expect("Failed to convert to JSON");
    assert_eq!(
        js_sys::JSON::stringify(&json).unwrap(),
        js_sys::JSON::stringify(&json2).unwrap()
    );
}
