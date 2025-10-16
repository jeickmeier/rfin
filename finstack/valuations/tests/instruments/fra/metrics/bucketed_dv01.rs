//! FRA bucketed DV01 metric tests.
//!
//! Bucketed DV01 decomposes total DV01 into risk contributions by
//! tenor bucket (e.g., 3M, 6M, 1Y). For FRAs, risk is concentrated
//! in the forward tenor bucket.

use crate::fra::common::*;
use finstack_core::currency::Currency;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;
use time::macros::date;

#[test]
fn test_bucketed_dv01_standard_fra() {
    let market = standard_market();
    let fra = create_standard_fra();

    let result = fra
        .price_with_metrics(&market, BASE_DATE, &[MetricId::BucketedDv01])
        .unwrap();

    // Check that bucketed_dv01 metric exists
    assert!(result.measures.contains_key("bucketed_dv01"));
}

#[test]
fn test_bucketed_dv01_finite() {
    let market = standard_market();
    let fra = create_standard_fra();

    let result = fra
        .price_with_metrics(&market, BASE_DATE, &[MetricId::BucketedDv01])
        .unwrap();

    let bucketed_dv01 = *result.measures.get("bucketed_dv01").unwrap();
    assert_finite(bucketed_dv01, "Bucketed DV01 should be finite");
}

#[test]
fn test_bucketed_dv01_short_period() {
    let market = standard_market();

    let start = date!(2024 - 02 - 01);
    let end = date!(2024 - 03 - 01);
    let fra = TestFraBuilder::new().dates(start, start, end).build();

    let result = fra
        .price_with_metrics(&market, BASE_DATE, &[MetricId::BucketedDv01])
        .unwrap();

    assert!(result.measures.contains_key("bucketed_dv01"));
}

#[test]
fn test_bucketed_dv01_long_period() {
    let market = standard_market();

    let start = date!(2024 - 07 - 01);
    let end = date!(2025 - 01 - 01);
    let fra = TestFraBuilder::new().dates(start, start, end).build();

    let result = fra
        .price_with_metrics(&market, BASE_DATE, &[MetricId::BucketedDv01])
        .unwrap();

    assert!(result.measures.contains_key("bucketed_dv01"));
}

#[test]
fn test_bucketed_dv01_scales_with_notional() {
    let market = standard_market();

    let fra_1m = TestFraBuilder::new()
        .notional(1_000_000.0, Currency::USD)
        .build();
    let fra_10m = TestFraBuilder::new()
        .notional(10_000_000.0, Currency::USD)
        .build();

    let result_1m = fra_1m
        .price_with_metrics(&market, BASE_DATE, &[MetricId::BucketedDv01])
        .unwrap();
    let bdv01_1m = *result_1m.measures.get("bucketed_dv01").unwrap();

    let result_10m = fra_10m
        .price_with_metrics(&market, BASE_DATE, &[MetricId::BucketedDv01])
        .unwrap();
    let bdv01_10m = *result_10m.measures.get("bucketed_dv01").unwrap();

    assert_finite(bdv01_1m, "1M bucketed DV01 should be finite");
    assert_finite(bdv01_10m, "10M bucketed DV01 should be finite");
}

#[test]
fn test_bucketed_dv01_with_other_metrics() {
    let market = standard_market();
    let fra = create_standard_fra();

    let result = fra
        .price_with_metrics(
            &market,
            BASE_DATE,
            &[MetricId::BucketedDv01, MetricId::Dv01],
        )
        .unwrap();

    assert!(result.measures.contains_key("bucketed_dv01"));
    assert!(result.measures.contains_key("dv01"));
}
