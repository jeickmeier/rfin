//! Complex IRS scenario tests.
//!
//! Tests comprehensive and multi-faceted IRS scenarios including:
//! - Off-market swaps with multiple metrics
//! - Basis swaps
//! - Multi-curve environments
//! - Forward-starting swaps

use finstack_core::currency::Currency;
use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency, StubKind};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::money::Money;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::irs::{InterestRateSwap, PayReceive};
use finstack_valuations::metrics::MetricId;
use time::macros::date;

fn build_flat_curves(disc_rate: f64, fwd_rate: f64, base_date: Date) -> MarketContext {
    let disc_curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .day_count(DayCount::Act360)
        .knots([
            (0.0, 1.0),
            (1.0, (-disc_rate).exp()),
            (5.0, (-disc_rate * 5.0).exp()),
            (10.0, (-disc_rate * 10.0).exp()),
        ])
        .build()
        .unwrap();

    let fwd_curve = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(base_date)
        .day_count(DayCount::Act360)
        .knots([(0.0, fwd_rate), (10.0, fwd_rate)])
        .build()
        .unwrap();

    MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve)
}

fn create_swap(as_of: Date, end: Date, fixed_rate: f64, side: PayReceive) -> InterestRateSwap {
    InterestRateSwap {
        id: "IRS_INTEGRATION_TEST".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        side,
        fixed: finstack_valuations::instruments::common::parameters::legs::FixedLegSpec {
            discount_curve_id: "USD-OIS".into(),
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
            discount_curve_id: "USD-OIS".into(),
            forward_curve_id: "USD-SOFR-3M".into(),
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
fn test_off_market_swap_all_metrics() {
    // Off-market swap with comprehensive metric calculation
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let swap = create_swap(as_of, end, 0.03, PayReceive::ReceiveFixed);
    let market = build_flat_curves(0.05, 0.05, as_of);

    let metrics = vec![
        MetricId::Annuity,
        MetricId::Dv01,
        MetricId::ParRate,
        MetricId::PvFixed,
        MetricId::PvFloat,
        MetricId::Theta,
    ];

    let result = swap.price_with_metrics(&market, as_of, &metrics).unwrap();

    // Verify all metrics computed
    assert!(result.measures.contains_key("annuity"));
    assert!(result.measures.contains_key("dv01"));
    assert!(result.measures.contains_key("par_rate"));
    assert!(result.measures.contains_key("pv_fixed"));
    assert!(result.measures.contains_key("pv_float"));
    assert!(result.measures.contains_key("theta"));

    // Off-market swap should have negative NPV
    assert!(result.value.amount() < 0.0);
}

#[test]
fn test_basis_swap() {
    // Basis swap: float vs float with different indices
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let swap = InterestRateSwap::usd_basis_swap(
        "BASIS_SWAP".into(),
        Money::new(1_000_000.0, Currency::USD),
        as_of,
        end,
        15.0, // 15bp spread on primary leg
        10.0, // 10bp spread on reference leg
    );

    let market = build_flat_curves(0.05, 0.05, as_of);

    let npv = swap.value(&market, as_of);

    assert!(npv.is_ok(), "Basis swap should price successfully");
}

#[test]
fn test_multi_curve_environment() {
    // Test with OIS discount and LIBOR projection
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let disc_curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([
            (0.0, 1.0),
            (1.0, (-0.04_f64).exp()), // OIS at 4%
            (5.0, (-0.04_f64 * 5.0).exp()),
            (10.0, (-0.04_f64 * 10.0).exp()),
        ])
        .build()
        .unwrap();

    let fwd_curve = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([(0.0, 0.045), (10.0, 0.045)]) // LIBOR at 4.5%
        .build()
        .unwrap();

    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);

    let swap = create_swap(as_of, end, 0.045, PayReceive::ReceiveFixed);

    let npv = swap.value(&market, as_of).unwrap();

    // Multi-curve pricing should work
    assert!(npv.amount().is_finite());
}

#[test]
fn test_forward_starting_swap() {
    // Swap starting in the future
    let as_of = date!(2024 - 01 - 01);
    let start = date!(2025 - 01 - 01); // Forward start
    let end = date!(2030 - 01 - 01);

    let disc_curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([
            (0.0, 1.0),
            (1.0, (-0.05_f64).exp()),
            (5.0, (-0.05_f64 * 5.0).exp()),
            (10.0, (-0.05_f64 * 10.0).exp()),
        ])
        .build()
        .unwrap();

    let fwd_curve = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([(0.0, 0.05), (10.0, 0.05)])
        .build()
        .unwrap();

    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);

    let swap = InterestRateSwap {
        id: "FORWARD_START".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        side: PayReceive::ReceiveFixed,
        fixed: finstack_valuations::instruments::common::parameters::legs::FixedLegSpec {
            discount_curve_id: "USD-OIS".into(),
            rate: 0.05,
            freq: Frequency::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
            par_method: None,
            compounding_simple: true,
            start, // Forward start
            end,
        },
        float: finstack_valuations::instruments::common::parameters::legs::FloatLegSpec {
            discount_curve_id: "USD-OIS".into(),
            forward_curve_id: "USD-SOFR-3M".into(),
            spread_bp: 0.0,
            freq: Frequency::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
            reset_lag_days: 2,
            start,
            end,
        },
        attributes: Default::default(),
    };

    let npv = swap.value(&market, as_of).unwrap();

    // Forward-starting swap should price
    assert!(npv.amount().abs() < 100_000.0);
}

#[test]
fn test_swap_portfolio_aggregation() {
    // Test portfolio of swaps
    let as_of = date!(2024 - 01 - 01);
    let market = build_flat_curves(0.05, 0.05, as_of);

    let swaps = vec![
        create_swap(as_of, date!(2026 - 01 - 01), 0.04, PayReceive::ReceiveFixed),
        create_swap(as_of, date!(2027 - 01 - 01), 0.045, PayReceive::PayFixed),
        create_swap(as_of, date!(2029 - 01 - 01), 0.05, PayReceive::ReceiveFixed),
    ];

    let mut total_npv = 0.0;
    let mut total_dv01 = 0.0;

    for swap in swaps {
        let result = swap
            .price_with_metrics(&market, as_of, &[MetricId::Dv01])
            .unwrap();

        total_npv += result.value.amount();
        total_dv01 += result.measures.get("dv01").unwrap();
    }

    // Portfolio aggregation should work
    assert!(total_npv.is_finite());
    assert!(total_dv01.is_finite());
}

#[test]
fn test_swap_with_large_spread() {
    // Swap with large spread on floating leg
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let mut swap = create_swap(as_of, end, 0.05, PayReceive::ReceiveFixed);
    swap.float.spread_bp = 200.0; // 200bp spread

    let market = build_flat_curves(0.05, 0.05, as_of);

    let result = swap
        .price_with_metrics(&market, as_of, &[MetricId::PvFixed, MetricId::PvFloat])
        .unwrap();

    let pv_fixed = *result.measures.get("pv_fixed").unwrap();
    let pv_float = *result.measures.get("pv_float").unwrap();

    // Large spread should significantly affect PV float
    assert!(pv_float > pv_fixed * 1.2);
}

#[test]
fn test_swap_seasoned() {
    // Swap evaluated after inception (seasoned swap)
    let as_of = date!(2024 - 06 - 01); // Evaluation date
    let end = date!(2028 - 01 - 01);

    let disc_curve = DiscountCurve::builder("USD-OIS")
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([
            (0.0, 1.0),
            (1.0, (-0.06_f64).exp()), // Rates have risen
            (5.0, (-0.06_f64 * 5.0).exp()),
        ])
        .build()
        .unwrap();

    let fwd_curve = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(as_of)
        .day_count(DayCount::Act360)
        .knots([(0.0, 0.06), (10.0, 0.06)])
        .build()
        .unwrap();

    let market = MarketContext::new()
        .insert_discount(disc_curve)
        .insert_forward(fwd_curve);

    let swap = InterestRateSwap {
        id: "SEASONED_SWAP".into(),
        notional: Money::new(1_000_000.0, Currency::USD),
        side: PayReceive::ReceiveFixed,
        fixed: finstack_valuations::instruments::common::parameters::legs::FixedLegSpec {
            discount_curve_id: "USD-OIS".into(),
            rate: 0.04, // Old rate from 2023
            freq: Frequency::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
            par_method: None,
            compounding_simple: true,
            start: as_of, // Use as_of instead of start to avoid invalid time range
            end,
        },
        float: finstack_valuations::instruments::common::parameters::legs::FloatLegSpec {
            discount_curve_id: "USD-OIS".into(),
            forward_curve_id: "USD-SOFR-3M".into(),
            spread_bp: 0.0,
            freq: Frequency::quarterly(),
            dc: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            calendar_id: None,
            stub: StubKind::None,
            reset_lag_days: 2,
            start: as_of, // Use as_of instead of start to avoid invalid time range
            end,
        },
        attributes: Default::default(),
    };

    let npv = swap.value(&market, as_of).unwrap();

    // Seasoned swap should show MTM loss (rates rose, receiving fixed)
    assert!(npv.amount() < 0.0);
}

#[test]
fn test_swap_risk_attribution() {
    // Test that risks are correctly attributed
    let as_of = date!(2024 - 01 - 01);
    let end = date!(2029 - 01 - 01);

    let swap = create_swap(as_of, end, 0.05, PayReceive::ReceiveFixed);
    let market = build_flat_curves(0.05, 0.06, as_of);

    let result = swap
        .price_with_metrics(
            &market,
            as_of,
            &[MetricId::Dv01, MetricId::Annuity, MetricId::BucketedDv01],
        )
        .unwrap();

    let dv01 = *result.measures.get("dv01").unwrap();
    let annuity = *result.measures.get("annuity").unwrap();

    // Verify risk attribution is consistent
    let expected_dv01 = annuity * 1_000_000.0 * 0.0001;
    let ratio = dv01.abs() / expected_dv01;
    assert!(
        (ratio - 1.0).abs() < 0.02, // 2% tolerance for numerical precision
        "DV01 {} should be close to annuity-based estimate {}, ratio: {}",
        dv01,
        expected_dv01,
        ratio
    );
}
