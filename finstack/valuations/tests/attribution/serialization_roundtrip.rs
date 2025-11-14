//! Attribution envelope JSON serialization roundtrip tests.
//!
//! Ensures attribution request/response envelopes can be serialized to JSON
//! and deserialized back without loss.

use finstack_core::currency::Currency;
use finstack_core::dates::create_date;
use finstack_core::market_data::context::MarketContextState;
use finstack_core::money::Money;
use finstack_valuations::attribution::{
    AttributionConfig, AttributionEnvelope, AttributionMethod, AttributionSpec,
};
use finstack_valuations::instruments::json_loader::InstrumentJson;
use finstack_valuations::instruments::Bond;
use time::Month;

#[test]
fn test_attribution_envelope_json_roundtrip() {
    let bond = Bond::fixed(
        "TEST-BOND",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        create_date(2024, Month::January, 1).unwrap(),
        create_date(2034, Month::January, 1).unwrap(),
        "USD-OIS",
    );

    let spec = AttributionSpec {
        instrument: InstrumentJson::Bond(bond),
        market_t0: MarketContextState {
            curves: vec![],
            surfaces: vec![],
            prices: std::collections::BTreeMap::new(),
            series: vec![],
            inflation_indices: vec![],
            credit_indices: vec![],
            collateral: std::collections::BTreeMap::new(),
        },
        market_t1: MarketContextState {
            curves: vec![],
            surfaces: vec![],
            prices: std::collections::BTreeMap::new(),
            series: vec![],
            inflation_indices: vec![],
            credit_indices: vec![],
            collateral: std::collections::BTreeMap::new(),
        },
        as_of_t0: create_date(2025, Month::January, 1).unwrap(),
        as_of_t1: create_date(2025, Month::January, 2).unwrap(),
        method: AttributionMethod::Parallel,
        config: None,
    };

    let envelope = AttributionEnvelope::new(spec);

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&envelope).unwrap();

    // Deserialize back
    let parsed: AttributionEnvelope = serde_json::from_str(&json).unwrap();

    // Verify schema version
    assert_eq!(parsed.schema, "finstack.attribution/1");

    // Verify dates
    assert_eq!(parsed.attribution.as_of_t0, envelope.attribution.as_of_t0);
    assert_eq!(parsed.attribution.as_of_t1, envelope.attribution.as_of_t1);

    // Verify method
    assert!(matches!(
        parsed.attribution.method,
        AttributionMethod::Parallel
    ));
}

#[test]
fn test_attribution_envelope_waterfall_roundtrip() {
    use finstack_valuations::attribution::AttributionFactor;

    let bond = Bond::fixed(
        "TEST-BOND",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        create_date(2024, Month::January, 1).unwrap(),
        create_date(2034, Month::January, 1).unwrap(),
        "USD-OIS",
    );

    let spec = AttributionSpec {
        instrument: InstrumentJson::Bond(bond),
        market_t0: MarketContextState {
            curves: vec![],
            surfaces: vec![],
            prices: std::collections::BTreeMap::new(),
            series: vec![],
            inflation_indices: vec![],
            credit_indices: vec![],
            collateral: std::collections::BTreeMap::new(),
        },
        market_t1: MarketContextState {
            curves: vec![],
            surfaces: vec![],
            prices: std::collections::BTreeMap::new(),
            series: vec![],
            inflation_indices: vec![],
            credit_indices: vec![],
            collateral: std::collections::BTreeMap::new(),
        },
        as_of_t0: create_date(2025, Month::January, 1).unwrap(),
        as_of_t1: create_date(2025, Month::January, 2).unwrap(),
        method: AttributionMethod::Waterfall(vec![
            AttributionFactor::Carry,
            AttributionFactor::RatesCurves,
            AttributionFactor::CreditCurves,
        ]),
        config: None,
    };

    let envelope = AttributionEnvelope::new(spec);
    let json = serde_json::to_string_pretty(&envelope).unwrap();
    let parsed: AttributionEnvelope = serde_json::from_str(&json).unwrap();

    // Verify waterfall method with correct order
    if let AttributionMethod::Waterfall(factors) = parsed.attribution.method {
        assert_eq!(factors.len(), 3);
        assert_eq!(factors[0], AttributionFactor::Carry);
        assert_eq!(factors[1], AttributionFactor::RatesCurves);
        assert_eq!(factors[2], AttributionFactor::CreditCurves);
    } else {
        panic!("Expected Waterfall method");
    }
}

#[test]
fn test_attribution_config_roundtrip() {
    let config = AttributionConfig {
        tolerance_abs: Some(0.01),
        tolerance_pct: Some(0.001),
        metrics: Some(vec!["theta".to_string(), "dv01".to_string()]),
    };

    let json = serde_json::to_string(&config).unwrap();
    let parsed: AttributionConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.tolerance_abs, Some(0.01));
    assert_eq!(parsed.tolerance_pct, Some(0.001));
    assert_eq!(parsed.metrics.as_ref().unwrap().len(), 2);
}

#[test]
fn test_attribution_envelope_from_example_json() {
    // Load the example JSON file
    let json = include_str!("json_examples/bond_attribution_parallel.example.json");

    // Parse it
    let envelope: AttributionEnvelope = serde_json::from_str(json).unwrap();

    // Verify structure
    assert_eq!(envelope.schema, "finstack.attribution/1");
    assert!(matches!(
        envelope.attribution.method,
        AttributionMethod::Parallel
    ));

    // Verify instrument
    if let InstrumentJson::Bond(bond) = &envelope.attribution.instrument {
        assert_eq!(bond.id.as_str(), "CORP-BOND-001");
        assert_eq!(bond.notional.currency(), Currency::USD);
    } else {
        panic!("Expected Bond instrument");
    }
}

#[test]
fn test_attribution_envelope_to_from_json_helpers() {
    let bond = Bond::fixed(
        "TEST-BOND",
        Money::new(1_000_000.0, Currency::USD),
        0.05,
        create_date(2024, Month::January, 1).unwrap(),
        create_date(2034, Month::January, 1).unwrap(),
        "USD-OIS",
    );

    let spec = AttributionSpec {
        instrument: InstrumentJson::Bond(bond),
        market_t0: MarketContextState {
            curves: vec![],
            surfaces: vec![],
            prices: std::collections::BTreeMap::new(),
            series: vec![],
            inflation_indices: vec![],
            credit_indices: vec![],
            collateral: std::collections::BTreeMap::new(),
        },
        market_t1: MarketContextState {
            curves: vec![],
            surfaces: vec![],
            prices: std::collections::BTreeMap::new(),
            series: vec![],
            inflation_indices: vec![],
            credit_indices: vec![],
            collateral: std::collections::BTreeMap::new(),
        },
        as_of_t0: create_date(2025, Month::January, 1).unwrap(),
        as_of_t1: create_date(2025, Month::January, 2).unwrap(),
        method: AttributionMethod::MetricsBased,
        config: None,
    };

    let envelope = AttributionEnvelope::new(spec);

    // Test to_string() helper
    let json_str = envelope.to_string().unwrap();

    // Test from_json() helper
    let parsed = AttributionEnvelope::from_json(&json_str).unwrap();

    assert_eq!(parsed.schema, envelope.schema);
    assert!(matches!(
        parsed.attribution.method,
        AttributionMethod::MetricsBased
    ));
}

#[test]
fn test_attribution_result_envelope_roundtrip() {
    use finstack_core::config::results_meta;
    use finstack_valuations::attribution::{
        AttributionResult, AttributionResultEnvelope, PnlAttribution,
    };

    let total = Money::new(1000.0, Currency::USD);
    let pnl_attr = PnlAttribution::new(
        total,
        "TEST-BOND",
        create_date(2025, Month::January, 1).unwrap(),
        create_date(2025, Month::January, 2).unwrap(),
        AttributionMethod::Parallel,
    );

    let result = AttributionResult {
        attribution: pnl_attr,
        results_meta: results_meta(&finstack_core::config::FinstackConfig::default()),
    };

    let envelope = AttributionResultEnvelope::new(result);
    let json_str = envelope.to_string().unwrap();
    let parsed = AttributionResultEnvelope::from_json(&json_str).unwrap();

    assert_eq!(parsed.schema, "finstack.attribution/1");
    assert_eq!(parsed.result.attribution.total_pnl, total);
}
