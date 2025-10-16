//! FRA PV metric tests.
//!
//! The PV metric is a passthrough that returns the base value from
//! the pricing engine. These tests verify it's consistent with
//! direct value() calls.

use crate::fra::common::*;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;

#[test]
fn test_pv_metric_matches_value() {
    let market = standard_market();
    let fra = TestFraBuilder::new().fixed_rate(0.06).build();

    let direct_pv = fra.value(&market, BASE_DATE).unwrap();
    let result = fra
        .price_with_metrics(&market, BASE_DATE, &[MetricId::custom("fra_pv")])
        .unwrap();

    let metric_pv = *result.measures.get("fra_pv").unwrap();

    assert_approx_equal(
        metric_pv,
        direct_pv.amount(),
        0.01,
        "PV metric should match direct value()",
    );
}

#[test]
fn test_pv_metric_at_market() {
    let market = standard_market();
    let fra = create_standard_fra();

    let result = fra
        .price_with_metrics(&market, BASE_DATE, &[MetricId::custom("fra_pv")])
        .unwrap();

    let pv = *result.measures.get("fra_pv").unwrap();

    assert_near_zero(pv, 1000.0, "At-market FRA PV should be near zero");
}

#[test]
fn test_pv_metric_off_market_positive() {
    let market = standard_market();
    let fra = TestFraBuilder::new()
        .fixed_rate(0.06) // above market
        .pay_fixed(true) // true = receive fixed
        .build();

    let result = fra
        .price_with_metrics(&market, BASE_DATE, &[MetricId::custom("fra_pv")])
        .unwrap();

    let pv = *result.measures.get("fra_pv").unwrap();

    assert_positive(pv, "Above-market receive-fixed should have positive PV");
}

#[test]
fn test_pv_metric_off_market_negative() {
    let market = standard_market();
    let fra = TestFraBuilder::new()
        .fixed_rate(0.04) // below market
        .pay_fixed(true) // true = receive fixed
        .build();

    let result = fra
        .price_with_metrics(&market, BASE_DATE, &[MetricId::custom("fra_pv")])
        .unwrap();

    let pv = *result.measures.get("fra_pv").unwrap();

    assert_negative(pv, "Below-market receive-fixed should have negative PV");
}

#[test]
fn test_pv_metric_with_other_metrics() {
    let market = standard_market();
    let fra = TestFraBuilder::new().fixed_rate(0.06).build();

    let result = fra
        .price_with_metrics(
            &market,
            BASE_DATE,
            &[MetricId::custom("fra_pv"), MetricId::Dv01],
        )
        .unwrap();

    assert!(result.measures.contains_key("fra_pv"));
    assert!(result.measures.contains_key("dv01"));
}
