//! Edge case and boundary condition tests for basis swaps.
//!
//! Tests validate robustness across extreme market conditions, unusual
//! instrument configurations, and boundary scenarios.

use finstack_core::dates::{BusinessDayConvention, Date, DayCount, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_core::{currency::Currency::USD, math::interp::InterpStyle};
use finstack_valuations::instruments::rates::basis_swap::{BasisSwap, BasisSwapLeg};
use finstack_valuations::instruments::Instrument;
use finstack_valuations::metrics::MetricId;
use time::Month;

fn d(y: i32, m: u8, day: u8) -> Date {
    Date::from_calendar_date(y, Month::try_from(m).unwrap(), day).unwrap()
}

const CALENDAR_ID: &str = "usny";

fn market() -> MarketContext {
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 1.0), (0.5, 0.99), (1.0, 0.98), (5.0, 0.90)])
        .interp(InterpStyle::LogLinear)
        .build()
        .unwrap();
    let f3m = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 0.02), (0.5, 0.021), (1.0, 0.022), (5.0, 0.025)])
        .interp(InterpStyle::Linear)
        .build()
        .unwrap();
    let f1m = ForwardCurve::builder("USD-SOFR-1M", 1.0 / 12.0)
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 0.019), (0.5, 0.020), (1.0, 0.021), (5.0, 0.024)])
        .interp(InterpStyle::Linear)
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

    let swap = BasisSwap::new(
        "ZERO-NOTIONAL",
        Money::new(0.0, USD),
        d(2025, 1, 2),
        d(2026, 1, 2),
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    )
    .expect("swap construction")
    .with_calendar(CALENDAR_ID);

    // Request metrics that work with zero notional (not BasisParSpread, which now
    // correctly returns an error for zero notional to avoid NaN/Inf)
    let res = swap
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::Dv01, MetricId::PvPrimary, MetricId::PvReference],
        )
        .unwrap();

    // All metrics should be zero for zero notional
    assert_eq!(res.measures[MetricId::Dv01.as_str()], 0.0);
    assert_eq!(res.measures[MetricId::PvPrimary.as_str()], 0.0);
    assert_eq!(res.measures[MetricId::PvReference.as_str()], 0.0);

    // BasisParSpread should return an explicit error for zero notional (tested separately)
    let par_spread_result = swap.price_with_metrics(&ctx, as_of, &[MetricId::BasisParSpread]);
    assert!(
        par_spread_result.is_err(),
        "BasisParSpread should error for zero notional"
    );
}

#[test]
fn very_small_notional() {
    // Test swap with very small notional
    let ctx = market();
    let as_of = d(2025, 1, 2);

    let swap = BasisSwap::new(
        "SMALL-NOTIONAL",
        Money::new(1.0, USD),
        d(2025, 1, 2),
        d(2026, 1, 2),
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    )
    .expect("swap construction")
    .with_calendar(CALENDAR_ID);

    let npv = swap.value(&ctx, as_of).unwrap();
    assert!(npv.amount().is_finite());
    assert!(npv.amount().abs() < 10.0); // Should be very small
}

#[test]
fn very_large_notional() {
    // Test swap with very large notional
    let ctx = market();
    let as_of = d(2025, 1, 2);

    let swap = BasisSwap::new(
        "LARGE-NOTIONAL",
        Money::new(1_000_000_000_000.0, USD),
        d(2025, 1, 2),
        d(2026, 1, 2),
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    )
    .expect("swap construction")
    .with_calendar(CALENDAR_ID);

    let res = swap
        .price_with_metrics(&ctx, as_of, &[MetricId::Dv01])
        .unwrap();

    // Extract primary forward curve DV01 from measures using composite key (note: sanitized with underscores)
    let dv01 = res
        .measures
        .get("bucketed_dv01::usd_sofr_3m")
        .copied()
        .unwrap_or(0.0);

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

    let swap = BasisSwap::new(
        "SHORT-MAT",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2025, 2, 2),
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    )
    .expect("swap construction")
    .with_calendar(CALENDAR_ID);

    let npv = swap.value(&ctx, as_of);
    assert!(npv.is_ok(), "Short maturity swap should price");
}

#[test]
fn very_long_maturity() {
    // Test swap with very long maturity (30 years)
    let ctx = market();
    let as_of = d(2025, 1, 2);

    let swap = BasisSwap::new(
        "LONG-MAT",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2055, 1, 2),
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    )
    .expect("swap construction")
    .with_calendar(CALENDAR_ID);

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

    let swap = BasisSwap::new(
        "EXTREME-SPREAD",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2026, 1, 2),
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.10, // 1000bp spread
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    )
    .expect("swap construction")
    .with_calendar(CALENDAR_ID);

    let npv = swap.value(&ctx, as_of).unwrap();
    assert!(npv.amount().is_finite());
    assert!(npv.amount() > 500_000.0); // Should be significantly positive
}

#[test]
fn extreme_negative_spread() {
    // Test with extremely large negative spread (-1000bp)
    let ctx = market();
    let as_of = d(2025, 1, 2);

    let swap = BasisSwap::new(
        "EXTREME-NEG-SPREAD",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2026, 1, 2),
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: -0.10, // -1000bp spread
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    )
    .expect("swap construction")
    .with_calendar(CALENDAR_ID);

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
        .interp(InterpStyle::LogLinear)
        .allow_non_monotonic()
        .build()
        .unwrap();
    let f3m = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 1e-6), (1.0, 1e-6), (2.0, 1e-6)])
        .interp(InterpStyle::Linear)
        .build()
        .unwrap();
    let f1m = ForwardCurve::builder("USD-SOFR-1M", 1.0 / 12.0)
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 1e-6), (1.0, 1e-6), (2.0, 1e-6)])
        .interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_forward(f3m)
        .insert_forward(f1m);

    let swap = BasisSwap::new(
        "ZERO-RATES",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2026, 1, 2),
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    )
    .expect("swap construction")
    .with_calendar(CALENDAR_ID);

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
        .interp(InterpStyle::LogLinear)
        .allow_non_monotonic() // Allow increasing DFs for negative rate test
        .build()
        .unwrap();
    let f3m = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 1e-6), (1.0, 2e-6), (2.0, 3e-6)])
        .interp(InterpStyle::Linear)
        .build()
        .unwrap();
    let f1m = ForwardCurve::builder("USD-SOFR-1M", 1.0 / 12.0)
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 1e-6), (1.0, 2e-6), (2.0, 3e-6)])
        .interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_forward(f3m)
        .insert_forward(f1m);

    let swap = BasisSwap::new(
        "NEG-RATES",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2026, 1, 2),
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    )
    .expect("swap construction")
    .with_calendar(CALENDAR_ID);

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

    let swap = BasisSwap::new(
        "AT-MATURITY",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        maturity,
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    )
    .expect("swap construction")
    .with_calendar(CALENDAR_ID);

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

    let swap = BasisSwap::new(
        "AFTER-MATURITY",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        maturity,
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    )
    .expect("swap construction")
    .with_calendar(CALENDAR_ID);

    let npv = swap.value(&ctx, after_maturity).unwrap();
    // After maturity, all cashflows are in the past, NPV should be zero
    assert!(npv.amount().abs() < 1.0);
}

#[test]
fn identical_forward_curves() {
    // Test when both legs reference the same forward curve.
    // Must use new_allowing_same_curve() since new() rejects identical curves.
    let ctx = market();
    let as_of = d(2025, 1, 2);

    let swap = BasisSwap::new_allowing_same_curve(
        "IDENTICAL-CURVES",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2026, 1, 2),
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    )
    .expect("swap construction with same curves allowed")
    .with_calendar(CALENDAR_ID);

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
        .interp(InterpStyle::LogLinear)
        .build()
        .unwrap();
    let f3m = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 0.01), (0.5, 0.05), (1.0, 0.10), (2.0, 0.15)])
        .interp(InterpStyle::Linear)
        .build()
        .unwrap();
    let f1m = ForwardCurve::builder("USD-SOFR-1M", 1.0 / 12.0)
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 0.01), (0.5, 0.04), (1.0, 0.09), (2.0, 0.14)])
        .interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_forward(f3m)
        .insert_forward(f1m);

    let swap = BasisSwap::new(
        "STEEP-CURVE",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2027, 1, 2),
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    )
    .expect("swap construction")
    .with_calendar(CALENDAR_ID);

    let res = swap
        .price_with_metrics(&ctx, d(2025, 1, 2), &[MetricId::Dv01])
        .unwrap();

    assert!(res.measures[MetricId::Dv01.as_str()].is_finite());
}

#[test]
fn seasoned_swap_requires_fixings() {
    // Test that a seasoned swap (as_of after reset but before payment) requires historical fixings
    //
    // For a quarterly swap:
    // - Period: 2025-01-02 to 2025-04-02
    // - Reset date: 2025-01-02 (period start with reset_lag=0)
    // - Payment date: 2025-04-02 (period end)
    //
    // Valuation at 2025-02-15 is AFTER reset (needs fixing) but BEFORE payment (period still active)
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(d(2025, 2, 15)) // Base date matches valuation
        .knots(vec![(0.0, 1.0), (0.5, 0.99), (1.0, 0.98), (5.0, 0.90)])
        .interp(InterpStyle::LogLinear)
        .build()
        .unwrap();
    let f3m = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(d(2025, 2, 15))
        .knots(vec![(0.0, 0.02), (0.5, 0.021), (1.0, 0.022), (5.0, 0.025)])
        .interp(InterpStyle::Linear)
        .build()
        .unwrap();
    let f1m = ForwardCurve::builder("USD-SOFR-1M", 1.0 / 12.0)
        .base_date(d(2025, 2, 15))
        .knots(vec![(0.0, 0.019), (0.5, 0.020), (1.0, 0.021), (5.0, 0.024)])
        .interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_forward(f3m)
        .insert_forward(f1m);

    // Create a swap where we're in the middle of a period
    let swap = BasisSwap::new(
        "SEASONED-NO-FIX",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2), // Started early January
        d(2026, 7, 2),
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    )
    .expect("swap construction")
    .with_calendar(CALENDAR_ID);

    // Valuation date 2025-02-15 is AFTER the Q1 reset (2025-01-02) but BEFORE the Q1 payment (2025-04-02)
    // This means we need a fixing for the 2025-01-02 reset
    let result = swap.value(&ctx, d(2025, 2, 15));
    assert!(
        result.is_err(),
        "Expected error for seasoned swap without fixings"
    );
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("fixing") || err_msg.contains("Seasoned"),
        "Error should mention fixings requirement: {}",
        err_msg
    );
}

#[test]
fn seasoned_swap_with_fixings_succeeds() {
    // Test that a seasoned swap prices correctly when fixings are provided
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 1.0), (0.5, 0.99), (1.0, 0.98), (2.0, 0.96)])
        .interp(InterpStyle::LogLinear)
        .build()
        .unwrap();
    let f3m = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 0.02), (0.5, 0.021), (1.0, 0.022), (2.0, 0.023)])
        .interp(InterpStyle::Linear)
        .build()
        .unwrap();
    let f1m = ForwardCurve::builder("USD-SOFR-1M", 1.0 / 12.0)
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 0.019), (0.5, 0.020), (1.0, 0.021), (2.0, 0.022)])
        .interp(InterpStyle::Linear)
        .build()
        .unwrap();

    // Historical fixings for the past reset dates
    let fix_3m = finstack_core::market_data::scalars::ScalarTimeSeries::new(
        "FIXING:USD-SOFR-3M",
        vec![
            (d(2024, 7, 2), 0.0195),  // First reset
            (d(2024, 10, 2), 0.0198), // Second reset
        ],
        None,
    )
    .expect("fixings series");
    let fix_1m = finstack_core::market_data::scalars::ScalarTimeSeries::new(
        "FIXING:USD-SOFR-1M",
        vec![
            (d(2024, 7, 2), 0.0185),  // First reset
            (d(2024, 10, 2), 0.0188), // Second reset
        ],
        None,
    )
    .expect("fixings series");

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_forward(f3m)
        .insert_forward(f1m)
        .insert_series(fix_3m)
        .insert_series(fix_1m);

    let swap = BasisSwap::new(
        "SEASONED-WITH-FIX",
        Money::new(10_000_000.0, USD),
        d(2024, 7, 2), // Started 6 months ago
        d(2026, 7, 2),
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0005, // 5bp spread
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Tenor::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    )
    .expect("swap construction")
    .with_calendar(CALENDAR_ID);

    // Should succeed with fixings
    let result = swap.value(&ctx, d(2025, 1, 2));
    assert!(
        result.is_ok(),
        "Expected success with fixings: {:?}",
        result
    );

    let npv = result.unwrap();
    assert!(npv.amount().is_finite(), "NPV should be finite");
}
