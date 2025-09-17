//! Tests for Total Return Swap instruments.

use finstack_core::{
    currency::Currency::*,
    dates::{Date, DayCount},
    market_data::{
        scalars::MarketScalar, term_structures::DiscountCurve, term_structures::ForwardCurve,
        MarketContext,
    },
    math::interp::InterpStyle,
    money::Money,
};
use finstack_valuations::cashflow::builder::ScheduleParams;
use finstack_valuations::instruments::{
    derivatives::trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap, TrsSide},
    equity::EquityUnderlyingParams,
    traits::Priceable,
};
use finstack_valuations::instruments::derivatives::trs::parameters::IndexUnderlyingParams;
use time::Month;

fn create_test_market_context() -> MarketContext {
    let mut context = MarketContext::new();

    // Add discount curve
    let disc_curve = DiscountCurve::builder("USD-OIS")
        .base_date(Date::from_calendar_date(2025, Month::January, 2).unwrap())
        .knots(vec![
            (0.0, 1.0),
            (0.25, 0.995),
            (0.5, 0.990),
            (1.0, 0.980),
            (2.0, 0.960),
            (5.0, 0.900),
        ])
        .set_interp(InterpStyle::LogLinear)
        .build()
        .unwrap();
    context = context.insert_discount(disc_curve);

    // Add forward curve
    let fwd_curve = ForwardCurve::builder("USD-SOFR-3M", 0.25)
        .base_date(Date::from_calendar_date(2025, Month::January, 2).unwrap())
        .knots(vec![
            (0.0, 0.02),
            (0.25, 0.021),
            (0.5, 0.022),
            (1.0, 0.023),
            (2.0, 0.024),
        ])
        .set_interp(InterpStyle::Linear)
        .build()
        .unwrap();
    context = context.insert_forward(fwd_curve);

    // Add market scalars
    context = context.insert_price("SPX-SPOT", MarketScalar::Unitless(5000.0));
    context = context.insert_price("SPX-DIV-YIELD", MarketScalar::Unitless(0.015)); // 1.5% dividend yield
    context = context.insert_price("HY-INDEX-YIELD", MarketScalar::Unitless(0.055)); // 5.5% yield for HY index
    context = context.insert_price("HY-INDEX-DURATION", MarketScalar::Unitless(4.5)); // 4.5 years duration

    context
}

#[test]
fn test_equity_trs_creation() {
    let as_of = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let notional = Money::new(10_000_000.0, USD);

    let underlying = EquityUnderlyingParams::new("SPX", "SPX-SPOT")
        .with_dividend_yield("SPX-DIV-YIELD")
        .with_contract_size(1.0);

    let sched = ScheduleParams::quarterly_act360();

    let trs = EquityTotalReturnSwap::builder()
        .id("TRS-SPX-001".into())
        .notional(notional)
        .underlying(underlying)
        .financing(finstack_valuations::instruments::derivatives::trs::FinancingLegSpec::new("USD-OIS", "USD-SOFR-3M", 25.0, DayCount::Act360))
        .schedule(finstack_valuations::instruments::derivatives::trs::TrsScheduleSpec::from_params(
            as_of,
            Date::from_calendar_date(2026, Month::January, 2).unwrap(),
            sched,
        ))
        .side(TrsSide::ReceiveTotalReturn)
        .build()
        .unwrap();

    assert_eq!(trs.id.as_str(), "TRS-SPX-001");
    assert_eq!(trs.notional, notional);
    assert_eq!(trs.side, TrsSide::ReceiveTotalReturn);
}

#[test]
fn test_equity_trs_pricing() {
    let as_of = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let notional = Money::new(10_000_000.0, USD);
    let context = create_test_market_context();

    let underlying = EquityUnderlyingParams::new("SPX", "SPX-SPOT")
        .with_dividend_yield("SPX-DIV-YIELD")
        .with_contract_size(1.0);

    let sched = ScheduleParams::quarterly_act360();

    let trs = EquityTotalReturnSwap::builder()
        .id("TRS-SPX-001".into())
        .notional(notional)
        .underlying(underlying)
        .financing(finstack_valuations::instruments::derivatives::trs::FinancingLegSpec::new("USD-OIS", "USD-SOFR-3M", 50.0, DayCount::Act360))
        .schedule(finstack_valuations::instruments::derivatives::trs::TrsScheduleSpec::from_params(
            as_of,
            Date::from_calendar_date(2026, Month::January, 2).unwrap(),
            sched,
        ))
        .side(TrsSide::ReceiveTotalReturn)
        .build()
        .unwrap();

    let npv = trs.value(&context, as_of).unwrap();

    // With a 50bp spread to compensate for equity risk premium, NPV should be reasonable
    assert!(
        npv.amount().abs() < 500_000.0,
        "NPV should be reasonable but was {}",
        npv.amount()
    );
}

#[test]
fn test_equity_trs_delta() {
    let as_of = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let notional = Money::new(10_000_000.0, USD);
    let context = create_test_market_context();

    let underlying = EquityUnderlyingParams::new("SPX", "SPX-SPOT")
        .with_dividend_yield("SPX-DIV-YIELD")
        .with_contract_size(1.0);

    let trs = EquityTotalReturnSwap::builder()
        .id("TRS-SPX-001".into())
        .notional(notional)
        .underlying(underlying)
        .financing(finstack_valuations::instruments::derivatives::trs::FinancingLegSpec::new("USD-OIS", "USD-SOFR-3M", 25.0, DayCount::Act360))
        .schedule(finstack_valuations::instruments::derivatives::trs::TrsScheduleSpec::from_params(
            as_of,
            Date::from_calendar_date(2026, Month::January, 2).unwrap(),
            ScheduleParams::quarterly_act360(),
        ))
        .side(TrsSide::ReceiveTotalReturn)
        .build()
        .unwrap();

    let result = trs.price_with_metrics(&context, as_of, &[]).unwrap();

    // For now, just check that the TRS can be priced without error
    // TODO: Add proper metric calculation when registry is integrated
    assert!(result.value.amount().is_finite());
}

#[test]
fn test_fi_index_trs_creation() {
    let as_of = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let notional = Money::new(10_000_000.0, USD);

    let underlying = IndexUnderlyingParams::new("HY.BOND.INDEX", USD)
        .with_yield("HY-INDEX-YIELD")
        .with_duration("HY-INDEX-DURATION")
        .with_contract_size(1.0);

    let sched = ScheduleParams::quarterly_act360();

    let trs = FIIndexTotalReturnSwap::builder()
        .id("TRS-HY-001".into())
        .notional(notional)
        .underlying(underlying)
        .financing(finstack_valuations::instruments::derivatives::trs::FinancingLegSpec::new("USD-OIS", "USD-SOFR-3M", 100.0, DayCount::Act360))
        .schedule(finstack_valuations::instruments::derivatives::trs::TrsScheduleSpec::from_params(
            as_of,
            Date::from_calendar_date(2026, Month::January, 2).unwrap(),
            sched,
        ))
        .side(TrsSide::ReceiveTotalReturn)
        .build()
        .unwrap();

    assert_eq!(trs.id.as_str(), "TRS-HY-001");
    assert_eq!(trs.notional, notional);
    assert_eq!(trs.side, TrsSide::ReceiveTotalReturn);
}

#[test]
fn test_fi_index_trs_par_spread() {
    let as_of = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let notional = Money::new(10_000_000.0, USD);
    let context = create_test_market_context();

    let underlying = IndexUnderlyingParams::new("HY.BOND.INDEX", USD)
        .with_yield("HY-INDEX-YIELD")
        .with_duration("HY-INDEX-DURATION")
        .with_contract_size(1.0);

    let trs = FIIndexTotalReturnSwap::builder()
        .id("TRS-HY-001".into())
        .notional(notional)
        .underlying(underlying)
        .financing(finstack_valuations::instruments::derivatives::trs::FinancingLegSpec::new("USD-OIS", "USD-SOFR-3M", 0.0, DayCount::Act360))
        .schedule(finstack_valuations::instruments::derivatives::trs::TrsScheduleSpec::from_params(
            as_of,
            Date::from_calendar_date(2026, Month::January, 2).unwrap(),
            ScheduleParams::quarterly_act360(),
        ))
        .side(TrsSide::ReceiveTotalReturn)
        .build()
        .unwrap();

    let result = trs.price_with_metrics(&context, as_of, &[]).unwrap();

    // For now, just check that the TRS can be priced without error
    // TODO: Add proper metric calculation when registry is integrated
    assert!(result.value.amount().is_finite());
}

#[test]
fn test_currency_safety() {
    let as_of = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let notional = Money::new(10_000_000.0, USD);

    // Try to create FI index TRS with mismatched currencies
    let underlying = IndexUnderlyingParams::new("EUR.BOND.INDEX", EUR)
        .with_yield("EUR-INDEX-YIELD")
        .with_contract_size(1.0);

    let result = FIIndexTotalReturnSwap::builder()
        .id("TRS-EUR-001".into())
        .notional(notional) // USD notional
        .underlying(underlying) // EUR index
        .financing(finstack_valuations::instruments::derivatives::trs::FinancingLegSpec::new("USD-OIS", "USD-SOFR-3M", 0.0, DayCount::Act360))
        .schedule(finstack_valuations::instruments::derivatives::trs::TrsScheduleSpec::from_params(
            as_of,
            Date::from_calendar_date(2026, Month::January, 2).unwrap(),
            ScheduleParams::quarterly_act360(),
        ))
        .build();

    assert!(result.is_err(), "Should fail with currency mismatch");
}

#[test]
fn test_trs_cashflow_schedule() {
    let as_of = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let notional = Money::new(10_000_000.0, USD);
    let context = create_test_market_context();

    let underlying =
        EquityUnderlyingParams::new("SPX", "SPX-SPOT").with_dividend_yield("SPX-DIV-YIELD");

    let trs = EquityTotalReturnSwap::builder()
        .id("TRS-SPX-001".into())
        .notional(notional)
        .underlying(underlying)
        .financing(finstack_valuations::instruments::derivatives::trs::FinancingLegSpec::new("USD-OIS", "USD-SOFR-3M", 0.0, DayCount::Act360))
        .schedule(finstack_valuations::instruments::derivatives::trs::TrsScheduleSpec::from_params(
            as_of,
            Date::from_calendar_date(2026, Month::January, 2).unwrap(),
            ScheduleParams::quarterly_act360(),
        ))
        .side(TrsSide::ReceiveTotalReturn)
        .build()
        .unwrap();

    use finstack_valuations::cashflow::traits::CashflowProvider;
    let flows = trs.build_schedule(&context, as_of).unwrap();

    // Should have 4 quarterly payments
    assert_eq!(flows.len(), 4, "Should have 4 quarterly payment dates");

    // All flows should be in USD
    for (_, amount) in &flows {
        assert_eq!(amount.currency(), USD);
    }
}

#[test]
fn test_pay_vs_receive_total_return() {
    let as_of = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    let notional = Money::new(10_000_000.0, USD);
    let context = create_test_market_context();

    let underlying =
        EquityUnderlyingParams::new("SPX", "SPX-SPOT").with_dividend_yield("SPX-DIV-YIELD");

    // Receive total return
    let trs_receive = EquityTotalReturnSwap::builder()
        .id("TRS-RECEIVE".into())
        .notional(notional)
        .underlying(underlying.clone())
        .financing(finstack_valuations::instruments::derivatives::trs::FinancingLegSpec::new("USD-OIS", "USD-SOFR-3M", 50.0, DayCount::Act360))
        .schedule(finstack_valuations::instruments::derivatives::trs::TrsScheduleSpec::from_params(
            as_of,
            Date::from_calendar_date(2026, Month::January, 2).unwrap(),
            ScheduleParams::quarterly_act360(),
        ))
        .side(TrsSide::ReceiveTotalReturn)
        .build()
        .unwrap();

    // Pay total return
    let trs_pay = EquityTotalReturnSwap::builder()
        .id("TRS-PAY".into())
        .notional(notional)
        .underlying(underlying)
        .financing(finstack_valuations::instruments::derivatives::trs::FinancingLegSpec::new("USD-OIS", "USD-SOFR-3M", 50.0, DayCount::Act360))
        .schedule(finstack_valuations::instruments::derivatives::trs::TrsScheduleSpec::from_params(
            as_of,
            Date::from_calendar_date(2026, Month::January, 2).unwrap(),
            ScheduleParams::quarterly_act360(),
        ))
        .side(TrsSide::PayTotalReturn)
        .build()
        .unwrap();

    let npv_receive = trs_receive.value(&context, as_of).unwrap();
    let npv_pay = trs_pay.value(&context, as_of).unwrap();

    // NPVs should be opposite in sign
    assert!(
        (npv_receive.amount() + npv_pay.amount()).abs() < 1.0,
        "Receive and pay NPVs should sum to ~0 but got {} and {}",
        npv_receive.amount(),
        npv_pay.amount()
    );
}
