//! Integration tests for CDS Option pricing workflows.

use super::common::*;
use crate::finstack_test_utils as test_utils;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::metrics::MetricId;
use rust_decimal::prelude::ToPrimitive;
use time::macros::date;

#[test]
fn test_call_option_positive_value() {
    let as_of = date!(2025 - 01 - 01);
    let option = CDSOptionBuilder::new().call().strike(100.0).build(as_of);
    let market = standard_market(as_of);

    let pv = option.value(&market, as_of).unwrap();

    assert_non_negative(pv.amount(), "Call option PV");
    assert_finite(pv.amount(), "Call option PV");
}

#[test]
fn test_put_option_positive_value() {
    let as_of = date!(2025 - 01 - 01);
    let option = CDSOptionBuilder::new().put().strike(100.0).build(as_of);
    let market = standard_market(as_of);

    let pv = option.value(&market, as_of).unwrap();

    assert_non_negative(pv.amount(), "Put option PV");
    assert_finite(pv.amount(), "Put option PV");
}

#[test]
fn test_atm_option_value() {
    let as_of = date!(2025 - 01 - 01);
    // Strike near forward spread (200bp based on 2% hazard * 10000 * (1-0.4))
    let option = CDSOptionBuilder::new().strike(200.0).build(as_of);
    let market = standard_market(as_of);

    let pv = option.value(&market, as_of).unwrap();

    assert_positive(pv.amount(), "ATM option should have positive time value");
}

#[test]
fn test_deep_itm_call() {
    let as_of = date!(2025 - 01 - 01);
    // Strike well below forward
    let option = CDSOptionBuilder::new().call().strike(50.0).build(as_of);
    let market = standard_market(as_of);

    let pv = option.value(&market, as_of).unwrap();

    assert_positive(pv.amount(), "Deep ITM call should have substantial value");
}

#[test]
fn test_deep_otm_call() {
    let as_of = date!(2025 - 01 - 01);
    // Strike well above forward
    let option = CDSOptionBuilder::new().call().strike(500.0).build(as_of);
    let market = standard_market(as_of);

    let pv = option.value(&market, as_of).unwrap();

    // OTM options still have time value
    assert_non_negative(pv.amount(), "OTM call should be non-negative");
}

#[test]
fn test_notional_scaling() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    let option1 = CDSOptionBuilder::new()
        .notional(10_000_000.0, finstack_core::currency::Currency::USD)
        .build(as_of);
    let option2 = CDSOptionBuilder::new()
        .notional(20_000_000.0, finstack_core::currency::Currency::USD)
        .build(as_of);

    let pv1 = option1.value(&market, as_of).unwrap().amount();
    let pv2 = option2.value(&market, as_of).unwrap().amount();

    // Double notional should approximately double PV
    assert_approx_eq(pv2 / pv1, 2.0, 0.001, "Notional scaling");
}

#[test]
fn test_time_to_expiry_effect() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    let mut values = Vec::new();
    for months in [3, 6, 12, 18, 24] {
        let option = CDSOptionBuilder::new()
            .expiry_months(months)
            .cds_maturity_months(months + 48)
            .build(as_of);
        let pv = option.value(&market, as_of).unwrap().amount();
        values.push((months as f64, pv));
    }

    // Longer time to expiry should increase option value
    assert_increasing(&values, "Time to expiry (months)", "Option value");
}

#[test]
fn test_volatility_effect() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    let mut values = Vec::new();
    for vol in [0.10, 0.20, 0.30, 0.40, 0.50] {
        let option = CDSOptionBuilder::new().implied_vol(vol).build(as_of);
        let pv = option.value(&market, as_of).unwrap().amount();
        values.push((vol, pv));
    }

    // Higher volatility should increase option value
    assert_increasing(&values, "Volatility", "Option value");
}

#[test]
fn test_near_expiry_option() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    // Very short time to expiry (1 week)
    let option = CDSOptionBuilder::new()
        .expiry_months(0) // Will be adjusted to very near-term
        .build(as_of);

    let result = option.value(&market, as_of);
    assert!(
        result.is_ok(),
        "Near-expiry option should price successfully"
    );
}

#[test]
fn test_very_short_dated_option() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);

    // Option with very short time to expiry (1 week via 0 months)
    let option = CDSOptionBuilder::new()
        .expiry_months(1) // 1 month is shortest practical period
        .cds_maturity_months(13)
        .build(as_of);

    let result = option.value(&market, as_of);
    assert!(
        result.is_ok(),
        "Very short-dated option should price successfully"
    );

    // Short-dated options have value
    let pv = result.unwrap().amount();
    assert_positive(pv, "Short-dated option value");
}

#[test]
fn test_forward_spread_calculation() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);
    let option = CDSOptionBuilder::new().build(as_of);

    let strike_bp = option.strike.to_f64().unwrap_or(0.0) * 10000.0;
    let mut underlying = test_utils::cds_buy_protection(
        "CDS-FWD",
        option.notional,
        strike_bp,
        option.expiry,
        option.cds_maturity,
        option.discount_curve_id.clone(),
        option.credit_curve_id.clone(),
    )
    .expect("underlying CDS should build");
    underlying.protection.recovery_rate = option.recovery_rate;
    let forward = underlying
        .price_with_metrics(&market, as_of, &[MetricId::ParSpread])
        .expect("par spread should compute")
        .measures[&MetricId::ParSpread];

    assert_positive(forward, "Forward spread");
    assert_in_range(forward, 50.0, 500.0, "Forward spread reasonableness");
}

#[test]
fn test_price_with_metrics() {
    let as_of = date!(2025 - 01 - 01);
    let market = standard_market(as_of);
    let option = CDSOptionBuilder::new().build(as_of);

    let result = option
        .price_with_metrics(
            &market,
            as_of,
            &[
                finstack_valuations::metrics::MetricId::Delta,
                finstack_valuations::metrics::MetricId::Vega,
            ],
        )
        .unwrap();

    assert_non_negative(result.value.amount(), "PV in result");
    assert_eq!(result.measures.len(), 2, "Should have 2 metrics");
    assert!(result.measures.contains_key("delta"));
    assert!(result.measures.contains_key("vega"));
}
