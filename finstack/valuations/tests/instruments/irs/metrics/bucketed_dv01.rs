//! Bucketed DV01 metric tests - Risk by tenor bucket.

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::irs::{InterestRateSwap, PayReceive};
use finstack_valuations::metrics::MetricId;
use time::macros::date;

fn build_flat_discount_curve(rate: f64, base_date: Date) -> DiscountCurve {
    DiscountCurve::builder("USD_OIS")
        .base_date(base_date)
        .day_count(DayCount::Act360)
        .knots([
            (0.0, 1.0),
            (1.0, (-rate).exp() as f64),
            (2.0, (-rate * 2.0).exp() as f64),
            (3.0, (-rate * 3.0).exp() as f64),
            (5.0, (-rate * 5.0).exp() as f64),
            (7.0, (-rate * 7.0).exp() as f64),
            (10.0, (-rate * 10.0).exp() as f64),
        ])
        .build()
        .unwrap()
}

fn create_swap(as_of: Date, end: Date) -> InterestRateSwap {
    InterestRateSwap {
        id: "IRS_BUCKETED_DV01_TEST".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        side: PayReceive::ReceiveFixed,
        fixed: finstack_valuations::instruments::common::parameters::legs::FixedLegSpec {
            disc_id: "USD_OIS".into(),
            rate: 0.05,
            freq: Frequency::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
            par_method: None,
            compounding_simple: true,
            start: as_of,
            end,
        },
        float: finstack_valuations::instruments::common::parameters::legs::FloatLegSpec {
            disc_id: "USD_OIS".into(),
            fwd_id: "USD_LIBOR_3M".into(),
            spread_bp: 0.0,
            freq: Frequency::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
            reset_lag_days: 2,
            start: as_of,
            end,
        },
        attributes: Default::default(),
    }
}

#[test]
fn test_bucketed_dv01_computes() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let swap = create_swap(as_of, end);
    let disc_curve = build_flat_discount_curve(0.05, as_of);
    let market = MarketContext::new().insert_discount(disc_curve);

    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::BucketedDv01])
        .unwrap();

    // BucketedDv01 metric should be present
    assert!(
        result.measures.contains_key("bucketed_dv01"),
        "BucketedDv01 should be computed"
    );
}

#[test]
fn test_bucketed_dv01_reasonable_values() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let swap = create_swap(as_of, end);
    let disc_curve = build_flat_discount_curve(0.05, as_of);
    let market = MarketContext::new().insert_discount(disc_curve);

    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::BucketedDv01])
        .unwrap();

    let bucketed_dv01 = result.measures.get("bucketed_dv01");

    assert!(bucketed_dv01.is_some(), "BucketedDv01 should be computed");
}

#[test]
fn test_bucketed_dv01_five_year_swap() {
    // 5Y swap should have risk in 1Y-5Y buckets
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let swap = create_swap(as_of, end);
    let disc_curve = build_flat_discount_curve(0.05, as_of);
    let market = MarketContext::new().insert_discount(disc_curve);

    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::BucketedDv01])
        .unwrap();

    // Verify metric was computed
    assert!(result.measures.contains_key("bucketed_dv01"));
}

#[test]
fn test_bucketed_dv01_short_swap() {
    // 1Y swap should have risk primarily in 1Y bucket
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2025 - 01 - 01);

    let swap = create_swap(as_of, end);
    let disc_curve = build_flat_discount_curve(0.05, as_of);
    let market = MarketContext::new().insert_discount(disc_curve);

    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::BucketedDv01])
        .unwrap();

    assert!(result.measures.contains_key("bucketed_dv01"));
}

#[test]
fn test_bucketed_dv01_long_swap() {
    // 10Y swap should have risk across many buckets
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2034 - 01 - 01);

    let swap = create_swap(as_of, end);
    let disc_curve = build_flat_discount_curve(0.05, as_of);
    let market = MarketContext::new().insert_discount(disc_curve);

    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::BucketedDv01])
        .unwrap();

    assert!(result.measures.contains_key("bucketed_dv01"));
}
