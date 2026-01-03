//! Parity tests for WASM bindings.
//!
//! These tests validate that key WASM exports exist and can be instantiated,
//! serving as a basic parity check against the Python bindings manifest.
//! They also test JSON serialization roundtrips for key types.

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// ============================================================================
// Core Types Existence Tests
// ============================================================================

/// Test that core date types can be constructed.
#[wasm_bindgen_test]
fn test_date_types_exist() {
    use finstack_wasm::*;

    // Date construction (may fail with invalid date, but we want to test existence)
    let date_result = FsDate::new(2025, 6, 15);
    assert!(date_result.is_ok(), "Should create valid date");

    // DayCount enum exists
    let _dc = DayCount::act_360();
    let _dc_30360 = DayCount::thirty_360();

    // Tenor can be constructed
    let tenor = Tenor::from_months(3);
    assert!(tenor.is_ok(), "Should create 3M tenor");
}

/// Test that schedule types can be constructed.
#[wasm_bindgen_test]
fn test_schedule_types_exist() {
    use finstack_wasm::*;

    let _freq = Frequency::monthly();
}

// ============================================================================
// Market Data Types Existence Tests
// ============================================================================

/// Test that market context can be constructed with defaults.
#[wasm_bindgen_test]
fn test_market_context_exists() {
    use finstack_wasm::*;

    let _ctx = MarketContext::new();
}

/// Test that bump types exist.
#[wasm_bindgen_test]
fn test_bump_types_exist() {
    use finstack_wasm::*;

    // BumpType has parallel
    let _type_parallel = BumpType::parallel();

    // BumpSpec has helper constructors
    let _spec = BumpSpec::parallel_bp(25.0);
}

// ============================================================================
// Statement Types Existence Tests
// ============================================================================

/// Test that normalization types can be constructed.
#[wasm_bindgen_test]
fn test_normalization_types_exist() {
    use finstack_wasm::*;

    let _config = NormalizationConfig::new("EBITDA");
}

// ============================================================================
// JSON Serialization Roundtrip Tests
// ============================================================================

/// Test that MarketContext can roundtrip through JSON.
#[wasm_bindgen_test]
fn test_market_context_json_roundtrip() {
    use finstack_wasm::*;

    let ctx = MarketContext::new();

    // Serialize to JSON string
    let json = ctx.to_json_string().expect("toJsonString should succeed");

    // Deserialize from JSON
    let restored = MarketContext::from_json(&json).expect("fromJson should succeed");

    // Verify the restored context produces valid JSON
    let _restored_json = restored
        .to_json_string()
        .expect("roundtrip toJsonString should succeed");
}

/// Test that FinstackConfig can roundtrip through JSON.
#[wasm_bindgen_test]
fn test_config_json_roundtrip() {
    use finstack_wasm::*;

    let config = FinstackConfig::new();

    // Serialize to JSON string
    let json = config
        .to_json_string()
        .expect("toJsonString should succeed");

    // Deserialize from JSON
    let _restored = FinstackConfig::from_json(&json).expect("fromJson should succeed");
}

/// Test that Adjustment can roundtrip through JSON.
#[wasm_bindgen_test]
fn test_adjustment_json_roundtrip() {
    use finstack_wasm::*;

    // Create a percentage adjustment
    let adj = Adjustment::percentage("adj1", "Test Adjustment", "revenue", 0.05)
        .expect("percentage should succeed");

    // Serialize to JSON string
    let json = adj.to_json_string().expect("toJsonString should succeed");

    // Deserialize from JSON
    let restored = Adjustment::from_json(&json).expect("fromJson should succeed");

    // Verify the restored JSON matches
    let restored_json = restored
        .to_json_string()
        .expect("roundtrip toJsonString should succeed");
    assert!(
        restored_json.contains("Test Adjustment"),
        "Name should be preserved"
    );
    assert!(
        restored_json.contains("revenue"),
        "Node ID should be preserved"
    );
}

/// Test that NormalizationConfig can roundtrip through JSON.
#[wasm_bindgen_test]
fn test_normalization_config_json_roundtrip() {
    use finstack_wasm::*;

    let config = NormalizationConfig::new("EBITDA");

    // Serialize to JSON string
    let json = config
        .to_json_string()
        .expect("toJsonString should succeed");

    // Deserialize from JSON
    let _restored = NormalizationConfig::from_json(&json).expect("fromJson should succeed");

    assert!(json.contains("EBITDA"), "Target node should be in JSON");
}

// ============================================================================
// Error Taxonomy Tests
// ============================================================================

/// Test that errors have proper error objects.
#[wasm_bindgen_test]
fn test_error_taxonomy() {
    use finstack_wasm::*;

    // Try to create a tenor with 0 months - should fail
    let bad_tenor = Tenor::from_months(0);
    assert!(bad_tenor.is_err(), "Should fail to create zero-month tenor");
}

// ============================================================================
// Performance & Registry Smoke Tests
// ============================================================================

/// Test that scalar NPV calculation is wired through to core.
#[wasm_bindgen_test]
fn test_calculate_npv_exists() {
    use finstack_wasm::*;
    use js_sys::{Array, Date as JsDate};
    use wasm_bindgen::JsValue;

    let cash_flows = Array::new();

    // Use local-time constructor to avoid timezone parsing surprises.
    let d0 = JsDate::new_with_year_month_day(2024, 0, 1);
    let d1 = JsDate::new_with_year_month_day(2025, 0, 1);

    let cf0 = Array::new();
    cf0.push(&d0.into());
    cf0.push(&JsValue::from_f64(-100_000.0));

    let cf1 = Array::new();
    cf1.push(&d1.into());
    cf1.push(&JsValue::from_f64(110_000.0));

    cash_flows.push(&cf0.into());
    cash_flows.push(&cf1.into());

    let pv = calculateNpv(cash_flows, 0.05).expect("calculateNpv should succeed");
    assert!(pv > 4700.0 && pv < 4800.0, "NPV should be ~4761.9");
}

/// Test that PricerRegistry can be constructed.
#[wasm_bindgen_test]
fn test_pricer_registry_exists() {
    use finstack_wasm::*;

    // Create an empty registry using the public constructor
    let _registry = PricerRegistry::new_empty();
}

/// Test that Bond can be created and converted to JsValue.
#[wasm_bindgen_test]
fn test_bond_creation() {
    use finstack_wasm::*;
    use wasm_bindgen::JsValue;

    let usd = Currency::new("USD").expect("Currency constructor should succeed");
    let notional = Money::new(1_000_000.0, &usd);
    let issue = FsDate::new(2024, 1, 1).expect("Valid date");
    let maturity = FsDate::new(2025, 1, 1).expect("Valid date");

    let bond = Bond::new(
        "bond1",
        &notional,
        &issue,
        &maturity,
        "USD-OIS",
        Some(0.05),
        Some(Frequency::semi_annual()),
        Some(DayCount::thirty_360()),
        Some(BusinessDayConvention::ModifiedFollowing),
        Some("usny".to_string()),
        Some(StubKind::none()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .expect("Bond constructor should succeed");
    let _bond_js: JsValue = bond.into();
}

/// Test that the generic `priceInstrument` entrypoint exists and accepts a typed wrapper as `JsValue`.
#[wasm_bindgen_test]
fn test_pricer_registry_price_instrument_exists() {
    use finstack_wasm::*;
    use wasm_bindgen::JsValue;

    // Use a smaller, modular registry to avoid WASM memory blowups while still
    // testing the dynamic dispatch entrypoint.
    let registry = createRatesRegistry();

    // Create a simple bond
    let usd = Currency::new("USD").expect("Currency constructor should succeed");
    let notional = Money::new(1_000_000.0, &usd);
    let issue = FsDate::new(2024, 1, 1).expect("Valid date");
    let maturity = FsDate::new(2025, 1, 1).expect("Valid date");
    let as_of = FsDate::new(2024, 6, 1).expect("Valid date");

    let bond = Bond::new(
        "bond1",
        &notional,
        &issue,
        &maturity,
        "USD-OIS",
        Some(0.05),
        Some(Frequency::semi_annual()),
        Some(DayCount::thirty_360()),
        Some(BusinessDayConvention::ModifiedFollowing),
        Some("usny".to_string()),
        Some(StubKind::none()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .expect("Bond constructor should succeed");
    let bond_js: JsValue = bond.into();
    let market = MarketContext::new();

    // The dynamic dispatch path should work (returns a pricing error due to missing market data).
    let result = registry.price_instrument(&bond_js, "discounting", &market, &as_of, None);

    assert!(
        result.is_err(),
        "Pricing should fail without required market data"
    );

    // Check that we got a valid error result
    let err = result.err().unwrap();
    let err_msg = js_sys::Reflect::get(&err, &JsValue::from_str("message"))
        .ok()
        .and_then(|v| v.as_string())
        .or_else(|| err.as_string())
        .unwrap_or_else(|| "Unknown error".to_string());

    assert!(!err_msg.is_empty(), "Error message should not be empty");
}
