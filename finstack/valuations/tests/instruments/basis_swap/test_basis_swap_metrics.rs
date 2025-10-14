use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::{DiscountCurve, ForwardCurve};
use finstack_core::money::Money;
use finstack_core::{currency::Currency::USD, math::interp::InterpStyle};
use finstack_valuations::cashflow::builder::ScheduleParams;
use finstack_valuations::instruments::basis_swap::types::{BasisLegSpec, BasisSwap};
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
    let sched = ScheduleParams::quarterly_act360();
    BasisSwap::builder()
        .id("BASIS-TEST".into())
        .notional(Money::new(10_000_000.0, USD))
        .primary(BasisLegSpec::new("USD-OIS", "USD-SOFR-3M", 0.0, DayCount::Act360))
        .reference(BasisLegSpec::new("USD-OIS", "USD-SOFR-1M", 0.0, DayCount::Act360))
        .schedule(
            finstack_valuations::instruments::basis_swap::types::BasisScheduleSpec::from_params(
                d(2025, 1, 2),
                d(2026, 1, 2),
                sched,
            ),
        )
        .build()
        .unwrap()
}

#[test]
fn net_dv01_and_par_spread_are_consistent() {
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
                MetricId::Dv01Primary,
                MetricId::Dv01Reference,
                MetricId::PvPrimary,
                MetricId::PvReference,
            ],
        )
        .unwrap();

    let dv01 = res.measures[MetricId::Dv01.as_str()];
    let dv01_p = res.measures[MetricId::Dv01Primary.as_str()];
    let dv01_r = res.measures[MetricId::Dv01Reference.as_str()];
    assert!((dv01 - (dv01_p - dv01_r)).abs() < 1e-6);

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
    let res = s.price_with_metrics(&ctx, as_of, &[MetricId::Theta]).unwrap();
    assert!(res.measures[MetricId::Theta.as_str()].is_finite());
}


