//! Par rate metric tests.
//!
//! Tests par swap rate calculation: the fixed rate that makes NPV = 0.

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::money::Money;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::irs::{InterestRateSwap, PayReceive};
use finstack_valuations::metrics::MetricId;
use time::macros::date;

fn build_flat_curves(rate: f64, base_date: Date) -> MarketContext {
    let disc_curve = DiscountCurve::builder("USD_OIS")
        .base_date(base_date)
        .day_count(DayCount::Act360)
        .knots([
            (0.0, 1.0),
            (1.0, (-rate).exp() as f64),
            (5.0, (-rate * 5.0).exp() as f64),
            (10.0, (-rate * 10.0).exp() as f64),
        ])
        .build()
        .unwrap();

    let fwd_curve = ForwardCurve::builder("USD_LIBOR_3M", 0.25)
        .base_date(base_date)
        .day_count(DayCount::Act360)
        .knots([(0.0, rate), (10.0, rate)])
        .build()
        .unwrap();

    MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve)
}

fn create_standard_swap(as_of: Date, end: Date, fixed_rate: f64) -> InterestRateSwap {
    InterestRateSwap {
        id: "IRS_PAR_TEST".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        side: PayReceive::ReceiveFixed,
        fixed: finstack_valuations::instruments::common::parameters::legs::FixedLegSpec {
            disc_id: "USD_OIS".into(),
            rate: fixed_rate,
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
fn test_par_rate_flat_curve() {
    // In flat 5% environment, par rate should be ~5%
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let swap = create_standard_swap(as_of, end, 0.05);
    let market = build_flat_curves(0.05, as_of);

    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::ParRate])
        .unwrap();

    let par_rate = *result.measures.get("par_rate").unwrap();

    assert!(
        (par_rate - 0.05).abs() < 0.001,
        "Par rate should be ~5% in flat 5% curve, got {}",
        par_rate
    );
}

#[test]
fn test_par_rate_makes_npv_zero() {
    // Swap at par rate should have NPV ≈ 0
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let swap = create_standard_swap(as_of, end, 0.05);
    let market = build_flat_curves(0.05, as_of);

    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::ParRate])
        .unwrap();

    let par_rate = *result.measures.get("par_rate").unwrap();

    // Create swap at par rate
    let swap_at_par = create_standard_swap(as_of, end, par_rate);
    let npv = swap_at_par.value(&market, as_of).unwrap();

    assert!(
        npv.amount().abs() < 1000.0,
        "Swap at par rate should have NPV ≈ 0, got {}",
        npv.amount()
    );
}

#[test]
fn test_par_rate_positive() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let swap = create_standard_swap(as_of, end, 0.05);
    let market = build_flat_curves(0.05, as_of);

    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::ParRate])
        .unwrap();

    let par_rate = *result.measures.get("par_rate").unwrap();

    assert!(par_rate > 0.0, "Par rate should be positive");
}

#[test]
fn test_par_rate_independent_of_fixed_rate() {
    // Par rate shouldn't depend on the swap's fixed rate
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let market = build_flat_curves(0.05, as_of);

    let swap_3pct = create_standard_swap(as_of, end, 0.03);
    let swap_7pct = create_standard_swap(as_of, end, 0.07);

    let par_rate_3pct = *swap_3pct
        .price_with_metrics(&market, as_of, &[MetricId::ParRate])
        .unwrap()
        .measures
        .get("par_rate")
        .unwrap();

    let par_rate_7pct = *swap_7pct
        .price_with_metrics(&market, as_of, &[MetricId::ParRate])
        .unwrap()
        .measures
        .get("par_rate")
        .unwrap();

    assert!(
        (par_rate_3pct - par_rate_7pct).abs() < 0.001,
        "Par rate should be independent of swap fixed rate: {}% vs {}%",
        par_rate_3pct * 100.0,
        par_rate_7pct * 100.0
    );
}

#[test]
fn test_par_rate_independent_of_side() {
    // Par rate shouldn't depend on receive vs pay
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let market = build_flat_curves(0.05, as_of);

    let mut swap_receive = create_standard_swap(as_of, end, 0.05);
    swap_receive.side = PayReceive::ReceiveFixed;

    let mut swap_pay = create_standard_swap(as_of, end, 0.05);
    swap_pay.side = PayReceive::PayFixed;

    let par_rate_receive = *swap_receive
        .price_with_metrics(&market, as_of, &[MetricId::ParRate])
        .unwrap()
        .measures
        .get("par_rate")
        .unwrap();

    let par_rate_pay = *swap_pay
        .price_with_metrics(&market, as_of, &[MetricId::ParRate])
        .unwrap()
        .measures
        .get("par_rate")
        .unwrap();

    assert!(
        (par_rate_receive - par_rate_pay).abs() < 0.001,
        "Par rate should be independent of side"
    );
}

#[test]
fn test_par_rate_increases_with_forward_curve() {
    // Higher forward rates → higher par rate
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let swap = create_standard_swap(as_of, end, 0.05);

    let market_3pct = build_flat_curves(0.03, as_of);
    let market_7pct = build_flat_curves(0.07, as_of);

    let par_rate_3pct = *swap
        .price_with_metrics(&market_3pct, as_of, &[MetricId::ParRate])
        .unwrap()
        .measures
        .get("par_rate")
        .unwrap();

    let par_rate_7pct = *swap
        .price_with_metrics(&market_7pct, as_of, &[MetricId::ParRate])
        .unwrap()
        .measures
        .get("par_rate")
        .unwrap();

    assert!(
        par_rate_7pct > par_rate_3pct,
        "Higher forward curve should give higher par rate: 7%={}, 3%={}",
        par_rate_7pct,
        par_rate_3pct
    );
}

#[test]
fn test_par_rate_with_spread() {
    // Spread on float leg affects par rate
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let market = build_flat_curves(0.05, as_of);

    let swap_no_spread = create_standard_swap(as_of, end, 0.05);

    let mut swap_with_spread = create_standard_swap(as_of, end, 0.05);
    swap_with_spread.float.spread_bp = 50.0;

    let par_rate_no_spread = *swap_no_spread
        .price_with_metrics(&market, as_of, &[MetricId::ParRate])
        .unwrap()
        .measures
        .get("par_rate")
        .unwrap();

    let par_rate_with_spread = *swap_with_spread
        .price_with_metrics(&market, as_of, &[MetricId::ParRate])
        .unwrap()
        .measures
        .get("par_rate")
        .unwrap();

    assert!(
        par_rate_with_spread > par_rate_no_spread,
        "Spread should increase par rate: with={}%, without={}%",
        par_rate_with_spread * 100.0,
        par_rate_no_spread * 100.0
    );
}

#[test]
fn test_par_rate_short_vs_long() {
    // Par rate should be similar across maturities in flat curve
    let as_of = date!(2024 - 01 - 01);
    let market = build_flat_curves(0.05, as_of);

    let swap_2y = create_standard_swap(as_of, date!(2026 - 01 - 01), 0.05);
    let swap_10y = create_standard_swap(as_of, date!(2034 - 01 - 01), 0.05);

    let par_rate_2y = *swap_2y
        .price_with_metrics(&market, as_of, &[MetricId::ParRate])
        .unwrap()
        .measures
        .get("par_rate")
        .unwrap();

    let par_rate_10y = *swap_10y
        .price_with_metrics(&market, as_of, &[MetricId::ParRate])
        .unwrap()
        .measures
        .get("par_rate")
        .unwrap();

    // In flat curve, par rates should be similar
    assert!(
        (par_rate_2y - par_rate_10y).abs() < 0.01,
        "Par rates should be similar in flat curve: 2Y={}, 10Y={}",
        par_rate_2y,
        par_rate_10y
    );
}
