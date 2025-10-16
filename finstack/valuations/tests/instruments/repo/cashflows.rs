//! Tests for repo cashflow schedule generation.

use super::fixtures::*;
use finstack_core::prelude::*;
use finstack_valuations::cashflow::traits::CashflowProvider;
use finstack_valuations::instruments::repo::Repo;

#[test]
fn test_cashflow_schedule_structure() {
    let context = create_standard_market_context();
    let collateral = treasury_collateral();

    let repo = Repo::term(
        "CF_STRUCTURE",
        Money::new(1_000_000.0, Currency::USD),
        collateral,
        0.05,
        date(2025, 1, 15),
        date(2025, 4, 15),
        "USD-OIS",
    );

    let cashflows = repo.build_schedule(&context, date(2025, 1, 10)).unwrap();

    // Should have exactly 2 cashflows: initial outflow and final inflow
    assert_eq!(cashflows.len(), 2);
}

#[test]
fn test_initial_cashflow_negative() {
    let context = create_standard_market_context();
    let collateral = treasury_collateral();

    let repo = Repo::term(
        "CF_INITIAL",
        Money::new(1_000_000.0, Currency::USD),
        collateral,
        0.05,
        date(2025, 1, 15),
        date(2025, 4, 15),
        "USD-OIS",
    );

    let cashflows = repo.build_schedule(&context, date(2025, 1, 10)).unwrap();

    let (start_date, cash_outflow) = &cashflows[0];

    assert_eq!(*start_date, date(2025, 1, 15));
    assert_eq!(
        cash_outflow.amount(),
        -1_000_000.0,
        "Initial flow should be negative (outflow)"
    );
    assert_eq!(cash_outflow.currency(), Currency::USD);
}

#[test]
fn test_final_cashflow_includes_interest() {
    let context = create_standard_market_context();
    let collateral = treasury_collateral();

    let repo = Repo::term(
        "CF_FINAL",
        Money::new(1_000_000.0, Currency::USD),
        collateral,
        0.05,
        date(2025, 1, 15),
        date(2025, 4, 15),
        "USD-OIS",
    );

    let cashflows = repo.build_schedule(&context, date(2025, 1, 10)).unwrap();

    let (maturity_date, cash_inflow) = &cashflows[1];

    assert_eq!(*maturity_date, date(2025, 4, 15));
    assert!(
        cash_inflow.amount() > 1_000_000.0,
        "Final flow should include interest"
    );
    assert_eq!(cash_inflow.currency(), Currency::USD);

    // Verify it matches total repayment
    let expected_repayment = repo.total_repayment().unwrap();
    assert_money_approx_eq(*cash_inflow, expected_repayment, 0.01);
}

#[test]
fn test_cashflow_dates_match_repo_dates() {
    let context = create_standard_market_context();
    let collateral = treasury_collateral();

    let start = date(2025, 2, 1);
    let maturity = date(2025, 8, 1);

    let repo = Repo::term(
        "CF_DATES",
        Money::new(2_000_000.0, Currency::USD),
        collateral,
        0.045,
        start,
        maturity,
        "USD-OIS",
    );

    let cashflows = repo.build_schedule(&context, date(2025, 1, 15)).unwrap();

    assert_eq!(
        cashflows[0].0, start,
        "First cashflow should be at start date"
    );
    assert_eq!(
        cashflows[1].0, maturity,
        "Second cashflow should be at maturity"
    );
}

#[test]
fn test_cashflow_net_present_value() {
    let context = create_standard_market_context();
    let collateral = treasury_collateral();

    let repo = Repo::term(
        "CF_NPV",
        Money::new(1_000_000.0, Currency::USD),
        collateral,
        0.05,
        date(2025, 1, 15),
        date(2025, 4, 15),
        "USD-OIS",
    );

    let cashflows = repo.build_schedule(&context, date(2025, 1, 10)).unwrap();

    // Net cashflow should equal interest amount (ignoring time value)
    let net_undiscounted = cashflows[0].1.checked_add(cashflows[1].1).unwrap();
    let interest = repo.interest_amount().unwrap();

    assert_money_approx_eq(net_undiscounted, interest, 0.01);
}

#[test]
fn test_zero_rate_cashflows() {
    let context = create_standard_market_context();
    let collateral = treasury_collateral();

    let repo = Repo::term(
        "CF_ZERO",
        Money::new(1_000_000.0, Currency::USD),
        collateral,
        0.0, // Zero rate
        date(2025, 1, 15),
        date(2025, 4, 15),
        "USD-OIS",
    );

    let cashflows = repo.build_schedule(&context, date(2025, 1, 10)).unwrap();

    // Final cashflow should equal principal (no interest)
    assert_eq!(cashflows[1].1.amount(), 1_000_000.0);
}

#[test]
fn test_overnight_repo_cashflows() {
    let context = create_standard_market_context();
    let collateral = treasury_collateral();

    let repo = Repo::overnight(
        "CF_OVERNIGHT",
        Money::new(5_000_000.0, Currency::USD),
        collateral,
        0.05,
        date(2025, 1, 15),
        "USD-OIS",
    )
    .unwrap();

    let cashflows = repo.build_schedule(&context, date(2025, 1, 14)).unwrap();

    assert_eq!(cashflows.len(), 2);
    assert_eq!(cashflows[0].0, date(2025, 1, 15));
    assert!(cashflows[1].0 > date(2025, 1, 15), "Maturity after start");

    // Small interest for overnight
    let interest = cashflows[1].1.amount() - 5_000_000.0;
    assert!(interest > 0.0);
    assert!(interest < 1000.0, "Overnight interest should be small");
}

#[test]
fn test_cashflows_currency_consistency() {
    let context = create_standard_market_context();
    let collateral = treasury_collateral();

    let repo = Repo::term(
        "CF_CURRENCY",
        Money::new(1_000_000.0, Currency::EUR),
        collateral,
        0.035,
        date(2025, 1, 15),
        date(2025, 4, 15),
        "USD-OIS",
    );

    let cashflows = repo.build_schedule(&context, date(2025, 1, 10)).unwrap();

    // All cashflows should be in EUR
    assert_eq!(cashflows[0].1.currency(), Currency::EUR);
    assert_eq!(cashflows[1].1.currency(), Currency::EUR);
}

#[test]
fn test_large_notional_cashflows() {
    let context = create_standard_market_context();
    let collateral = treasury_collateral();

    let repo = Repo::term(
        "CF_LARGE",
        Money::new(100_000_000.0, Currency::USD), // 100 million
        collateral,
        0.05,
        date(2025, 1, 15),
        date(2025, 4, 15),
        "USD-OIS",
    );

    let cashflows = repo.build_schedule(&context, date(2025, 1, 10)).unwrap();

    assert_eq!(cashflows[0].1.amount(), -100_000_000.0);
    assert!(cashflows[1].1.amount() > 100_000_000.0);

    // Interest should be proportionally scaled
    let interest = repo.interest_amount().unwrap();
    assert!(interest.amount() > 1_000_000.0);
}
