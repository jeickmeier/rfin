//! Cashflow schema and example parity tests.
//!
//! Validates that all example JSON files under `schemas/cashflow/1/examples/`
//! can be deserialized into the corresponding Rust types and re-serialized
//! back to equivalent JSON.
//!
//! Also verifies that deserialized values match expected market-standard conventions.

use finstack_cashflows::builder::specs::{
    AmortizationSpec, CouponType, DefaultEvent, DefaultModelSpec, FeeSpec, FeeTier,
    FixedCouponSpec, FloatingCouponSpec, FloatingRateSpec, Notional, PrepaymentCurve,
    PrepaymentModelSpec, RecoveryModelSpec, ScheduleParams,
};
use finstack_core::dates::{BusinessDayConvention, DayCount};
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};

/// Generic envelope for cashflow specs with schema version.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct CashflowEnvelope<T> {
    schema: String,
    #[serde(flatten)]
    payload: T,
}

// Payload wrapper types for each spec
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct NotionalPayload {
    notional: Notional,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct AmortizationSpecPayload {
    amortization_spec: AmortizationSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct CouponTypePayload {
    coupon_type: CouponType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FixedCouponSpecPayload {
    fixed_coupon_spec: FixedCouponSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FloatingRateSpecPayload {
    floating_rate_spec: FloatingRateSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FloatingCouponSpecPayload {
    floating_coupon_spec: FloatingCouponSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct PrepaymentModelSpecPayload {
    prepayment_model_spec: PrepaymentModelSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct DefaultModelSpecPayload {
    default_model_spec: DefaultModelSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct DefaultEventPayload {
    default_event: DefaultEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct RecoveryModelSpecPayload {
    recovery_model_spec: RecoveryModelSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FeeSpecPayload {
    fee_spec: FeeSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FeeTierPayload {
    fee_tier: FeeTier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScheduleParamsPayload {
    schedule_params: ScheduleParams,
}

/// Helper to deserialize, re-serialize, and compare JSON for parity.
fn test_roundtrip<T>(json_str: &str)
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    // Deserialize from example JSON
    let envelope: T = serde_json::from_str(json_str).expect("Failed to deserialize");

    // Re-serialize
    let reserialized = serde_json::to_string(&envelope).expect("Failed to serialize");

    // Parse both to Value for comparison (to ignore whitespace/ordering)
    let original_value: serde_json::Value =
        serde_json::from_str(json_str).expect("Failed to parse original JSON");
    let reserialized_value: serde_json::Value =
        serde_json::from_str(&reserialized).expect("Failed to parse reserialized JSON");

    assert_eq!(
        original_value, reserialized_value,
        "Roundtrip mismatch for envelope"
    );
}

#[test]
fn test_notional_par() {
    let json = include_str!("examples/notional_par.example.json");
    test_roundtrip::<CashflowEnvelope<NotionalPayload>>(json);
}

#[test]
fn test_notional_percent_per_period() {
    let json = include_str!("examples/notional_percent_per_period.example.json");
    test_roundtrip::<CashflowEnvelope<NotionalPayload>>(json);
}

#[test]
fn test_amortization_linear_to() {
    let json = include_str!("examples/amortization_linear_to.example.json");
    test_roundtrip::<CashflowEnvelope<AmortizationSpecPayload>>(json);
}

#[test]
fn test_amortization_step_remaining() {
    let json = include_str!("examples/amortization_step_remaining.example.json");
    test_roundtrip::<CashflowEnvelope<AmortizationSpecPayload>>(json);
}

#[test]
fn test_coupon_type_cash() {
    let json = include_str!("examples/coupon_type_cash.example.json");
    test_roundtrip::<CashflowEnvelope<CouponTypePayload>>(json);
}

#[test]
fn test_coupon_type_split() {
    let json = include_str!("examples/coupon_type_split.example.json");
    test_roundtrip::<CashflowEnvelope<CouponTypePayload>>(json);
}

#[test]
fn test_fixed_coupon_spec() {
    let json = include_str!("examples/fixed_coupon_spec.example.json");

    // Verify deserialized values match market-standard conventions
    let envelope: CashflowEnvelope<FixedCouponSpecPayload> =
        serde_json::from_str(json).expect("Failed to deserialize");
    let spec = &envelope.payload.fixed_coupon_spec;

    // Rate should be 4.25% (expressed as 0.0425)
    assert!(
        (spec.rate.to_f64().unwrap_or(0.0) - 0.0425).abs() < 1e-10,
        "Fixed coupon rate should be 4.25%, got {}",
        spec.rate
    );

    // Day count should be 30/360 (standard for USD corporate bonds)
    assert_eq!(
        spec.dc,
        DayCount::Thirty360,
        "Fixed coupon day count should be 30/360"
    );

    // Business day convention should be Modified Following (market standard)
    assert_eq!(
        spec.bdc,
        BusinessDayConvention::ModifiedFollowing,
        "Fixed coupon BDC should be Modified Following"
    );

    // Coupon type should be Cash
    assert!(
        matches!(spec.coupon_type, CouponType::Cash),
        "Coupon type should be Cash"
    );

    test_roundtrip::<CashflowEnvelope<FixedCouponSpecPayload>>(json);
}

#[test]
fn test_floating_rate_spec() {
    let json = include_str!("examples/floating_rate_spec.example.json");
    test_roundtrip::<CashflowEnvelope<FloatingRateSpecPayload>>(json);
}

#[test]
fn test_floating_coupon_spec() {
    let json = include_str!("examples/floating_coupon_spec.example.json");

    // Verify deserialized values match market-standard conventions
    let envelope: CashflowEnvelope<FloatingCouponSpecPayload> =
        serde_json::from_str(json).expect("Failed to deserialize");
    let spec = &envelope.payload.floating_coupon_spec;

    // Spread should be 150 bps
    assert!(
        (spec.rate_spec.spread_bp.to_f64().unwrap_or(0.0) - 150.0).abs() < 1e-10,
        "Floating rate spread should be 150 bps, got {}",
        spec.rate_spec.spread_bp
    );

    // Day count should be Act/360 (standard for EUR EURIBOR)
    assert_eq!(
        spec.rate_spec.dc,
        DayCount::Act360,
        "Floating rate day count should be Act/360"
    );

    // Reset lag should be T-2 (standard for EURIBOR)
    assert_eq!(
        spec.rate_spec.reset_lag_days, 2,
        "Reset lag should be 2 days (T-2)"
    );

    // Gearing should be 1.0 (no leverage)
    assert!(
        (spec.rate_spec.gearing.to_f64().unwrap_or(0.0) - 1.0).abs() < 1e-10,
        "Gearing should be 1.0"
    );

    test_roundtrip::<CashflowEnvelope<FloatingCouponSpecPayload>>(json);
}

#[test]
fn test_prepayment_model_constant() {
    let json = include_str!("examples/prepayment_model_constant.example.json");
    test_roundtrip::<CashflowEnvelope<PrepaymentModelSpecPayload>>(json);
}

#[test]
fn test_prepayment_model_psa_100() {
    let json = include_str!("examples/prepayment_model_psa_100.example.json");

    // Verify deserialized values match PSA standard
    let envelope: CashflowEnvelope<PrepaymentModelSpecPayload> =
        serde_json::from_str(json).expect("Failed to deserialize");
    let spec = &envelope.payload.prepayment_model_spec;

    // CPR should be 6% (100% PSA terminal rate)
    assert!(
        (spec.cpr - 0.06).abs() < 1e-10,
        "PSA 100 CPR should be 6%, got {}",
        spec.cpr
    );

    // Should have PSA curve with 1.0 multiplier
    match &spec.curve {
        Some(PrepaymentCurve::Psa { speed_multiplier }) => {
            assert!(
                (*speed_multiplier - 1.0).abs() < 1e-10,
                "PSA 100 speed multiplier should be 1.0, got {}",
                speed_multiplier
            );
        }
        _ => panic!("PSA 100 should have Psa curve variant"),
    }

    test_roundtrip::<CashflowEnvelope<PrepaymentModelSpecPayload>>(json);
}

#[test]
fn test_default_model_constant() {
    let json = include_str!("examples/default_model_constant.example.json");
    test_roundtrip::<CashflowEnvelope<DefaultModelSpecPayload>>(json);
}

#[test]
fn test_default_model_sda_100() {
    let json = include_str!("examples/default_model_sda_100.example.json");
    test_roundtrip::<CashflowEnvelope<DefaultModelSpecPayload>>(json);
}

#[test]
fn test_default_event() {
    let json = include_str!("examples/default_event.example.json");
    test_roundtrip::<CashflowEnvelope<DefaultEventPayload>>(json);
}

#[test]
fn test_recovery_model_standard() {
    let json = include_str!("examples/recovery_model_standard.example.json");
    test_roundtrip::<CashflowEnvelope<RecoveryModelSpecPayload>>(json);
}

#[test]
fn test_fee_spec_fixed() {
    let json = include_str!("examples/fee_spec_fixed.example.json");
    test_roundtrip::<CashflowEnvelope<FeeSpecPayload>>(json);
}

#[test]
fn test_fee_spec_periodic_bps() {
    let json = include_str!("examples/fee_spec_periodic_bps.example.json");
    test_roundtrip::<CashflowEnvelope<FeeSpecPayload>>(json);
}

#[test]
fn test_fee_tier() {
    let json = include_str!("examples/fee_tier.example.json");
    test_roundtrip::<CashflowEnvelope<FeeTierPayload>>(json);
}

#[test]
fn test_schedule_params_usd_act360() {
    let json = include_str!("examples/schedule_params_usd_act360.example.json");

    // Verify deserialized values match USD market conventions
    let envelope: CashflowEnvelope<ScheduleParamsPayload> =
        serde_json::from_str(json).expect("Failed to deserialize");
    let spec = &envelope.payload.schedule_params;

    // Day count should be Act/360 (USD money market convention)
    assert_eq!(
        spec.dc,
        DayCount::Act360,
        "USD standard day count should be Act/360"
    );

    // Business day convention should be Modified Following
    assert_eq!(
        spec.bdc,
        BusinessDayConvention::ModifiedFollowing,
        "USD standard BDC should be Modified Following"
    );

    // Calendar should be USD
    assert_eq!(spec.calendar_id, "USD", "Calendar should be USD");

    test_roundtrip::<CashflowEnvelope<ScheduleParamsPayload>>(json);
}
