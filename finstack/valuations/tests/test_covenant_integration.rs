//! Integration tests for covenant enforcement in private credit instruments.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, Frequency};
use finstack_core::money::Money;
use finstack_valuations::covenants::engine::{CovenantEngine, CovenantSpec, InstrumentMutator};
use finstack_valuations::instruments::fixed_income::loan::covenants::{
    Covenant, CovenantConsequence, CovenantType,
};
use finstack_valuations::instruments::fixed_income::loan::{InterestSpec, Loan};
use finstack_valuations::metrics::{MetricContext, MetricId};
use std::sync::Arc;
use time::Month;

#[test]
fn test_loan_covenant_breach_rate_increase() {
    // Create a loan with a leverage covenant
    let mut loan = Loan::new(
        "LOAN-COVENANT-INTEGRATION",
        Money::new(10_000_000.0, Currency::USD),
        Date::from_calendar_date(2025, Month::January, 1).unwrap(),
        Date::from_calendar_date(2030, Month::January, 1).unwrap(),
        InterestSpec::Fixed {
            rate: 0.055,
            step_ups: None,
        },
    );

    // Create covenant with rate increase consequence
    let covenant = Covenant::new(
        CovenantType::MaxDebtToEBITDA { threshold: 4.0 },
        Frequency::quarterly(),
    )
    .with_consequence(CovenantConsequence::RateIncrease { bp_increase: 100.0 });

    loan = loan.with_covenant(covenant.clone());

    // Create covenant engine and add the covenant spec
    let mut engine = CovenantEngine::new();
    engine.add_spec(CovenantSpec::with_metric(
        covenant,
        MetricId::custom("debt_to_ebitda"),
    ));

    // Simulate covenant breach by registering a custom metric that fails the test
    engine.register_metric("debt_to_ebitda", |_ctx| Ok(5.0)); // 5.0 > 4.0 threshold

    // Create metric context for evaluation
    let curves = finstack_core::market_data::multicurve::CurveSet::new();
    let test_date = Date::from_calendar_date(2025, Month::March, 31).unwrap();
    let mut metric_ctx = MetricContext::new(
        Arc::new(loan.clone()),
        Arc::new(curves),
        test_date,
        Money::new(0.0, Currency::USD),
    );

    // Evaluate covenants
    let reports = engine.evaluate(&mut metric_ctx, test_date).unwrap();

    // Verify covenant failed
    let debt_ebitda_report = reports.get("Debt/EBITDA <= 4.00").unwrap();
    assert!(!debt_ebitda_report.passed);
    assert_eq!(debt_ebitda_report.actual_value, Some(5.0));
    assert_eq!(debt_ebitda_report.threshold, Some(4.0));

    // Apply consequences to the loan
    let breaches = vec![finstack_valuations::covenants::engine::CovenantBreach {
        covenant_type: "Debt/EBITDA <= 4.00".to_string(),
        breach_date: test_date,
        actual_value: Some(5.0),
        threshold: Some(4.0),
        cure_deadline: None,
        is_cured: false,
        applied_consequences: vec![],
    }];

    let applications = engine
        .apply_consequences(&mut loan, &breaches, test_date)
        .unwrap();

    // Verify consequence was applied
    assert_eq!(applications.len(), 1);
    assert_eq!(applications[0].consequence_type, "Rate Increase");

    // Verify loan rate was increased
    match &loan.interest {
        InterestSpec::Fixed { step_ups, .. } => {
            assert!(step_ups.is_some());
            let steps = step_ups.as_ref().unwrap();
            assert_eq!(steps.len(), 1);
            assert_eq!(steps[0].1, 0.065); // Original 5.5% + 1% = 6.5%
        }
        _ => panic!("Expected Fixed interest"),
    }
}

#[test]
fn test_loan_cash_sweep_simulation() {
    // Create loan with cash sweep
    let mut loan = Loan::new(
        "LOAN-CASH-SWEEP",
        Money::new(5_000_000.0, Currency::USD),
        Date::from_calendar_date(2025, Month::January, 1).unwrap(),
        Date::from_calendar_date(2028, Month::January, 1).unwrap(),
        InterestSpec::Fixed {
            rate: 0.08,
            step_ups: None,
        },
    );

    // Apply 50% cash sweep
    loan.set_cash_sweep(0.5).unwrap();

    // Verify cash sweep was applied
    assert_eq!(loan.cash_sweep_pct, 0.5);

    // Test that the cash sweep affects future behavior
    // (Full simulation would require LoanFacility implementation for Loan)
}

#[test]
fn test_revolver_utilization_fee_with_covenant_rate_increase() {
    use finstack_valuations::instruments::fixed_income::loan::{
        RevolvingCreditFacility, UtilizationFeeSchedule,
    };

    let mut revolver = RevolvingCreditFacility::new(
        "RCF-UTIL-COVENANT",
        Money::new(20_000_000.0, Currency::USD),
        Date::from_calendar_date(2025, Month::January, 1).unwrap(),
        Date::from_calendar_date(2028, Month::January, 1).unwrap(),
        Date::from_calendar_date(2030, Month::January, 1).unwrap(),
    )
    .with_utilization_fees(
        UtilizationFeeSchedule::new()
            .with_tier(0.0, 0.5, 25.0) // <50% utilization: 25bps
            .with_tier(0.5, 1.0, 50.0), // 50%+ utilization: 50bps
    );

    // Initial state
    assert_eq!(revolver.utilization(), 0.0);
    assert_eq!(revolver.cash_sweep_pct, 0.0);

    // Apply covenant consequences
    revolver.increase_rate(0.0075).unwrap(); // 75bps rate increase
    revolver.set_cash_sweep(0.3).unwrap(); // 30% cash sweep

    // Verify changes
    match &revolver.interest_spec {
        InterestSpec::Floating { spread_step_ups, .. } => {
            assert!(spread_step_ups.is_some());
            let steps = spread_step_ups.as_ref().unwrap();
            assert_eq!(steps[0].1, 325.0); // 250bps + 75bps = 325bps
        }
        _ => panic!("Expected Floating interest"),
    }
    assert_eq!(revolver.cash_sweep_pct, 0.3);
}
