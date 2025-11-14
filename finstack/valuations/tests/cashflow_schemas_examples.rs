//! Cashflow schema and example parity tests.
//!
//! Validates that all example JSON files under `schemas/cashflow/1/examples/`
//! can be deserialized into the corresponding Rust types and re-serialized
//! back to equivalent JSON.

use finstack_valuations::cashflow::builder::specs::{
    AmortizationSpec, CouponType, DefaultEvent, DefaultModelSpec, FeeSpec, FeeTier,
    FixedCouponSpec, FloatingCouponSpec, FloatingRateSpec, Notional, PrepaymentModelSpec,
    RecoveryModelSpec, ScheduleParams,
};
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
    let json = include_str!("cashflow/json_examples/notional_par.example.json");
    test_roundtrip::<CashflowEnvelope<NotionalPayload>>(json);
}

#[test]
fn test_notional_percent_per_period() {
    let json = include_str!("cashflow/json_examples/notional_percent_per_period.example.json");
    test_roundtrip::<CashflowEnvelope<NotionalPayload>>(json);
}

#[test]
fn test_amortization_linear_to() {
    let json = include_str!("cashflow/json_examples/amortization_linear_to.example.json");
    test_roundtrip::<CashflowEnvelope<AmortizationSpecPayload>>(json);
}

#[test]
fn test_amortization_step_remaining() {
    let json = include_str!("cashflow/json_examples/amortization_step_remaining.example.json");
    test_roundtrip::<CashflowEnvelope<AmortizationSpecPayload>>(json);
}

#[test]
fn test_coupon_type_cash() {
    let json = include_str!("cashflow/json_examples/coupon_type_cash.example.json");
    test_roundtrip::<CashflowEnvelope<CouponTypePayload>>(json);
}

#[test]
fn test_coupon_type_split() {
    let json = include_str!("cashflow/json_examples/coupon_type_split.example.json");
    test_roundtrip::<CashflowEnvelope<CouponTypePayload>>(json);
}

#[test]
fn test_fixed_coupon_spec() {
    let json = include_str!("cashflow/json_examples/fixed_coupon_spec.example.json");
    test_roundtrip::<CashflowEnvelope<FixedCouponSpecPayload>>(json);
}

#[test]
fn test_floating_rate_spec() {
    let json = include_str!("cashflow/json_examples/floating_rate_spec.example.json");
    test_roundtrip::<CashflowEnvelope<FloatingRateSpecPayload>>(json);
}

#[test]
fn test_floating_coupon_spec() {
    let json = include_str!("cashflow/json_examples/floating_coupon_spec.example.json");
    test_roundtrip::<CashflowEnvelope<FloatingCouponSpecPayload>>(json);
}

#[test]
fn test_prepayment_model_constant() {
    let json = include_str!("cashflow/json_examples/prepayment_model_constant.example.json");
    test_roundtrip::<CashflowEnvelope<PrepaymentModelSpecPayload>>(json);
}

#[test]
fn test_prepayment_model_psa_100() {
    let json = include_str!("cashflow/json_examples/prepayment_model_psa_100.example.json");
    test_roundtrip::<CashflowEnvelope<PrepaymentModelSpecPayload>>(json);
}

#[test]
fn test_default_model_constant() {
    let json = include_str!("cashflow/json_examples/default_model_constant.example.json");
    test_roundtrip::<CashflowEnvelope<DefaultModelSpecPayload>>(json);
}

#[test]
fn test_default_model_sda_100() {
    let json = include_str!("cashflow/json_examples/default_model_sda_100.example.json");
    test_roundtrip::<CashflowEnvelope<DefaultModelSpecPayload>>(json);
}

#[test]
fn test_default_event() {
    let json = include_str!("cashflow/json_examples/default_event.example.json");
    test_roundtrip::<CashflowEnvelope<DefaultEventPayload>>(json);
}

#[test]
fn test_recovery_model_standard() {
    let json = include_str!("cashflow/json_examples/recovery_model_standard.example.json");
    test_roundtrip::<CashflowEnvelope<RecoveryModelSpecPayload>>(json);
}

#[test]
fn test_fee_spec_fixed() {
    let json = include_str!("cashflow/json_examples/fee_spec_fixed.example.json");
    test_roundtrip::<CashflowEnvelope<FeeSpecPayload>>(json);
}

#[test]
fn test_fee_spec_periodic_bps() {
    let json = include_str!("cashflow/json_examples/fee_spec_periodic_bps.example.json");
    test_roundtrip::<CashflowEnvelope<FeeSpecPayload>>(json);
}

#[test]
fn test_fee_tier() {
    let json = include_str!("cashflow/json_examples/fee_tier.example.json");
    test_roundtrip::<CashflowEnvelope<FeeTierPayload>>(json);
}

#[test]
fn test_schedule_params_usd_standard() {
    let json = include_str!("cashflow/json_examples/schedule_params_usd_standard.example.json");
    test_roundtrip::<CashflowEnvelope<ScheduleParamsPayload>>(json);
}
