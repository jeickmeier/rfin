//! Edge case and boundary condition tests for basis swaps.
//!
//! Tests validate robustness across extreme market conditions, unusual
//! instrument configurations, and boundary scenarios.

use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Frequency};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_core::{currency::Currency::USD, math::interp::InterpStyle};
use finstack_valuations::cashflow::builder::ScheduleParams;
use finstack_valuations::instruments::basis_swap::{BasisSwap, BasisSwapLeg};
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::metrics::MetricId;
use time::Month;

fn d(y: i32, m: u8, day: u8) -> Date {
    Date::from_calendar_date(y, Month::try_from(m).unwrap(), day).unwrap()
}

fn market() -> MarketContext {
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 1.0), (0.5, 0.99), (1.0, 0.98), (5.0, 0.90)])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap();
    let f3m = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 0.02), (0.5, 0.021), (1.0, 0.022), (5.0, 0.025)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();
    let f1m = ForwardCurve::builder("USD-SOFR-1M", 1.0 / 12.0)
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 0.019), (0.5, 0.020), (1.0, 0.021), (5.0, 0.024)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();
    MarketContext::new()
        .insert_discount(disc)
        .insert_forward(f3m)
        .insert_forward(f1m)
}

#[test]
fn zero_notional() {
    // Test swap with zero notional
    let ctx = market();
    let as_of = d(2025, 1, 2);
    let _sched = ScheduleParams::quarterly_act360();

    let swap = BasisSwap::new(
        "ZERO-NOTIONAL",
        Money::new(0.0, USD),
        d(2025, 1, 2),
        d(2026, 1, 2),
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    );

    let res = swap
        .price_with_metrics(
            &ctx,
            as_of,
            &[
                MetricId::Dv01,
                MetricId::PvPrimary,
                MetricId::PvReference,
                MetricId::BasisParSpread,
            ],
        )
        .unwrap();

    // All metrics should be zero or finite
    assert_eq!(res.measures[MetricId::Dv01.as_str()], 0.0);
    assert_eq!(res.measures[MetricId::PvPrimary.as_str()], 0.0);
    assert_eq!(res.measures[MetricId::PvReference.as_str()], 0.0);
    // Par spread may be NaN/Inf with zero notional (division by zero), so we skip the finite check
}

#[test]
fn very_small_notional() {
    // Test swap with very small notional
    let ctx = market();
    let as_of = d(2025, 1, 2);
    let _sched = ScheduleParams::quarterly_act360();

    let swap = BasisSwap::new(
        "SMALL-NOTIONAL",
        Money::new(1.0, USD),
        d(2025, 1, 2),
        d(2026, 1, 2),
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    );

    let npv = swap.value(&ctx, as_of).unwrap();
    assert!(npv.amount().is_finite());
    assert!(npv.amount().abs() < 10.0); // Should be very small
}

#[test]
fn very_large_notional() {
    // Test swap with very large notional
    let ctx = market();
    let as_of = d(2025, 1, 2);
    let _sched = ScheduleParams::quarterly_act360();

    let swap = BasisSwap::new(
        "LARGE-NOTIONAL",
        Money::new(1_000_000_000_000.0, USD),
        d(2025, 1, 2),
        d(2026, 1, 2),
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    );

    let res = swap
        .price_with_metrics(&ctx, as_of, &[MetricId::Dv01Primary])
        .unwrap();
    let dv01 = res.measures[MetricId::Dv01Primary.as_str()];

    assert!(dv01.is_finite());
    assert!(
        dv01 > 1_000_000.0,
        "Large notional should have substantial DV01: got {}",
        dv01
    ); // Should be substantial
}

#[test]
fn very_short_maturity() {
    // Test swap with very short maturity (1 month)
    let ctx = market();
    let as_of = d(2025, 1, 2);
    let _sched = ScheduleParams::quarterly_act360();

    let swap = BasisSwap::new(
        "SHORT-MAT",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2025, 2, 2),
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    );

    let npv = swap.value(&ctx, as_of);
    assert!(npv.is_ok(), "Short maturity swap should price");
}

#[test]
fn very_long_maturity() {
    // Test swap with very long maturity (30 years)
    let ctx = market();
    let as_of = d(2025, 1, 2);
    let _sched = ScheduleParams::quarterly_act360();

    let swap = BasisSwap::new(
        "LONG-MAT",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2055, 1, 2),
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    );

    let res = swap
        .price_with_metrics(&ctx, as_of, &[MetricId::AnnuityPrimary])
        .unwrap();
    let annuity = res.measures[MetricId::AnnuityPrimary.as_str()];

    assert!(annuity.is_finite());
    assert!(annuity > 0.0);
}

#[test]
fn extreme_positive_spread() {
    // Test with extremely large positive spread (1000bp)
    let ctx = market();
    let as_of = d(2025, 1, 2);
    let _sched = ScheduleParams::quarterly_act360();

    let swap = BasisSwap::new(
        "EXTREME-SPREAD",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2026, 1, 2),
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.10, // 1000bp spread
        },
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    );

    let npv = swap.value(&ctx, as_of).unwrap();
    assert!(npv.amount().is_finite());
    assert!(npv.amount() > 500_000.0); // Should be significantly positive
}

#[test]
fn extreme_negative_spread() {
    // Test with extremely large negative spread (-1000bp)
    let ctx = market();
    let as_of = d(2025, 1, 2);
    let _sched = ScheduleParams::quarterly_act360();

    let swap = BasisSwap::new(
        "EXTREME-NEG-SPREAD",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2026, 1, 2),
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: -0.10, // -1000bp spread
        },
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    );

    let npv = swap.value(&ctx, as_of).unwrap();
    assert!(npv.amount().is_finite());
    assert!(npv.amount() < -500_000.0); // Should be significantly negative
}

#[test]
fn flat_curves_zero_rates() {
    // Test with flat curves at zero rates
    // NOTE: Flat curves require allow_non_monotonic() since monotonicity is enforced by default
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 1.0), (1.0, 1.0), (2.0, 1.0)])
        .set_interp(InterpStyle::LogLinear)
        .allow_non_monotonic()
        .build()
        .unwrap();
    let f3m = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 1e-6), (1.0, 1e-6), (2.0, 1e-6)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();
    let f1m = ForwardCurve::builder("USD-SOFR-1M", 1.0 / 12.0)
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 1e-6), (1.0, 1e-6), (2.0, 1e-6)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_forward(f3m)
        .insert_forward(f1m);

    let _sched = ScheduleParams::quarterly_act360();
    let swap = BasisSwap::new(
        "ZERO-RATES",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2026, 1, 2),
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    );

    let npv = swap.value(&ctx, d(2025, 1, 2)).unwrap();
    assert!(npv.amount().abs() < 1.0); // Should be essentially zero
}

#[test]
fn negative_rates() {
    // Test with negative interest rates (European scenario)
    // NOTE: Increasing DFs (negative rates) require allow_non_monotonic()
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 1.0), (1.0, 1.005), (2.0, 1.01)])
        .set_interp(InterpStyle::LogLinear)
        .allow_non_monotonic() // Allow increasing DFs for negative rate test
        .build()
        .unwrap();
    let f3m = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 1e-6), (1.0, 2e-6), (2.0, 3e-6)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();
    let f1m = ForwardCurve::builder("USD-SOFR-1M", 1.0 / 12.0)
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 1e-6), (1.0, 2e-6), (2.0, 3e-6)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_forward(f3m)
        .insert_forward(f1m);

    let _sched = ScheduleParams::quarterly_act360();
    let swap = BasisSwap::new(
        "NEG-RATES",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2026, 1, 2),
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    );

    let res = swap
        .price_with_metrics(
            &ctx,
            d(2025, 1, 2),
            &[MetricId::PvPrimary, MetricId::PvReference],
        )
        .unwrap();

    // Should handle negative rates without panic
    assert!(res.measures[MetricId::PvPrimary.as_str()].is_finite());
    assert!(res.measures[MetricId::PvReference.as_str()].is_finite());
}

#[test]
fn valuation_at_maturity() {
    // Test valuation when as_of date equals maturity
    let ctx = market();
    let maturity = d(2026, 1, 2);
    let _sched = ScheduleParams::quarterly_act360();

    let swap = BasisSwap::new(
        "AT-MATURITY",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        maturity,
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    );

    let npv = swap.value(&ctx, maturity).unwrap();
    // At maturity, NPV should be zero or very small
    assert!(npv.amount().abs() < 100.0);
}

#[test]
fn valuation_after_maturity() {
    // Test valuation after maturity date
    let ctx = market();
    let maturity = d(2026, 1, 2);
    let after_maturity = d(2026, 6, 1);
    let _sched = ScheduleParams::quarterly_act360();

    let swap = BasisSwap::new(
        "AFTER-MATURITY",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        maturity,
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    );

    let npv = swap.value(&ctx, after_maturity).unwrap();
    // After maturity, all cashflows are in the past, NPV should be zero
    assert!(npv.amount().abs() < 1.0);
}

#[test]
fn identical_forward_curves() {
    // Test when both legs reference the same forward curve
    let ctx = market();
    let as_of = d(2025, 1, 2);
    let _sched = ScheduleParams::quarterly_act360();

    let swap = BasisSwap::new(
        "IDENTICAL-CURVES",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2026, 1, 2),
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    );

    let res = swap
        .price_with_metrics(
            &ctx,
            as_of,
            &[
                MetricId::PvPrimary,
                MetricId::PvReference,
                MetricId::BasisParSpread,
            ],
        )
        .unwrap();

    let pv_primary = res.measures[MetricId::PvPrimary.as_str()];
    let pv_reference = res.measures[MetricId::PvReference.as_str()];
    let par_spread = res.measures[MetricId::BasisParSpread.as_str()];

    // If same curve and same frequency/daycount, PVs should be equal
    assert!(
        (pv_primary - pv_reference).abs() < 1.0,
        "PVs should match for identical curves"
    );
    // Par spread should be near zero
    assert!(
        par_spread.abs() < 0.1,
        "Par spread should be near zero for identical curves"
    );
}

#[test]
fn steep_curve() {
    // Test with very steep yield curve
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 1.0), (0.5, 0.95), (1.0, 0.85), (2.0, 0.70)])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap();
    let f3m = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 0.01), (0.5, 0.05), (1.0, 0.10), (2.0, 0.15)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();
    let f1m = ForwardCurve::builder("USD-SOFR-1M", 1.0 / 12.0)
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 0.01), (0.5, 0.04), (1.0, 0.09), (2.0, 0.14)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_forward(f3m)
        .insert_forward(f1m);

    let _sched = ScheduleParams::quarterly_act360();
    let swap = BasisSwap::new(
        "STEEP-CURVE",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2027, 1, 2),
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    );

    let res = swap
        .price_with_metrics(&ctx, d(2025, 1, 2), &[MetricId::Dv01])
        .unwrap();

    assert!(res.measures[MetricId::Dv01.as_str()].is_finite());
}
