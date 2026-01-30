//! Basic CDS Index metrics tests.
//!
//! Tests cover:
//! - NPV calculation via metrics framework
//! - Premium leg PV metric
//! - Protection leg PV metric
//! - Par spread metric
//! - Metric context and registry integration

use super::test_utils::*;
use finstack_valuations::instruments::credit_derivatives::cds::RECOVERY_SENIOR_UNSECURED;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

#[test]
fn test_metric_npv_matches_direct_value() {
    // Test: NPV via metrics matches direct value() call
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-NPV", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let direct_npv = idx.value(&ctx, as_of).unwrap();
    let result = idx.price_with_metrics(&ctx, as_of, &[]).unwrap();

    assert_money_approx_eq(result.value, direct_npv, 0.01, "Direct NPV vs metrics NPV");
}

#[test]
fn test_metric_par_spread() {
    // Test: Par spread metric calculation
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-PAR", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let result = idx
        .price_with_metrics(&ctx, as_of, &[MetricId::ParSpread])
        .unwrap();

    let par_spread = *result.measures.get("par_spread").unwrap();

    assert_positive(par_spread, "Par spread metric");
    let expected = flat_hazard_par_spread_bps(STANDARD_HAZARD_RATE, RECOVERY_SENIOR_UNSECURED);
    assert_in_range(
        par_spread,
        expected * 0.85,
        expected * 1.15,
        "Par spread near flat-hazard analytic",
    );
}

#[test]
fn test_metric_protection_leg_pv() {
    // Test: Protection leg PV metric
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-PROT", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let result = idx
        .price_with_metrics(&ctx, as_of, &[MetricId::ProtectionLegPv])
        .unwrap();

    let prot_pv = *result.measures.get("protection_leg_pv").unwrap();

    assert_positive(prot_pv, "Protection leg PV metric");
}

#[test]
fn test_metric_premium_leg_pv() {
    // Test: Premium leg PV metric
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-PREM", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let result = idx
        .price_with_metrics(&ctx, as_of, &[MetricId::PremiumLegPv])
        .unwrap();

    let prem_pv = *result.measures.get("premium_leg_pv").unwrap();

    assert_positive(prem_pv, "Premium leg PV metric");
}

#[test]
fn test_metric_legs_npv_consistency() {
    // Test: NPV = Protection PV - Premium PV (for protection buyer)
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-LEGS", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let result = idx
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::ProtectionLegPv, MetricId::PremiumLegPv],
        )
        .unwrap();

    let prot_pv = *result.measures.get("protection_leg_pv").unwrap();
    let prem_pv = *result.measures.get("premium_leg_pv").unwrap();
    let npv = result.value.amount();

    let expected_npv = prot_pv - prem_pv;
    assert_relative_eq(npv, expected_npv, 0.001, "NPV = Protection - Premium");
}

#[test]
fn test_multiple_metrics_single_call() {
    // Test: Multiple metrics computed in single call
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-MULTI", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let metrics = vec![
        MetricId::ParSpread,
        MetricId::ProtectionLegPv,
        MetricId::PremiumLegPv,
    ];

    let result = idx.price_with_metrics(&ctx, as_of, &metrics).unwrap();

    assert!(result.measures.contains_key("par_spread"));
    assert!(result.measures.contains_key("protection_leg_pv"));
    assert!(result.measures.contains_key("premium_leg_pv"));
}

#[test]
fn test_metrics_single_curve_mode() {
    // Test: Metrics work in single-curve mode
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-SINGLE", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let result = idx
        .price_with_metrics(&ctx, as_of, &[MetricId::ParSpread])
        .unwrap();

    assert!(result.measures.get("par_spread").is_some());
}

#[test]
fn test_metrics_constituents_mode() {
    // Test: Metrics work in constituents mode
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_constituents_index("CDX-CONST", start, end, 10_000_000.0, 5);
    let ctx = multi_constituent_market_context(as_of, 5);

    let result = idx
        .price_with_metrics(&ctx, as_of, &[MetricId::ParSpread])
        .unwrap();

    assert!(result.measures.get("par_spread").is_some());
}

#[test]
fn test_par_spread_matches_direct_method() {
    // Test: Par spread via metrics matches direct method
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-PAR", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let direct_par = idx.par_spread(&ctx, as_of).unwrap();

    let result = idx
        .price_with_metrics(&ctx, as_of, &[MetricId::ParSpread])
        .unwrap();
    let metric_par = *result.measures.get("par_spread").unwrap();

    assert_relative_eq(
        direct_par,
        metric_par,
        0.001,
        "Par spread: direct vs metric",
    );
}

#[test]
fn test_protection_pv_matches_direct_method() {
    // Test: Protection PV via metrics matches direct method
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-PROT", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let direct_prot = idx.pv_protection_leg(&ctx, as_of).unwrap().amount();

    let result = idx
        .price_with_metrics(&ctx, as_of, &[MetricId::ProtectionLegPv])
        .unwrap();
    let metric_prot = *result.measures.get("protection_leg_pv").unwrap();

    assert_relative_eq(
        direct_prot,
        metric_prot,
        0.001,
        "Protection PV: direct vs metric",
    );
}

#[test]
fn test_premium_pv_matches_direct_method() {
    // Test: Premium PV via metrics matches direct method
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-PREM", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let direct_prem = idx.pv_premium_leg(&ctx, as_of).unwrap().amount();

    let result = idx
        .price_with_metrics(&ctx, as_of, &[MetricId::PremiumLegPv])
        .unwrap();
    let metric_prem = *result.measures.get("premium_leg_pv").unwrap();

    assert_relative_eq(
        direct_prem,
        metric_prem,
        0.001,
        "Premium PV: direct vs metric",
    );
}

#[test]
fn test_empty_metrics_request() {
    // Test: Empty metrics request returns only NPV
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-EMPTY", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let result = idx.price_with_metrics(&ctx, as_of, &[]).unwrap();

    assert!(result.value.amount().is_finite());
    // Measures may be empty or contain default metrics
}

#[test]
fn test_metric_values_are_finite() {
    // Test: All metric values are finite (no NaN/Inf)
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-FINITE", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let metrics = vec![
        MetricId::ParSpread,
        MetricId::ProtectionLegPv,
        MetricId::PremiumLegPv,
    ];

    let result = idx.price_with_metrics(&ctx, as_of, &metrics).unwrap();

    for (name, value) in &result.measures {
        assert!(
            value.is_finite(),
            "Metric '{}' is not finite: {}",
            name,
            value
        );
    }
}

#[test]
fn test_par_spread_reasonable_range() {
    // Market Standard: Par spread should be in reasonable range for standard credit
    let start = date!(2025 - 01 - 01);
    let end = date!(2030 - 01 - 01);
    let as_of = start;

    let idx = standard_single_curve_index("CDX-RANGE", start, end, 10_000_000.0);
    let ctx = standard_market_context(as_of);

    let result = idx
        .price_with_metrics(&ctx, as_of, &[MetricId::ParSpread])
        .unwrap();
    let par_spread = *result.measures.get("par_spread").unwrap();

    let expected = flat_hazard_par_spread_bps(STANDARD_HAZARD_RATE, RECOVERY_SENIOR_UNSECURED);
    assert_in_range(
        par_spread,
        expected * 0.85,
        expected * 1.15,
        "Par spread near flat-hazard analytic",
    );
}
