use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::money::Money;
use finstack_valuations::instruments::structured_credit::components::coverage_tests::{CoverageTest, TestContext};
use finstack_valuations::instruments::structured_credit::components::tranche_valuation::{
    calculate_tranche_cs01, calculate_tranche_duration, calculate_tranche_wal, calculate_tranche_z_spread, TrancheCashflowResult,
};
use finstack_valuations::instruments::structured_credit::components::tranches::{Tranche, TrancheCoupon, TrancheStructure};
use finstack_valuations::instruments::structured_credit::components::waterfall::WaterfallBuilder;
use finstack_valuations::instruments::structured_credit::{components::pool::AssetPool, DealType, TrancheSeniority};
use time::Month;

fn d(y: i32, m: u8, day: u8) -> Date {
    Date::from_calendar_date(y, Month::try_from(m).unwrap(), day).unwrap()
}

fn flat_disc(rate: f64, base: Date, id: &str) -> DiscountCurve {
    DiscountCurve::builder(id)
        .base_date(base)
        .knots(vec![(0.0, 1.0), (1.0, (-rate).exp()), (5.0, (-rate * 5.0).exp())])
        .build()
        .unwrap()
}

#[test]
fn tranche_wal_duration_z_cs01_basic() {
    // Build simple principal-only cashflows
    let as_of = d(2025, 1, 1);
    let cashflows = TrancheCashflowResult {
        tranche_id: "A".to_string(),
        cashflows: vec![],
        detailed_flows: vec![],
        interest_flows: vec![],
        principal_flows: vec![(d(2026, 1, 1), Money::new(50_000.0, Currency::USD)), (d(2027, 1, 1), Money::new(50_000.0, Currency::USD))],
        pik_flows: vec![],
        final_balance: Money::new(0.0, Currency::USD),
        total_interest: Money::new(0.0, Currency::USD),
        total_principal: Money::new(100_000.0, Currency::USD),
        total_pik: Money::new(0.0, Currency::USD),
    };

    let wal = calculate_tranche_wal(&cashflows, as_of).unwrap();
    assert!(wal > 0.0 && wal < 3.0);

    // Duration and z-spread on the same flows
    let disc = flat_disc(0.04, as_of, "USD-OIS");
    let pv = Money::new(90_000.0, Currency::USD);
    let dur = calculate_tranche_duration(&cashflows.principal_flows, &disc, as_of, pv).unwrap();
    assert!(dur > 0.0);

    let z_bp = calculate_tranche_z_spread(&cashflows.principal_flows, &disc, pv, as_of).unwrap();
    assert!(z_bp.is_finite());

    let cs01 = calculate_tranche_cs01(&cashflows.principal_flows, &disc, z_bp / 10_000.0, as_of).unwrap();
    assert!(cs01.abs() > 0.0);
}

#[test]
fn waterfall_diversion_triggers_equity_to_senior() {
    // Tranche stack
    let senior = Tranche::new(
        "SENIOR",
        10.0,
        100.0,
        TrancheSeniority::Senior,
        Money::new(8_000_000.0, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.05 },
        d(2035, 1, 1),
    )
    .unwrap();
    let equity = Tranche::new(
        "EQUITY",
        0.0,
        10.0,
        TrancheSeniority::Equity,
        Money::new(2_000_000.0, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.0 },
        d(2035, 1, 1),
    )
    .unwrap();
    let tranches = TrancheStructure::new(vec![equity, senior]).unwrap();

    // Waterfall with equity residual and senior principal payment; set diversion triggers
    let mut wf = WaterfallBuilder::new(Currency::USD)
        .add_tranche_principal("SENIOR")
        .add_equity_distribution()
        .add_oc_ic_trigger("SENIOR", Some(1.20), Some(1.10))
        .build();

    // Apply with low OC so diversion activates
    let pool = AssetPool::new("POOL", DealType::CLO, Currency::USD);
    let market = MarketContext::new();
    let res = wf
        .apply_waterfall(
            Money::new(100_000.0, Currency::USD),
            Money::new(0.0, Currency::USD),
            d(2025, 1, 1),
            &tranches,
            Money::new(10_000_000.0, Currency::USD),
            &pool,
            &market,
        )
        .unwrap();

    // Expect diversion flag and distributions present
    assert!(res.had_diversions);
    assert!(res.distributions.values().map(|m| m.amount()).sum::<f64>() > 0.0);
}

#[test]
fn coverage_test_context_ic_and_oc_paths() {
    // Minimal pool and tranches
    let pool = AssetPool::new("POOL", DealType::CLO, Currency::USD);
    let senior = Tranche::new(
        "SENIOR",
        10.0,
        100.0,
        TrancheSeniority::Senior,
        Money::new(8_000_000.0, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.05 },
        d(2035, 1, 1),
    )
    .unwrap();
    let equity = Tranche::new(
        "EQUITY",
        0.0,
        10.0,
        TrancheSeniority::Equity,
        Money::new(2_000_000.0, Currency::USD),
        TrancheCoupon::Fixed { rate: 0.0 },
        d(2035, 1, 1),
    )
    .unwrap();
    let tranches = TrancheStructure::new(vec![equity, senior]).unwrap();

    // OC test failing
    let ctx = TestContext {
        pool: &pool,
        tranches: &tranches,
        tranche_id: "SENIOR",
        as_of: d(2025, 1, 1),
        cash_balance: Money::new(0.0, Currency::USD),
        interest_collections: Money::new(0.0, Currency::USD),
    };
    let oc = CoverageTest::new_oc(1.25).calculate(&ctx);
    assert!(!oc.is_passing);

    // IC test passing with sufficient collections
    let ctx_ic = TestContext { interest_collections: Money::new(1_000_000.0, Currency::USD), ..ctx };
    let ic = CoverageTest::new_ic(1.10).calculate(&ctx_ic);
    assert!(ic.is_passing);
}


