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
use finstack_valuations::instruments::trs::IndexUnderlyingParams;
use finstack_valuations::instruments::{
    traits::Priceable,
    trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap, TrsSide},
    underlying::EquityUnderlyingParams,
};
use finstack_valuations::metrics::MetricId;
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
        .financing(
            finstack_valuations::instruments::trs::FinancingLegSpec::new(
                "USD-OIS",
                "USD-SOFR-3M",
                25.0,
                DayCount::Act360,
            ),
        )
        .schedule(
            finstack_valuations::instruments::trs::TrsScheduleSpec::from_params(
                as_of,
                Date::from_calendar_date(2026, Month::January, 2).unwrap(),
                sched,
            ),
        )
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
        .financing(
            finstack_valuations::instruments::trs::FinancingLegSpec::new(
                "USD-OIS",
                "USD-SOFR-3M",
                50.0,
                DayCount::Act360,
            ),
        )
        .schedule(
            finstack_valuations::instruments::trs::TrsScheduleSpec::from_params(
                as_of,
                Date::from_calendar_date(2026, Month::January, 2).unwrap(),
                sched,
            ),
        )
        .side(TrsSide::ReceiveTotalReturn)
        .build()
        .unwrap();

    // Compute with metrics
    let res = trs
        .price_with_metrics(
            &context,
            as_of,
            &[
                MetricId::ParSpread,
                MetricId::FinancingAnnuity,
                MetricId::Ir01,
            ],
        )
        .unwrap();

    // Par spread should be in the ballpark of configured financing spread (50bp)
    let par_spread = *res.measures.get("par_spread").unwrap();
    assert!(
        (par_spread - 50.0).abs() < 50.0,
        "Par spread {} not near configured 50bp",
        par_spread
    );

    // Financing annuity positive; IR01 positive
    assert!(*res.measures.get("financing_annuity").unwrap() > 0.0);
    assert!(*res.measures.get("ir01").unwrap() > 0.0);
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
        .financing(
            finstack_valuations::instruments::trs::FinancingLegSpec::new(
                "USD-OIS",
                "USD-SOFR-3M",
                25.0,
                DayCount::Act360,
            ),
        )
        .schedule(
            finstack_valuations::instruments::trs::TrsScheduleSpec::from_params(
                as_of,
                Date::from_calendar_date(2026, Month::January, 2).unwrap(),
                ScheduleParams::quarterly_act360(),
            ),
        )
        .side(TrsSide::ReceiveTotalReturn)
        .build()
        .unwrap();

    // Compute TRS metrics
    let result = trs
        .price_with_metrics(
            &context,
            as_of,
            &[
                MetricId::ParSpread,
                MetricId::IndexDelta,
                MetricId::FinancingAnnuity,
                MetricId::Ir01,
            ],
        )
        .unwrap();

    // Basic sanity: value and metrics present
    assert!(result.value.amount().is_finite());
    let par_spread = *result.measures.get("par_spread").unwrap();
    let index_delta = *result.measures.get("index_delta").unwrap();
    let annuity = *result.measures.get("financing_annuity").unwrap();
    let ir01 = *result.measures.get("ir01").unwrap();

    // Financing annuity should be positive and <= notional * years (≈1y here)
    assert!(annuity > 0.0);
    assert!(annuity <= notional.amount() * 1.05);

    // IR01 proxy uses annuity; should be positive
    assert!(ir01 > 0.0);

    // Index delta sign sanity vs side
    assert!(index_delta > 0.0, "Receive TR should have positive delta");

    // Magnitude sanity via spot bump finite-difference: ΔPV ≈ delta * ΔS
    let spot = match context.price("SPX-SPOT").unwrap() {
        MarketScalar::Unitless(v) => *v,
        MarketScalar::Price(p) => p.amount(),
    };
    let eps = 0.01; // 1% spot bump
    let d_s = spot * eps;
    let context_bumped = context.clone().insert_price("SPX-SPOT", MarketScalar::Unitless(spot * (1.0 + eps)));
    let pv0 = trs.value(&context, as_of).unwrap().amount();
    let pv1 = trs.value(&context_bumped, as_of).unwrap().amount();
    let dpv_fd = pv1 - pv0;
    let dpv_lin = index_delta * d_s;
    // Allow some tolerance due to discounting and yield effects
    let tol = (notional.amount() * 0.02).max(1.0); // 2% of notional or $1
    assert!(
        (dpv_fd - dpv_lin).abs() < tol,
        "FD ΔPV {} vs linear {} differ by more than {}",
        dpv_fd,
        dpv_lin,
        tol
    );

    // Par spread sanity check: if we set financing spread to par, PV ~ 0
    let underlying2 = EquityUnderlyingParams::new("SPX", "SPX-SPOT")
        .with_dividend_yield("SPX-DIV-YIELD")
        .with_contract_size(1.0);
    let trs_par = EquityTotalReturnSwap::builder()
        .id("TRS-SPX-001-PAR".into())
        .notional(notional)
        .underlying(underlying2)
        .financing(
            finstack_valuations::instruments::trs::FinancingLegSpec::new(
                "USD-OIS",
                "USD-SOFR-3M",
                par_spread,
                DayCount::Act360,
            ),
        )
        .schedule(
            finstack_valuations::instruments::trs::TrsScheduleSpec::from_params(
                as_of,
                Date::from_calendar_date(2026, Month::January, 2).unwrap(),
                ScheduleParams::quarterly_act360(),
            ),
        )
        .side(TrsSide::ReceiveTotalReturn)
        .build()
        .unwrap();
    let pv_par = trs_par.value(&context, as_of).unwrap();
    assert!(pv_par.amount().abs() < 1.0, "PV at par should be ~0, got {}", pv_par.amount());
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
        .financing(
            finstack_valuations::instruments::trs::FinancingLegSpec::new(
                "USD-OIS",
                "USD-SOFR-3M",
                100.0,
                DayCount::Act360,
            ),
        )
        .schedule(
            finstack_valuations::instruments::trs::TrsScheduleSpec::from_params(
                as_of,
                Date::from_calendar_date(2026, Month::January, 2).unwrap(),
                sched,
            ),
        )
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
        .financing(
            finstack_valuations::instruments::trs::FinancingLegSpec::new(
                "USD-OIS",
                "USD-SOFR-3M",
                0.0,
                DayCount::Act360,
            ),
        )
        .schedule(
            finstack_valuations::instruments::trs::TrsScheduleSpec::from_params(
                as_of,
                Date::from_calendar_date(2026, Month::January, 2).unwrap(),
                ScheduleParams::quarterly_act360(),
            ),
        )
        .side(TrsSide::ReceiveTotalReturn)
        .build()
        .unwrap();

    // Compute TRS metrics
    let result = trs
        .price_with_metrics(
            &context,
            as_of,
            &[
                MetricId::ParSpread,
                MetricId::FinancingAnnuity,
                MetricId::Ir01,
            ],
        )
        .unwrap();
    assert!(result.value.amount().is_finite());

    let par_spread = *result.measures.get("par_spread").unwrap();
    let annuity = *result.measures.get("financing_annuity").unwrap();
    let ir01 = *result.measures.get("ir01").unwrap();

    // Financing annuity should be positive and bounded
    assert!(annuity > 0.0);
    assert!(annuity <= notional.amount() * 1.05);
    // IR01 proxy is positive
    assert!(ir01 > 0.0);

    // Apply computed par spread and expect PV ~ 0
    let underlying2 = IndexUnderlyingParams::new("HY.BOND.INDEX", USD)
        .with_yield("HY-INDEX-YIELD")
        .with_duration("HY-INDEX-DURATION")
        .with_contract_size(1.0);
    let trs_par = FIIndexTotalReturnSwap::builder()
        .id("TRS-HY-001-PAR".into())
        .notional(notional)
        .underlying(underlying2)
        .financing(
            finstack_valuations::instruments::trs::FinancingLegSpec::new(
                "USD-OIS",
                "USD-SOFR-3M",
                par_spread,
                DayCount::Act360,
            ),
        )
        .schedule(
            finstack_valuations::instruments::trs::TrsScheduleSpec::from_params(
                as_of,
                Date::from_calendar_date(2026, Month::January, 2).unwrap(),
                ScheduleParams::quarterly_act360(),
            ),
        )
        .side(TrsSide::ReceiveTotalReturn)
        .build()
        .unwrap();
    let pv_par = trs_par.value(&context, as_of).unwrap();
    assert!(pv_par.amount().abs() < 1.0, "PV at par should be ~0, got {}", pv_par.amount());
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
        .financing(
            finstack_valuations::instruments::trs::FinancingLegSpec::new(
                "USD-OIS",
                "USD-SOFR-3M",
                0.0,
                DayCount::Act360,
            ),
        )
        .schedule(
            finstack_valuations::instruments::trs::TrsScheduleSpec::from_params(
                as_of,
                Date::from_calendar_date(2026, Month::January, 2).unwrap(),
                ScheduleParams::quarterly_act360(),
            ),
        )
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
        .financing(
            finstack_valuations::instruments::trs::FinancingLegSpec::new(
                "USD-OIS",
                "USD-SOFR-3M",
                0.0,
                DayCount::Act360,
            ),
        )
        .schedule(
            finstack_valuations::instruments::trs::TrsScheduleSpec::from_params(
                as_of,
                Date::from_calendar_date(2026, Month::January, 2).unwrap(),
                ScheduleParams::quarterly_act360(),
            ),
        )
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
        .financing(
            finstack_valuations::instruments::trs::FinancingLegSpec::new(
                "USD-OIS",
                "USD-SOFR-3M",
                50.0,
                DayCount::Act360,
            ),
        )
        .schedule(
            finstack_valuations::instruments::trs::TrsScheduleSpec::from_params(
                as_of,
                Date::from_calendar_date(2026, Month::January, 2).unwrap(),
                ScheduleParams::quarterly_act360(),
            ),
        )
        .side(TrsSide::ReceiveTotalReturn)
        .build()
        .unwrap();

    // Pay total return
    let trs_pay = EquityTotalReturnSwap::builder()
        .id("TRS-PAY".into())
        .notional(notional)
        .underlying(underlying)
        .financing(
            finstack_valuations::instruments::trs::FinancingLegSpec::new(
                "USD-OIS",
                "USD-SOFR-3M",
                50.0,
                DayCount::Act360,
            ),
        )
        .schedule(
            finstack_valuations::instruments::trs::TrsScheduleSpec::from_params(
                as_of,
                Date::from_calendar_date(2026, Month::January, 2).unwrap(),
                ScheduleParams::quarterly_act360(),
            ),
        )
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
