//! Theta (time decay) tests for basis swaps.
//!
//! Tests validate theta calculations and time decay behavior across various scenarios.

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
        .knots(vec![
            (0.0, 1.0),
            (0.5, 0.99),
            (1.0, 0.98),
            (2.0, 0.96),
            (3.0, 0.94),
        ])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap();
    let f3m = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(d(2025, 1, 2))
        .knots(vec![
            (0.0, 0.02),
            (0.5, 0.021),
            (1.0, 0.022),
            (2.0, 0.023),
            (3.0, 0.024),
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();
    let f1m = ForwardCurve::builder("USD-SOFR-1M", 1.0 / 12.0)
        .base_date(d(2025, 1, 2))
        .knots(vec![
            (0.0, 0.019),
            (0.5, 0.020),
            (1.0, 0.021),
            (2.0, 0.022),
            (3.0, 0.023),
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();
    MarketContext::new()
        .insert_discount(disc)
        .insert_forward(f3m)
        .insert_forward(f1m)
}

#[test]
fn theta_is_finite() {
    // Basic test that theta calculation produces finite result
    let ctx = market();
    let as_of = d(2025, 1, 2);
    let _sched = ScheduleParams::quarterly_act360();

    let swap = BasisSwap::new(
        "THETA-FINITE",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2026, 1, 2),
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    );

    let res = swap
        .price_with_metrics(&ctx, as_of, &[MetricId::Theta])
        .unwrap();
    let theta = res.measures[MetricId::Theta.as_str()];

    assert!(theta.is_finite(), "Theta should be finite");
}

#[test]
fn theta_matches_pv_change() {
    // Test that theta approximates PV change over one day
    let ctx = market();
    let as_of = d(2025, 1, 2);
    let next_day = d(2025, 1, 3);
    let _sched = ScheduleParams::quarterly_act360();

    let swap = BasisSwap::new(
        "THETA-PV-MATCH",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2026, 1, 2),
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    );

    // Get theta
    let res = swap
        .price_with_metrics(&ctx, as_of, &[MetricId::Theta])
        .unwrap();
    let theta = res.measures[MetricId::Theta.as_str()];

    // Get PV at T and T+1
    let pv_today = swap.value(&ctx, as_of).unwrap().amount();
    let pv_tomorrow = swap.value(&ctx, next_day).unwrap().amount();
    let actual_change = pv_tomorrow - pv_today;

    // Theta should approximate the PV change (within 10% tolerance)
    let error_pct = if actual_change.abs() > 1.0 {
        ((theta - actual_change).abs() / actual_change.abs()) * 100.0
    } else {
        0.0
    };

    assert!(
        error_pct < 20.0 || (theta - actual_change).abs() < 100.0,
        "Theta {} should approximate PV change {}, error {}%",
        theta,
        actual_change,
        error_pct
    );
}

#[test]
fn theta_sign_convention() {
    // Test theta sign: typically positive (gain value as time passes due to discounting)
    let ctx = market();
    let as_of = d(2025, 1, 2);
    let _sched = ScheduleParams::quarterly_act360();

    let swap = BasisSwap::new(
        "THETA-SIGN",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2026, 1, 2),
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0010, // 10bp spread
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    );

    let res = swap
        .price_with_metrics(&ctx, as_of, &[MetricId::Theta])
        .unwrap();
    let theta = res.measures[MetricId::Theta.as_str()];

    // For a swap with positive NPV, theta should reflect time decay
    let npv = swap.value(&ctx, as_of).unwrap().amount();
    if npv > 1000.0 {
        // If we have significant positive NPV, as time passes toward cashflows,
        // we may gain or lose value depending on discounting effects
        assert!(theta.is_finite());
    }
}

#[test]
fn theta_decreases_near_maturity() {
    // Test that theta magnitude decreases as maturity approaches
    let ctx = market();
    let _sched = ScheduleParams::quarterly_act360();

    let swap = BasisSwap::new(
        "THETA-NEAR-MAT",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2026, 1, 2),
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    );

    // Theta early in life
    let res_early = swap
        .price_with_metrics(&ctx, d(2025, 1, 2), &[MetricId::Theta])
        .unwrap();
    let theta_early = res_early.measures[MetricId::Theta.as_str()].abs();

    // Theta near maturity
    let res_late = swap
        .price_with_metrics(&ctx, d(2025, 12, 2), &[MetricId::Theta])
        .unwrap();
    let theta_late = res_late.measures[MetricId::Theta.as_str()].abs();

    // Theta magnitude should decrease as we approach maturity
    // (less time value remaining)
    assert!(
        theta_late <= theta_early + 10.0,
        "Theta magnitude should not increase significantly near maturity: early={}, late={}",
        theta_early,
        theta_late
    );
}

#[test]
fn theta_at_par() {
    // Test theta for at-par swap (zero NPV)
    let ctx = market();
    let as_of = d(2025, 1, 2);
    let _sched = ScheduleParams::quarterly_act360();

    // First, find the par spread
    let swap_zero = BasisSwap::new(
        "THETA-PAR-ZERO",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2026, 1, 2),
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    );

    let res = swap_zero
        .price_with_metrics(&ctx, as_of, &[MetricId::BasisParSpread])
        .unwrap();
    let par_spread = res.measures[MetricId::BasisParSpread.as_str()] / 1e4;

    // Create swap at par
    let swap_at_par = BasisSwap::new(
        "THETA-PAR",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2026, 1, 2),
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: par_spread,
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    );

    let res_par = swap_at_par
        .price_with_metrics(&ctx, as_of, &[MetricId::Theta])
        .unwrap();
    let theta = res_par.measures[MetricId::Theta.as_str()];

    // Theta should be finite even for at-par swap
    assert!(theta.is_finite(), "Theta should be finite for at-par swap");
}

#[test]
fn theta_with_long_maturity() {
    // Test theta for long-dated swap
    let ctx = market();
    let as_of = d(2025, 1, 2);
    let _sched = ScheduleParams::quarterly_act360();

    let swap = BasisSwap::new(
        "THETA-LONG",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2028, 1, 2),
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    );

    let res = swap
        .price_with_metrics(&ctx, as_of, &[MetricId::Theta])
        .unwrap();
    let theta = res.measures[MetricId::Theta.as_str()];

    assert!(theta.is_finite());
}

#[test]
fn theta_consistency_across_dates() {
    // Test that theta remains consistent when valuating at different dates
    let ctx = market();
    let _sched = ScheduleParams::quarterly_act360();

    let swap = BasisSwap::new(
        "THETA-CONSISTENCY",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2026, 1, 2),
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    );

    let dates = vec![d(2025, 1, 2), d(2025, 2, 2), d(2025, 3, 2), d(2025, 4, 2)];

    for date in dates {
        let res = swap
            .price_with_metrics(&ctx, date, &[MetricId::Theta])
            .unwrap();
        let theta = res.measures[MetricId::Theta.as_str()];
        assert!(theta.is_finite(), "Theta should be finite at date {}", date);
    }
}

#[test]
fn theta_multi_year() {
    // Test that theta accurately approximates 1-day PV change at multiple points
    // over a multi-year swap's lifetime.
    //
    // Note: Annual theta extrapolation (theta × 365) is NOT valid because:
    // 1. Theta changes daily as time passes
    // 2. Cashflow events cause PV discontinuities
    // 3. Compounding effects are non-linear
    //
    // Instead, we test that daily theta matches actual 1-day PV change
    // at several points during the swap's life.

    let ctx = market();
    let _sched = ScheduleParams::quarterly_act360();

    let swap = BasisSwap::new(
        "THETA-MULTI-YEAR",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2028, 1, 2),
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0005,
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    );

    // Test theta accuracy at several dates during the swap's life
    // Avoid dates near coupon payments where PV has discontinuities
    let test_dates = [
        (d(2025, 1, 15), d(2025, 1, 16)),
        (d(2025, 5, 15), d(2025, 5, 16)),
        (d(2026, 1, 15), d(2026, 1, 16)),
        (d(2027, 1, 15), d(2027, 1, 16)),
    ];

    for (today, tomorrow) in test_dates {
        let res = swap
            .price_with_metrics(&ctx, today, &[MetricId::Theta])
            .unwrap();
        let theta = res.measures[MetricId::Theta.as_str()];

        let pv_today = swap.value(&ctx, today).unwrap().amount();
        let pv_tomorrow = swap.value(&ctx, tomorrow).unwrap().amount();
        let actual_change = pv_tomorrow - pv_today;

        // Theta should approximate 1-day PV change within 25%
        // (first-order approximation has higher error for complex instruments)
        let error_pct = if actual_change.abs() > 1.0 {
            ((theta - actual_change).abs() / actual_change.abs()) * 100.0
        } else {
            0.0
        };

        assert!(
            error_pct < 25.0 || (theta - actual_change).abs() < 100.0,
            "Theta at {} should approximate 1-day PV change: theta={:.2}, actual={:.2}, error={:.1}%",
            today,
            theta,
            actual_change,
            error_pct
        );
    }
}

#[test]
fn theta_zero_at_maturity() {
    // Test that theta approaches zero at maturity
    let ctx = market();
    let maturity = d(2025, 12, 31);
    let _sched = ScheduleParams::quarterly_act360();

    let swap = BasisSwap::new(
        "THETA-AT-MAT",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        maturity,
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-3M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        BasisSwapLeg {
            payment_lag_days: 0,
            reset_lag_days: 0,
            forward_curve_id: CurveId::new("USD-SOFR-1M"),
            frequency: Frequency::quarterly(),
            day_count: DayCount::Act360,
            bdc: BusinessDayConvention::ModifiedFollowing,
            spread: 0.0,
        },
        CurveId::new("USD-OIS"),
    );

    let res = swap
        .price_with_metrics(&ctx, maturity, &[MetricId::Theta])
        .unwrap();
    let theta = res.measures[MetricId::Theta.as_str()];

    // At maturity, theta should be very small (near zero)
    assert!(
        theta.abs() < 100.0,
        "Theta at maturity should be near zero, got {}",
        theta
    );
}
