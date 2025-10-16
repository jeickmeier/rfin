//! PV Fixed metric tests - Present value of fixed leg.

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
            (1.0, (-rate).exp()),
            (5.0, (-rate * 5.0).exp()),
            (10.0, (-rate * 10.0).exp()),
        ])
        .build()
        .unwrap()
}

fn create_swap(as_of: Date, end: Date, fixed_rate: f64) -> InterestRateSwap {
    InterestRateSwap {
        id: "IRS_PV_FIXED_TEST".into(),
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
fn test_pv_fixed_positive() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let swap = create_swap(as_of, end, 0.05);
    let disc_curve = build_flat_discount_curve(0.05, as_of);
    let market = MarketContext::new().insert_discount(disc_curve);

    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::PvFixed])
        .unwrap();

    let pv_fixed = *result.measures.get("pv_fixed").unwrap();

    assert!(pv_fixed > 0.0, "PV fixed should be positive");
}

#[test]
fn test_pv_fixed_scales_with_rate() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let disc_curve = build_flat_discount_curve(0.05, as_of);
    let market = MarketContext::new().insert_discount(disc_curve);

    let swap_3pct = create_swap(as_of, end, 0.03);
    let swap_6pct = create_swap(as_of, end, 0.06);

    let pv_fixed_3pct = *swap_3pct
        .price_with_metrics(&market, as_of, &[MetricId::PvFixed])
        .unwrap()
        .measures
        .get("pv_fixed")
        .unwrap();

    let pv_fixed_6pct = *swap_6pct
        .price_with_metrics(&market, as_of, &[MetricId::PvFixed])
        .unwrap()
        .measures
        .get("pv_fixed")
        .unwrap();

    // 6% rate should give 2x PV of 3% rate
    let ratio = pv_fixed_6pct / pv_fixed_3pct;
    assert!(
        (ratio - 2.0).abs() < 0.1,
        "PV should scale with rate: ratio={}",
        ratio
    );
}

#[test]
fn test_pv_fixed_scales_with_notional() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let disc_curve = build_flat_discount_curve(0.05, as_of);
    let market = MarketContext::new().insert_discount(disc_curve);

    let swap_1m = create_swap(as_of, end, 0.05);

    let mut swap_5m = create_swap(as_of, end, 0.05);
    swap_5m.notional = Money::new(5_000_000.0, Currency::USD);

    let pv_fixed_1m = *swap_1m
        .price_with_metrics(&market, as_of, &[MetricId::PvFixed])
        .unwrap()
        .measures
        .get("pv_fixed")
        .unwrap();

    let pv_fixed_5m = *swap_5m
        .price_with_metrics(&market, as_of, &[MetricId::PvFixed])
        .unwrap()
        .measures
        .get("pv_fixed")
        .unwrap();

    let ratio = pv_fixed_5m / pv_fixed_1m;
    assert!(
        (ratio - 5.0).abs() < 0.01,
        "PV should scale with notional: ratio={}",
        ratio
    );
}

#[test]
fn test_pv_fixed_reasonable_magnitude() {
    // $1MM notional, 5% rate, 5 years
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let swap = create_swap(as_of, end, 0.05);
    let disc_curve = build_flat_discount_curve(0.05, as_of);
    let market = MarketContext::new().insert_discount(disc_curve);

    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::PvFixed])
        .unwrap();

    let pv_fixed = *result.measures.get("pv_fixed").unwrap();

    // Rough estimate: $1MM × 5% × 4.3 annuity ≈ $215,000
    assert!(
        pv_fixed > 100_000.0 && pv_fixed < 300_000.0,
        "PV fixed should be reasonable: got {}",
        pv_fixed
    );
}

#[test]
fn test_pv_fixed_independent_of_side() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let disc_curve = build_flat_discount_curve(0.05, as_of);
    let market = MarketContext::new().insert_discount(disc_curve);

    let mut swap_receive = create_swap(as_of, end, 0.05);
    swap_receive.side = PayReceive::ReceiveFixed;

    let mut swap_pay = create_swap(as_of, end, 0.05);
    swap_pay.side = PayReceive::PayFixed;

    let pv_fixed_receive = *swap_receive
        .price_with_metrics(&market, as_of, &[MetricId::PvFixed])
        .unwrap()
        .measures
        .get("pv_fixed")
        .unwrap();

    let pv_fixed_pay = *swap_pay
        .price_with_metrics(&market, as_of, &[MetricId::PvFixed])
        .unwrap()
        .measures
        .get("pv_fixed")
        .unwrap();

    assert!(
        (pv_fixed_receive - pv_fixed_pay).abs() < 1.0,
        "PV fixed should be independent of side"
    );
}
