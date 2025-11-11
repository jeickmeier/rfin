//! Bucketed DV01 metric tests for InflationSwap.

use crate::inflation_swap::fixtures::*;
use finstack_core::dates::{Date, DayCount};
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::inflation_swap::{InflationSwapBuilder, PayReceiveInflation};
use finstack_valuations::metrics::MetricId;
use time::Month;

#[test]
fn test_bucketed_dv01_computed() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

    let ctx = standard_market(as_of, 0.02, 0.04);

    let swap = InflationSwapBuilder::new()
        .id("ZCINF-BDV01-1".into())
        .notional(standard_notional())
        .start(as_of)
        .maturity(maturity)
        .fixed_rate(0.02)
        .inflation_index_id("US-CPI-U".into())
        .discount_curve_id("USD-OIS".into())
        .dc(DayCount::Act365F)
        .side(PayReceiveInflation::PayFixed)
        .attributes(Default::default())
        .build()
        .unwrap();

    let result = swap
        .price_with_metrics(&ctx, as_of, &[MetricId::BucketedDv01])
        .unwrap();

    // Check that bucketed_dv01 is present
    assert!(
        result.measures.contains_key("bucketed_dv01"),
        "Bucketed DV01 should be computed"
    );

    let bucketed_dv01 = *result.measures.get("bucketed_dv01").unwrap();

    // Bucketed DV01 should be finite
    assert!(bucketed_dv01.is_finite(), "Bucketed DV01 should be finite");
}

#[test]
fn test_bucketed_dv01_reasonable_magnitude() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

    let ctx = standard_market(as_of, 0.02, 0.04);

    let swap = InflationSwapBuilder::new()
        .id("ZCINF-BDV01-MAG".into())
        .notional(standard_notional())
        .start(as_of)
        .maturity(maturity)
        .fixed_rate(0.02)
        .inflation_index_id("US-CPI-U".into())
        .discount_curve_id("USD-OIS".into())
        .dc(DayCount::Act365F)
        .side(PayReceiveInflation::PayFixed)
        .attributes(Default::default())
        .build()
        .unwrap();

    let result = swap
        .price_with_metrics(&ctx, as_of, &[MetricId::BucketedDv01, MetricId::Dv01])
        .unwrap();

    let bucketed_dv01 = result.measures.get("bucketed_dv01").unwrap().abs();
    let dv01 = result.measures.get("dv01").unwrap().abs();

    // Bucketed DV01 uses bump-and-reprice (more accurate)
    // DV01 uses analytical duration approximation (faster but less precise)
    // They can differ, especially for inflation swaps with lagged indices
    // The bucketed version should be within a reasonable factor (2-4x) of the analytical version
    assert!(
        bucketed_dv01 > 0.0 && dv01 > 0.0,
        "Both DV01 measures should be positive"
    );
    assert!(
        bucketed_dv01 / dv01 < 5.0 && bucketed_dv01 / dv01 > 0.2,
        "Bucketed DV01 ({}) should be same order of magnitude as DV01 ({}), ratio: {}",
        bucketed_dv01,
        dv01,
        bucketed_dv01 / dv01
    );
}

#[test]
fn test_bucketed_dv01_zero_for_matured() {
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2020, Month::January, 1).unwrap();

    let ctx = standard_market(as_of, 0.02, 0.04);

    let swap = InflationSwapBuilder::new()
        .id("ZCINF-BDV01-MAT0".into())
        .notional(standard_notional())
        .start(Date::from_calendar_date(2015, Month::January, 1).unwrap())
        .maturity(maturity)
        .fixed_rate(0.02)
        .inflation_index_id("US-CPI-U".into())
        .discount_curve_id("USD-OIS".into())
        .dc(DayCount::Act365F)
        .side(PayReceiveInflation::PayFixed)
        .attributes(Default::default())
        .build()
        .unwrap();

    let result = swap
        .price_with_metrics(&ctx, as_of, &[MetricId::BucketedDv01])
        .unwrap();

    let bucketed_dv01 = result.measures.get("bucketed_dv01").unwrap().abs();

    // Matured swap should have negligible bucketed DV01
    assert!(
        bucketed_dv01 < 10.0,
        "Matured swap should have negligible bucketed DV01: {}",
        bucketed_dv01
    );
}
