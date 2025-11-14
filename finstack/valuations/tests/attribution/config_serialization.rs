//! Attribution configuration types JSON serialization roundtrip tests.
//!
//! Ensures all attribution config/request types can be serialized to JSON
//! and deserialized back without loss.

use finstack_valuations::attribution::{AttributionFactor, AttributionMethod, ModelParamsSnapshot};
use finstack_valuations::instruments::convertible::ConversionSpec;
use finstack_valuations::instruments::structured_credit::components::specs::{
    DefaultModelSpec, PrepaymentModelSpec, RecoveryModelSpec,
};

#[test]
fn test_attribution_method_parallel_roundtrip() {
    let method = AttributionMethod::Parallel;
    let json = serde_json::to_string(&method).unwrap();
    let deserialized: AttributionMethod = serde_json::from_str(&json).unwrap();

    assert!(matches!(deserialized, AttributionMethod::Parallel));
}

#[test]
fn test_attribution_method_waterfall_roundtrip() {
    let method = AttributionMethod::Waterfall(vec![
        AttributionFactor::Carry,
        AttributionFactor::RatesCurves,
        AttributionFactor::CreditCurves,
    ]);

    let json = serde_json::to_string(&method).unwrap();
    let deserialized: AttributionMethod = serde_json::from_str(&json).unwrap();

    if let AttributionMethod::Waterfall(factors) = deserialized {
        assert_eq!(factors.len(), 3);
        assert_eq!(factors[0], AttributionFactor::Carry);
        assert_eq!(factors[1], AttributionFactor::RatesCurves);
        assert_eq!(factors[2], AttributionFactor::CreditCurves);
    } else {
        panic!("Expected Waterfall variant");
    }
}

#[test]
fn test_attribution_method_metrics_based_roundtrip() {
    let method = AttributionMethod::MetricsBased;
    let json = serde_json::to_string(&method).unwrap();
    let deserialized: AttributionMethod = serde_json::from_str(&json).unwrap();

    assert!(matches!(deserialized, AttributionMethod::MetricsBased));
}

#[test]
fn test_attribution_factor_roundtrip() {
    let factors = vec![
        AttributionFactor::Carry,
        AttributionFactor::RatesCurves,
        AttributionFactor::CreditCurves,
        AttributionFactor::InflationCurves,
        AttributionFactor::Correlations,
        AttributionFactor::Fx,
        AttributionFactor::Volatility,
        AttributionFactor::ModelParameters,
        AttributionFactor::MarketScalars,
    ];

    for factor in factors {
        let json = serde_json::to_string(&factor).unwrap();
        let deserialized: AttributionFactor = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, factor);
    }
}

#[test]
fn test_model_params_snapshot_structured_credit_roundtrip() {
    let snapshot = ModelParamsSnapshot::StructuredCredit {
        prepayment_spec: PrepaymentModelSpec::psa(1.5),
        default_spec: DefaultModelSpec::constant_cdr(0.02),
        recovery_spec: RecoveryModelSpec::with_lag(0.60, 12),
    };

    let json = serde_json::to_string(&snapshot).unwrap();
    let deserialized: ModelParamsSnapshot = serde_json::from_str(&json).unwrap();

    if let ModelParamsSnapshot::StructuredCredit {
        prepayment_spec,
        default_spec,
        recovery_spec,
    } = deserialized
    {
        // Verify prepayment
        assert_eq!(prepayment_spec.cpr, PrepaymentModelSpec::psa(1.5).cpr);

        // Verify default
        assert_eq!(default_spec.cdr, 0.02);

        // Verify recovery
        assert_eq!(recovery_spec.rate, 0.60);
        assert_eq!(recovery_spec.recovery_lag, 12);
    } else {
        panic!("Expected StructuredCredit variant");
    }
}

#[test]
fn test_model_params_snapshot_convertible_roundtrip() {
    use finstack_valuations::instruments::convertible::{
        AntiDilutionPolicy, ConversionPolicy, DividendAdjustment,
    };

    let conversion_spec = ConversionSpec {
        ratio: Some(25.0),
        price: None,
        policy: ConversionPolicy::Voluntary,
        anti_dilution: AntiDilutionPolicy::None,
        dividend_adjustment: DividendAdjustment::None,
    };

    let snapshot = ModelParamsSnapshot::Convertible {
        conversion_spec: conversion_spec.clone(),
    };

    let json = serde_json::to_string(&snapshot).unwrap();
    let deserialized: ModelParamsSnapshot = serde_json::from_str(&json).unwrap();

    if let ModelParamsSnapshot::Convertible {
        conversion_spec: cs,
    } = deserialized
    {
        assert_eq!(cs.ratio, Some(25.0));
        assert_eq!(cs.price, None);
    } else {
        panic!("Expected Convertible variant");
    }
}

#[test]
fn test_model_params_snapshot_none_roundtrip() {
    let snapshot = ModelParamsSnapshot::None;

    let json = serde_json::to_string(&snapshot).unwrap();
    let deserialized: ModelParamsSnapshot = serde_json::from_str(&json).unwrap();

    assert!(matches!(deserialized, ModelParamsSnapshot::None));
}

#[test]
fn test_attribution_method_json_structure() {
    // Verify the JSON structure for waterfall matches expected shape
    let method = AttributionMethod::Waterfall(vec![
        AttributionFactor::Carry,
        AttributionFactor::RatesCurves,
    ]);

    let json = serde_json::to_value(&method).unwrap();

    // Should be a tagged enum with "Waterfall" key
    assert!(json.is_object());
    assert!(json.get("Waterfall").is_some());

    // The waterfall value should be an array of factors
    let factors = json.get("Waterfall").unwrap();
    assert!(factors.is_array());
    assert_eq!(factors.as_array().unwrap().len(), 2);
}

#[test]
fn test_model_params_snapshot_json_structure() {
    let snapshot = ModelParamsSnapshot::StructuredCredit {
        prepayment_spec: PrepaymentModelSpec::psa(1.0),
        default_spec: DefaultModelSpec::constant_cdr(0.02),
        recovery_spec: RecoveryModelSpec::with_lag(0.60, 12),
    };

    let json = serde_json::to_value(&snapshot).unwrap();

    // Should be a tagged enum
    assert!(json.is_object());
    assert!(json.get("StructuredCredit").is_some());

    // Should contain the three spec fields
    let structured = json.get("StructuredCredit").unwrap();
    assert!(structured.get("prepayment_spec").is_some());
    assert!(structured.get("default_spec").is_some());
    assert!(structured.get("recovery_spec").is_some());
}
