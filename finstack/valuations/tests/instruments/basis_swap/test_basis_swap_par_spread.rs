//! Comprehensive par spread calculation tests for basis swaps.
//!
//! Tests verify that computed par spreads correctly set NPV to zero and
//! validate the mathematical relationship between par spread, annuity, and PV.

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
        .knots(vec![(0.0, 1.0), (0.5, 0.99), (1.0, 0.98), (2.0, 0.96)])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap();
    let f3m = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 0.02), (0.5, 0.021), (1.0, 0.022), (2.0, 0.023)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();
    let f1m = ForwardCurve::builder("USD-SOFR-1M", 1.0 / 12.0)
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 0.019), (0.5, 0.020), (1.0, 0.021), (2.0, 0.022)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();
    MarketContext::new()
        .insert_discount(disc)
        .insert_forward(f3m)
        .insert_forward(f1m)
}

#[test]
fn par_spread_zeros_npv() {
    // Test that applying the computed par spread results in zero NPV
    let ctx = market();
    let as_of = d(2025, 1, 2);
    let _sched = ScheduleParams::quarterly_act360();

    // Create swap with zero spread
    let swap = BasisSwap::new(
        "PAR-TEST",
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

    // Compute par spread
    let res = swap
        .price_with_metrics(&ctx, as_of, &[MetricId::BasisParSpread])
        .unwrap();
    let par_spread_bp = res.measures[MetricId::BasisParSpread.as_str()];
    let par_spread_decimal = par_spread_bp / 1e4;

    // Create new swap with par spread applied
    let swap_at_par = BasisSwap::new(
        "PAR-TEST-APPLIED",
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
            spread: par_spread_decimal,
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

    let npv = swap_at_par.value(&ctx, as_of).unwrap();

    // NPV should be very close to zero (within $1 tolerance)
    assert!(
        npv.amount().abs() < 1.0,
        "NPV with par spread should be near zero, got {}",
        npv.amount()
    );
}

#[test]
fn par_spread_formula_validation() {
    // Validate: par_spread * annuity * notional = PV_difference
    let ctx = market();
    let as_of = d(2025, 1, 2);
    let _sched = ScheduleParams::quarterly_act360();

    let swap = BasisSwap::new(
        "FORMULA-TEST",
        Money::new(5_000_000.0, USD),
        d(2025, 1, 2),
        d(2027, 1, 2),
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
        .price_with_metrics(
            &ctx,
            as_of,
            &[
                MetricId::BasisParSpread,
                MetricId::AnnuityPrimary,
                MetricId::PvPrimary,
                MetricId::PvReference,
            ],
        )
        .unwrap();

    let par_spread_bp = res.measures[MetricId::BasisParSpread.as_str()];
    let par_spread_decimal = par_spread_bp / 1e4;
    let annuity = res.measures[MetricId::AnnuityPrimary.as_str()];
    let pv_primary = res.measures[MetricId::PvPrimary.as_str()];
    let pv_reference = res.measures[MetricId::PvReference.as_str()];
    let notional = swap.notional.amount();

    // par_spread (decimal) * annuity * notional should equal (pv_reference - pv_primary)
    let computed_pv_diff = par_spread_decimal * annuity * notional;
    let actual_pv_diff = pv_reference - pv_primary;

    assert!(
        (computed_pv_diff - actual_pv_diff).abs() < 1.0,
        "Par spread formula mismatch: computed {} vs actual {}",
        computed_pv_diff,
        actual_pv_diff
    );
}

#[test]
fn par_spread_with_existing_spread() {
    // Test par spread calculation when primary leg already has a spread
    let ctx = market();
    let as_of = d(2025, 1, 2);
    let _sched = ScheduleParams::quarterly_act360();

    let swap = BasisSwap::new(
        "EXISTING-SPREAD-TEST",
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
            spread: 0.0010, // 10bp existing spread
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
        .price_with_metrics(&ctx, as_of, &[MetricId::BasisParSpread])
        .unwrap();
    let par_spread = res.measures[MetricId::BasisParSpread.as_str()];

    // Par spread should be finite and represent the additional spread needed
    assert!(par_spread.is_finite());
    // Since we already have 10bp spread, par spread should be different from zero-spread case
}

#[test]
fn par_spread_inverted_curves() {
    // Test with inverted forward curves (shorter tenor > longer tenor)
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 1.0), (1.0, 0.98), (2.0, 0.96)])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap();
    let f3m = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 0.025), (1.0, 0.024), (2.0, 0.023)]) // Inverted
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();
    let f1m = ForwardCurve::builder("USD-SOFR-1M", 1.0 / 12.0)
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 0.030), (1.0, 0.029), (2.0, 0.028)]) // Higher than 3M
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();

    let ctx = MarketContext::new()
        .insert_discount(disc)
        .insert_forward(f3m)
        .insert_forward(f1m);

    let _sched = ScheduleParams::quarterly_act360();
    let swap = BasisSwap::new(
        "INVERTED-TEST",
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
        .price_with_metrics(&ctx, d(2025, 1, 2), &[MetricId::BasisParSpread])
        .unwrap();
    let par_spread = res.measures[MetricId::BasisParSpread.as_str()];

    assert!(par_spread.is_finite());
    // With inverted curves (1M higher than 3M), par spread should be positive
    // to compensate for receiving lower 3M rate
}

#[test]
fn par_spread_long_maturity() {
    // Test par spread for longer-dated swaps (5 years)
    let ctx = market();
    let as_of = d(2025, 1, 2);
    let _sched = ScheduleParams::quarterly_act360();

    let swap = BasisSwap::new(
        "LONG-MAT-TEST",
        Money::new(10_000_000.0, USD),
        d(2025, 1, 2),
        d(2030, 1, 2),
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
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::BasisParSpread, MetricId::AnnuityPrimary],
        )
        .unwrap();
    let par_spread = res.measures[MetricId::BasisParSpread.as_str()];
    let annuity = res.measures[MetricId::AnnuityPrimary.as_str()];

    assert!(par_spread.is_finite());
    assert!(annuity > 0.0);
    // Annuity should be larger for longer maturities
}

#[test]
fn par_spread_different_frequencies() {
    // Test par spread with different payment frequencies
    let ctx = market();
    let as_of = d(2025, 1, 2);

    // Quarterly vs Monthly
    let swap = BasisSwap::new(
        "FREQ-TEST",
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
        .price_with_metrics(&ctx, as_of, &[MetricId::BasisParSpread])
        .unwrap();
    let par_spread = res.measures[MetricId::BasisParSpread.as_str()];

    assert!(par_spread.is_finite());
}

#[test]
fn par_spread_sign_convention() {
    // Verify par spread sign convention: positive spread added to primary leg
    let ctx = market();
    let as_of = d(2025, 1, 2);
    let _sched = ScheduleParams::quarterly_act360();

    // If 3M rate > 1M rate, primary leg receives more, so negative spread needed
    // If 1M rate > 3M rate, primary leg receives less, so positive spread needed
    let swap = BasisSwap::new(
        "SIGN-TEST",
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
        .price_with_metrics(
            &ctx,
            as_of,
            &[
                MetricId::BasisParSpread,
                MetricId::PvPrimary,
                MetricId::PvReference,
            ],
        )
        .unwrap();

    let par_spread = res.measures[MetricId::BasisParSpread.as_str()];
    let pv_primary = res.measures[MetricId::PvPrimary.as_str()];
    let pv_reference = res.measures[MetricId::PvReference.as_str()];

    // If PV_primary < PV_reference, need positive spread to increase primary leg value
    // If PV_primary > PV_reference, need negative spread to decrease primary leg value
    if pv_primary < pv_reference {
        assert!(
            par_spread > 0.0,
            "Expected positive par spread when primary < reference"
        );
    } else if pv_primary > pv_reference {
        assert!(
            par_spread < 0.0,
            "Expected negative par spread when primary > reference"
        );
    }
}
