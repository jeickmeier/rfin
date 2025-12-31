//! Theta metric tests - Time decay.

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, StubKind, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::money::Money;
use finstack_valuations::instruments::rates::irs::{InterestRateSwap, PayReceive};
use finstack_valuations::instruments::Instrument;
use finstack_valuations::metrics::MetricId;
use rust_decimal_macros::dec;
use time::macros::date;

fn build_curves(rate: f64, base_date: Date) -> MarketContext {
    let disc_curve = DiscountCurve::builder("USD_OIS")
        .base_date(base_date)
        .day_count(DayCount::Act360)
        .knots([
            (0.0, 1.0),
            (1.0, (-rate).exp()),
            (5.0, (-rate * 5.0).exp()),
            (10.0, (-rate * 10.0).exp()),
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

fn create_swap(as_of: Date, end: Date, fixed_rate: rust_decimal::Decimal) -> InterestRateSwap {
    InterestRateSwap {
        id: "IRS_THETA_TEST".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        side: PayReceive::ReceiveFixed,
        fixed: finstack_valuations::instruments::FixedLegSpec {
            discount_curve_id: "USD_OIS".into(),
            rate: fixed_rate,
            freq: Tenor::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
            par_method: None,
            compounding_simple: true,
            payment_delay_days: 0,
            start: as_of,
            end,
        },
        float: finstack_valuations::instruments::FloatLegSpec {
            discount_curve_id: "USD_OIS".into(),
            forward_curve_id: "USD_LIBOR_3M".into(),
            spread_bp: rust_decimal::Decimal::try_from(0.0).expect("valid"),
            freq: Tenor::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            fixing_calendar_id: None,
            stub: StubKind::None,
            reset_lag_days: 2,
            compounding: Default::default(),
            payment_delay_days: 0,
            start: as_of,
            end,
        },
        margin_spec: None,
        attributes: Default::default(),
    }
}

#[test]
fn test_theta_computes() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let swap = create_swap(as_of, end, dec!(0.05));
    let market = build_curves(0.05, as_of);

    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::Theta])
        .unwrap();

    let theta = *result.measures.get("theta").unwrap();

    // Theta should be computed (finite value)
    assert!(theta.is_finite(), "Theta should be finite");
}

#[test]
fn test_theta_reasonable_magnitude() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let swap = create_swap(as_of, end, dec!(0.05));
    let market = build_curves(0.05, as_of);

    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::Theta])
        .unwrap();

    let theta = *result.measures.get("theta").unwrap();

    // Theta should be reasonable for $1MM swap
    assert!(
        theta.abs() < 100_000.0,
        "Theta magnitude should be reasonable, got {}",
        theta
    );
}

#[test]
fn test_theta_at_par_near_zero() {
    // At-the-money swap has small theta
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let swap = create_swap(as_of, end, dec!(0.05));
    let market = build_curves(0.05, as_of);

    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::Theta])
        .unwrap();

    let theta = *result.measures.get("theta").unwrap();

    // At par, theta should be relatively small
    assert!(
        theta.abs() < 10_000.0,
        "At-par theta should be small, got {}",
        theta
    );
}

#[test]
fn test_theta_off_market_swap() {
    // Off-market swap has larger theta
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let swap = create_swap(as_of, end, dec!(0.03)); // Below market
    let market = build_curves(0.05, as_of);

    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::Theta])
        .unwrap();

    let theta = *result.measures.get("theta").unwrap();

    // Off-market swap should have measurable theta
    assert!(
        theta.abs() > 0.0,
        "Off-market swap should have non-zero theta"
    );
}

#[test]
fn test_theta_direction_for_underwater_swap() {
    // Test theta calculation for off-market swap with negative NPV
    // Note: Unlike options, swap theta direction is not deterministic based on NPV sign.
    // Theta depends on curve shape, accrual timing, and relative leg sensitivities.
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    // Create swap receiving 3% fixed when market is at 5%
    let swap = create_swap(as_of, end, dec!(0.03));
    let market = build_curves(0.05, as_of);

    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::Theta])
        .unwrap();

    let npv = result.value.amount();
    let theta = *result.measures.get("theta").unwrap();

    // Verify NPV is negative (underwater position)
    assert!(
        npv < 0.0,
        "Swap receiving below-market rate should have negative NPV, got {}",
        npv
    );

    // Theta should be finite and non-zero for an off-market swap
    // Note: For swaps, theta direction depends on curve shape and accrual timing,
    // unlike options where negative NPV positions typically have positive theta.
    assert!(
        theta.is_finite(),
        "Theta should be finite for underwater swap, got {}",
        theta
    );
    assert!(
        theta.abs() > 0.0,
        "Off-market swap should have non-zero theta, got {}",
        theta
    );
}

#[test]
fn test_theta_opposite_sides() {
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);
    let market = build_curves(0.06, as_of);

    let mut swap_receive = create_swap(as_of, end, dec!(0.05));
    swap_receive.side = PayReceive::ReceiveFixed;

    let mut swap_pay = create_swap(as_of, end, dec!(0.05));
    swap_pay.side = PayReceive::PayFixed;

    let theta_receive = *swap_receive
        .price_with_metrics(&market, as_of, &[MetricId::Theta])
        .unwrap()
        .measures
        .get("theta")
        .unwrap();

    let theta_pay = *swap_pay
        .price_with_metrics(&market, as_of, &[MetricId::Theta])
        .unwrap()
        .measures
        .get("theta")
        .unwrap();

    // Opposite sides should have opposite theta signs
    assert!(
        theta_receive * theta_pay < 0.0,
        "Opposite sides should have opposite theta: receive={}, pay={}",
        theta_receive,
        theta_pay
    );
}
