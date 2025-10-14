use finstack_core::currency::Currency::*;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::{
    scalars::MarketScalar, term_structures::DiscountCurve, term_structures::ForwardCurve, MarketContext,
};
use finstack_core::math::interp::InterpStyle;
use finstack_core::money::Money;
use finstack_valuations::cashflow::builder::ScheduleParams;
use finstack_valuations::instruments::trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap, TrsSide};
use finstack_valuations::instruments::underlying::EquityUnderlyingParams;
use finstack_valuations::metrics::MetricId;
use time::Month;

fn d(y: i32, m: u8, day: u8) -> Date {
    Date::from_calendar_date(y, Month::try_from(m).unwrap(), day).unwrap()
}

fn market() -> MarketContext {
    let mut context = MarketContext::new();
    let disc_curve = DiscountCurve::builder("USD-OIS")
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 1.0), (0.25, 0.995), (0.5, 0.990), (1.0, 0.980), (2.0, 0.960)])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap();
    context = context.insert_discount(disc_curve);

    let fwd_curve = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(d(2025, 1, 2))
        .knots(vec![(0.0, 0.02), (0.25, 0.021), (0.5, 0.022), (1.0, 0.023), (2.0, 0.024)])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();
    context = context.insert_forward(fwd_curve);

    context = context.insert_price("SPX-SPOT", MarketScalar::Unitless(5000.0));
    context = context.insert_price("SPX-DIV-YIELD", MarketScalar::Unitless(0.015));
    context = context.insert_price("HY-INDEX-YIELD", MarketScalar::Unitless(0.055));
    context = context.insert_price("HY-INDEX-DURATION", MarketScalar::Unitless(4.5));
    context
}

#[test]
fn equity_trs_pricer_and_metrics() {
    let as_of = d(2025, 1, 2);
    let notional = Money::new(5_000_000.0, USD);
    let underlying = EquityUnderlyingParams::new("SPX", "SPX-SPOT")
        .with_dividend_yield("SPX-DIV-YIELD")
        .with_contract_size(1.0);
    let sched = ScheduleParams::quarterly_act360();

    let trs = EquityTotalReturnSwap::builder()
        .id("TRS-SPX-TEST".into())
        .notional(notional)
        .underlying(underlying)
        .financing(finstack_valuations::instruments::trs::FinancingLegSpec::new(
            "USD-OIS",
            "USD-SOFR-3M",
            25.0,
            DayCount::Act360,
        ))
        .schedule(finstack_valuations::instruments::trs::TrsScheduleSpec::from_params(
            as_of,
            d(2025, 7, 2),
            sched,
        ))
        .side(TrsSide::ReceiveTotalReturn)
        .build()
        .unwrap();

    let ctx = market();
    // Base pricing path should succeed
    let pv = trs.value(&ctx, as_of).unwrap();
    assert_eq!(pv.currency(), USD);

    // Metrics path
    let res = trs
        .price_with_metrics(
            &ctx,
            as_of,
            &[MetricId::FinancingAnnuity, MetricId::IndexDelta, MetricId::Ir01, MetricId::Theta],
        )
        .unwrap();
    assert!(res.measures[MetricId::FinancingAnnuity.as_str()].is_finite());
    assert!(res.measures[MetricId::IndexDelta.as_str()].is_finite());
    assert!(res.measures[MetricId::Ir01.as_str()].is_finite());
}

#[test]
fn fi_index_trs_par_spread_metric() {
    let as_of = d(2025, 1, 2);
    let notional = Money::new(3_000_000.0, USD);
    let sched = ScheduleParams::quarterly_act360();

    let trs = FIIndexTotalReturnSwap::builder()
        .id("TRS-FI-TEST".into())
        .notional(notional)
        .index_id("HY-INDEX")
        .financing(finstack_valuations::instruments::trs::FinancingLegSpec::new(
            "USD-OIS",
            "USD-SOFR-3M",
            0.0,
            DayCount::Act360,
        ))
        .schedule(finstack_valuations::instruments::trs::TrsScheduleSpec::from_params(
            as_of,
            d(2025, 7, 2),
            sched,
        ))
        .side(TrsSide::PayTotalReturn)
        .build()
        .unwrap();

    let ctx = market();
    let res = trs
        .price_with_metrics(&ctx, as_of, &[MetricId::ParSpread])
        .unwrap();
    // Par spread calculation should yield finite value
    assert!(res.measures[MetricId::ParSpread.as_str()].is_finite());
}


