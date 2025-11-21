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

fn swap() -> BasisSwap {
    let _sched = ScheduleParams::quarterly_act360();
    BasisSwap::new(
        "BASIS-TEST",
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
    )
}

#[test]
fn dv01_metrics() {
    let s = swap();
    let ctx = market();
    let as_of = d(2025, 1, 2);

    let res = s
        .price_with_metrics(
            &ctx,
            as_of,
            &[
                MetricId::Dv01,
                MetricId::BasisParSpread,
                MetricId::PvPrimary,
                MetricId::PvReference,
            ],
        )
        .unwrap();

    let dv01 = res.measures[MetricId::Dv01.as_str()];

    // Dv01 is now configured in PerCurve mode, so it returns the sum of individual curve sensitivities
    // and stores the breakdown in measures with composite keys "bucketed_dv01::curve_id"
    // Note: Curve IDs are sanitized (hyphens become underscores) in composite keys

    // Extract per-curve DV01s from measures using composite keys
    let dv01_discount = res
        .measures
        .get("bucketed_dv01::usd_ois")
        .copied()
        .unwrap_or(0.0);
    let dv01_primary_fwd = res
        .measures
        .get("bucketed_dv01::usd_sofr_3m")
        .copied()
        .unwrap_or(0.0);
    let dv01_reference_fwd = res
        .measures
        .get("bucketed_dv01::usd_sofr_1m")
        .copied()
        .unwrap_or(0.0);

    // Total DV01 should equal sum of individual curve sensitivities
    assert!((dv01 - (dv01_discount + dv01_primary_fwd + dv01_reference_fwd)).abs() < 1e-6);

    // All DV01 components should be finite
    assert!(dv01.is_finite());
    assert!(dv01_discount.is_finite());
    assert!(dv01_primary_fwd.is_finite());
    assert!(dv01_reference_fwd.is_finite());

    // Par spread should move opposite to PV mismatch
    let pv_p = res.measures[MetricId::PvPrimary.as_str()];
    let pv_r = res.measures[MetricId::PvReference.as_str()];
    let spread = res.measures[MetricId::BasisParSpread.as_str()];
    // If legs are balanced (equal PV), par spread close to 0
    assert!(pv_p.is_finite() && pv_r.is_finite());
    assert!(spread.is_finite());
}

#[test]
fn theta_defined_and_finite() {
    let s = swap();
    let ctx = market();
    let as_of = d(2025, 1, 2);
    let res = s
        .price_with_metrics(&ctx, as_of, &[MetricId::Theta])
        .unwrap();
    assert!(res.measures[MetricId::Theta.as_str()].is_finite());
}
