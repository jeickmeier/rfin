//! Tests for private credit instruments.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Frequency};
use finstack_core::market_data::multicurve::CurveSet;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
use finstack_core::money::Money;
use finstack_valuations::cashflow::primitives::AmortizationSpec;
use finstack_valuations::cashflow::traits::CashflowProvider;
use finstack_valuations::instruments::fixed_income::loan::ddtl::ExpectedFundingCurve;
use finstack_valuations::instruments::fixed_income::loan::revolver::{
    DrawRepayEvent, RevolverFundingCurve,
};
use finstack_valuations::instruments::fixed_income::loan::{
    Covenant, CovenantType, DelayedDrawTermLoan, DrawEvent, InterestSpec, Loan, PenaltyType,
    PrepaymentPenalty, PrepaymentSchedule, PrepaymentType, RevolvingCreditFacility,
    UtilizationFeeSchedule,
};
use finstack_valuations::instruments::traits::Priceable;
use std::sync::Arc;
use time::Month;

fn setup_curves(base: Date) -> Arc<CurveSet> {
    let disc = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots([
            (0.0, 1.0),
            (1.0, 0.97),
            (2.0, 0.94),
            (5.0, 0.85),
            (10.0, 0.70),
        ])
        .linear_df()
        .build()
        .unwrap();

    let fwd = ForwardCurve::builder("USD-SOFR-3M", 0.25) // 3-month tenor = 0.25 years
        .base_date(base)
        .knots([
            (0.0, 0.05),
            (1.0, 0.055),
            (2.0, 0.06),
            (5.0, 0.065),
            (10.0, 0.07),
        ])
        .linear_df()
        .build()
        .unwrap();

    Arc::new(CurveSet::new().with_discount(disc).with_forecast(fwd))
}

#[test]
fn test_loan_fixed_rate() {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();
    let curves = setup_curves(issue);

    let loan = Loan::new(
        "LOAN-001",
        Money::new(10_000_000.0, Currency::USD),
        issue,
        maturity,
        InterestSpec::Fixed {
            rate: 0.065,
            step_ups: None,
        },
    )
    .with_frequency(Frequency::quarterly())
    .with_day_count(DayCount::Act360);

    // Build cashflows
    let flows = loan.build_schedule(&curves, issue).unwrap();
    assert!(!flows.is_empty());

    // Compute NPV
    let value = loan.value(&curves, issue).unwrap();
    assert!(value.amount() > 0.0);
}

#[test]
fn test_loan_with_step_ups() {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();
    let curves = setup_curves(issue);

    let step_ups = vec![
        (
            Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            0.07,
        ),
        (
            Date::from_calendar_date(2027, Month::January, 1).unwrap(),
            0.075,
        ),
        (
            Date::from_calendar_date(2028, Month::January, 1).unwrap(),
            0.08,
        ),
    ];

    let loan = Loan::new(
        "LOAN-002",
        Money::new(5_000_000.0, Currency::USD),
        issue,
        maturity,
        InterestSpec::Fixed {
            rate: 0.065,
            step_ups: Some(step_ups),
        },
    );

    let flows = loan.build_schedule(&curves, issue).unwrap();
    assert!(!flows.is_empty());

    // Value should increase with step-ups
    let value = loan.value(&curves, issue).unwrap();
    assert!(value.amount() > 0.0);
}

#[test]
fn test_loan_with_amortization() {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();
    let curves = setup_curves(issue);

    let loan = Loan::new(
        "LOAN-003",
        Money::new(10_000_000.0, Currency::USD),
        issue,
        maturity,
        InterestSpec::Fixed {
            rate: 0.065,
            step_ups: None,
        },
    )
    .with_amortization(AmortizationSpec::LinearTo {
        final_notional: Money::new(0.0, Currency::USD),
    });

    let flows = loan.build_schedule(&curves, issue).unwrap();

    // Should have both interest and principal payments
    let principal_flows: Vec<_> = flows
        .iter()
        .filter(|(_, amount)| amount.amount() > 0.0)
        .collect();
    assert!(!principal_flows.is_empty());
}

#[test]
fn test_loan_pik_toggle() {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();
    let curves = setup_curves(issue);

    let toggle_schedule = vec![
        (
            Date::from_calendar_date(2025, Month::January, 1).unwrap(),
            false,
        ), // Cash
        (
            Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            true,
        ), // PIK
        (
            Date::from_calendar_date(2027, Month::January, 1).unwrap(),
            false,
        ), // Back to Cash
    ];

    let loan = Loan::new(
        "LOAN-004",
        Money::new(5_000_000.0, Currency::USD),
        issue,
        maturity,
        InterestSpec::PIKToggle {
            cash_rate: 0.06,
            pik_rate: 0.065,
            toggle_schedule,
        },
    );

    let flows = loan.build_schedule(&curves, issue).unwrap();
    assert!(!flows.is_empty());
}

#[test]
fn test_loan_with_prepayment() {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

    let prepayment = PrepaymentSchedule::new(PrepaymentType::SoftCall { premium_pct: 0.02 })
        .with_lockout(
            issue,
            Date::from_calendar_date(2026, Month::January, 1).unwrap(),
        )
        .with_penalty(PrepaymentPenalty {
            start: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            end: Some(Date::from_calendar_date(2028, Month::January, 1).unwrap()),
            penalty: PenaltyType::Percentage(0.03),
        });

    let _loan = Loan::new(
        "LOAN-005",
        Money::new(10_000_000.0, Currency::USD),
        issue,
        maturity,
        InterestSpec::Fixed {
            rate: 0.065,
            step_ups: None,
        },
    )
    .with_prepayment(prepayment.clone());

    // Test lockout period
    let lockout_date = Date::from_calendar_date(2025, Month::June, 1).unwrap();
    assert!(!prepayment.is_prepayment_allowed(lockout_date));

    // Test after lockout
    let after_lockout = Date::from_calendar_date(2026, Month::June, 1).unwrap();
    assert!(prepayment.is_prepayment_allowed(after_lockout));

    // Test penalty calculation
    let penalty_amount = prepayment
        .calculate_penalty(after_lockout, Money::new(1_000_000.0, Currency::USD))
        .unwrap();
    assert_eq!(penalty_amount.amount(), 30_000.0); // 3% of 1M
}

#[test]
fn test_ddtl_with_draws() {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let commitment_expiry = Date::from_calendar_date(2027, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();
    let curves = setup_curves(issue);

    let ddtl = DelayedDrawTermLoan::new(
        "DDTL-001",
        Money::new(20_000_000.0, Currency::USD),
        commitment_expiry,
        maturity,
        InterestSpec::Fixed {
            rate: 0.07,
            step_ups: None,
        },
    )
    .with_commitment_fee(0.0035) // 35 bps
    .with_draw(DrawEvent {
        date: Date::from_calendar_date(2025, Month::June, 1).unwrap(),
        amount: Money::new(5_000_000.0, Currency::USD),
        purpose: Some("Working capital".to_string()),
        conditional: false,
    })
    .with_draw(DrawEvent {
        date: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
        amount: Money::new(10_000_000.0, Currency::USD),
        purpose: Some("Acquisition".to_string()),
        conditional: false,
    });

    // Test undrawn amount
    assert_eq!(ddtl.undrawn_amount().amount(), 20_000_000.0);

    // Build cashflows
    let as_of = Date::from_calendar_date(2026, Month::June, 1).unwrap();
    let flows = ddtl.build_schedule(&curves, as_of).unwrap();
    assert!(!flows.is_empty());

    // Compute value
    let value = ddtl.value(&curves, as_of).unwrap();
    assert!(value.amount() != 0.0); // Can be negative due to commitment fees
}

#[test]
fn test_revolver_with_utilization_fees() {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let availability_end = Date::from_calendar_date(2028, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();
    let curves = setup_curves(issue);

    let util_fees = UtilizationFeeSchedule::new()
        .with_tier(0.0, 0.33, 10.0) // < 33%: 10 bps
        .with_tier(0.33, 0.66, 15.0) // 33-66%: 15 bps
        .with_tier(0.66, 1.0, 25.0); // > 66%: 25 bps

    let revolver = RevolvingCreditFacility::new(
        "RCF-001",
        Money::new(50_000_000.0, Currency::USD),
        issue,
        availability_end,
        maturity,
    )
    .with_commitment_fee(0.0025) // 25 bps
    .with_utilization_fees(util_fees.clone())
    .with_event(DrawRepayEvent {
        date: Date::from_calendar_date(2025, Month::March, 1).unwrap(),
        amount: Money::new(10_000_000.0, Currency::USD),
        mandatory: false,
        description: Some("Draw 1".to_string()),
    })
    .with_event(DrawRepayEvent {
        date: Date::from_calendar_date(2025, Month::June, 1).unwrap(),
        amount: Money::new(15_000_000.0, Currency::USD),
        mandatory: false,
        description: Some("Draw 2".to_string()),
    })
    .with_event(DrawRepayEvent {
        date: Date::from_calendar_date(2025, Month::September, 1).unwrap(),
        amount: Money::new(-5_000_000.0, Currency::USD),
        mandatory: true,
        description: Some("Mandatory repayment".to_string()),
    });

    // Test utilization calculation
    assert_eq!(revolver.utilization(), 0.0);

    // Test fee schedule
    assert_eq!(util_fees.get_rate(0.2), 10.0);
    assert_eq!(util_fees.get_rate(0.5), 15.0);
    assert_eq!(util_fees.get_rate(0.8), 25.0);

    // Build cashflows
    let as_of = Date::from_calendar_date(2025, Month::October, 1).unwrap();
    let flows = revolver.build_schedule(&curves, as_of).unwrap();
    assert!(!flows.is_empty());

    // Compute value with metrics
    let result = revolver.price_with_metrics(&curves, as_of, &[]).unwrap();
    assert!(result.measures.contains_key("utilization"));
    assert!(result.measures.contains_key("drawn"));
    assert!(result.measures.contains_key("undrawn"));
}

#[test]
fn test_loan_with_covenants() {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

    let leverage_covenant = Covenant::new(
        CovenantType::MaxDebtToEBITDA { threshold: 4.0 },
        Frequency::quarterly(),
    )
    .with_cure_period(Some(30));

    let coverage_covenant = Covenant::new(
        CovenantType::MinInterestCoverage { threshold: 2.5 },
        Frequency::quarterly(),
    )
    .with_cure_period(Some(15));

    let loan = Loan::new(
        "LOAN-COV",
        Money::new(15_000_000.0, Currency::USD),
        issue,
        maturity,
        InterestSpec::Fixed {
            rate: 0.065,
            step_ups: None,
        },
    )
    .with_covenant(leverage_covenant.clone())
    .with_covenant(coverage_covenant.clone());

    // Test covenant descriptions
    assert_eq!(leverage_covenant.description(), "Debt/EBITDA ≤ 4.00x");
    assert_eq!(coverage_covenant.description(), "Interest Coverage ≥ 2.50x");

    assert_eq!(loan.covenants.len(), 2);
}

#[test]
fn test_floating_rate_loan() {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();
    let curves = setup_curves(issue);

    let loan = Loan::new(
        "LOAN-FLOAT",
        Money::new(10_000_000.0, Currency::USD),
        issue,
        maturity,
        InterestSpec::Floating {
            index_id: "USD-SOFR-3M",
            spread_bp: 250.0,
            spread_step_ups: None,
            gearing: 1.0,
            reset_lag_days: 2,
        },
    );

    let flows = loan.build_schedule(&curves, issue).unwrap();
    assert!(!flows.is_empty());

    let value = loan.value(&curves, issue).unwrap();
    assert!(value.amount() > 0.0);
}

#[test]
fn test_ddtl_with_expected_funding_curve() {
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let curves = setup_curves(base);

    // Create DDTL with expected future draws
    let mut ddtl = DelayedDrawTermLoan::new(
        "DDTL-002",
        Money::new(30_000_000.0, Currency::USD),
        Date::from_calendar_date(2026, Month::December, 31).unwrap(), // Commitment expiry
        Date::from_calendar_date(2030, Month::December, 31).unwrap(), // Maturity
        InterestSpec::Fixed {
            rate: 0.08,
            step_ups: None,
        },
    );
    ddtl.drawn_amount = Money::new(5_000_000.0, Currency::USD); // Already drawn 5M
    let ddtl = ddtl.with_expected_draws(vec![
        DrawEvent {
            date: Date::from_calendar_date(2025, Month::June, 1).unwrap(),
            amount: Money::new(10_000_000.0, Currency::USD),
            purpose: Some("Expansion".to_string()),
            conditional: false,
        },
        DrawEvent {
            date: Date::from_calendar_date(2026, Month::January, 1).unwrap(),
            amount: Money::new(5_000_000.0, Currency::USD),
            purpose: Some("Working capital".to_string()),
            conditional: false,
        },
    ]);

    // Price should include expected future draws
    let value = ddtl.value(&curves, base).unwrap();

    // The value should be negative (from lender perspective) as we're funding future draws
    // but positive from interest payments
    assert!(value.amount() != 0.0);

    // Also test with probabilities
    let ddtl_with_probs = DelayedDrawTermLoan::new(
        "DDTL-003",
        Money::new(30_000_000.0, Currency::USD),
        Date::from_calendar_date(2026, Month::December, 31).unwrap(),
        Date::from_calendar_date(2030, Month::December, 31).unwrap(),
        InterestSpec::Fixed {
            rate: 0.08,
            step_ups: None,
        },
    )
    .with_expected_funding_curve(ExpectedFundingCurve::with_probabilities(
        vec![DrawEvent {
            date: Date::from_calendar_date(2025, Month::June, 1).unwrap(),
            amount: Money::new(10_000_000.0, Currency::USD),
            purpose: None,
            conditional: false,
        }],
        vec![0.75], // 75% probability
    ));

    let value_with_probs = ddtl_with_probs.value(&curves, base).unwrap();
    assert!(value_with_probs.amount() != 0.0);
}

#[test]
fn test_revolver_with_expected_funding_curve() {
    let base = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let curves = setup_curves(base);

    // Create RCF with expected draws and repayments
    let mut rcf = RevolvingCreditFacility::new(
        "RCF-002",
        Money::new(50_000_000.0, Currency::USD),
        Date::from_calendar_date(2025, Month::January, 1).unwrap(),
        Date::from_calendar_date(2026, Month::December, 31).unwrap(),
        Date::from_calendar_date(2028, Month::December, 31).unwrap(),
    )
    .with_interest(InterestSpec::Fixed {
        rate: 0.06,
        step_ups: None,
    });
    rcf.drawn_amount = Money::new(10_000_000.0, Currency::USD); // Currently drawn 10M
    let rcf = rcf.with_expected_events(vec![
        DrawRepayEvent {
            date: Date::from_calendar_date(2025, Month::March, 1).unwrap(),
            amount: Money::new(15_000_000.0, Currency::USD), // Draw 15M
            mandatory: false,
            description: Some("Q1 draw".to_string()),
        },
        DrawRepayEvent {
            date: Date::from_calendar_date(2025, Month::September, 1).unwrap(),
            amount: Money::new(-10_000_000.0, Currency::USD), // Repay 10M
            mandatory: false,
            description: Some("Q3 repayment".to_string()),
        },
        DrawRepayEvent {
            date: Date::from_calendar_date(2026, Month::March, 1).unwrap(),
            amount: Money::new(20_000_000.0, Currency::USD), // Draw 20M
            mandatory: false,
            description: Some("Q1 2026 draw".to_string()),
        },
    ]);

    // Price should reflect expected future funding activity
    let value = rcf.value(&curves, base).unwrap();
    assert!(value.amount() != 0.0);

    // Test with probabilities
    let rcf_with_probs = RevolvingCreditFacility::new(
        "RCF-003",
        Money::new(50_000_000.0, Currency::USD),
        Date::from_calendar_date(2025, Month::January, 1).unwrap(),
        Date::from_calendar_date(2026, Month::December, 31).unwrap(),
        Date::from_calendar_date(2028, Month::December, 31).unwrap(),
    )
    .with_interest(InterestSpec::Fixed {
        rate: 0.06,
        step_ups: None,
    })
    .with_expected_funding_curve(RevolverFundingCurve::with_probabilities(
        vec![DrawRepayEvent {
            date: Date::from_calendar_date(2025, Month::June, 1).unwrap(),
            amount: Money::new(25_000_000.0, Currency::USD),
            mandatory: false,
            description: None,
        }],
        vec![0.5], // 50% probability
    ));

    let value_with_probs = rcf_with_probs.value(&curves, base).unwrap();
    assert!(value_with_probs.amount() != 0.0);
}

#[test]
fn test_cash_plus_pik_loan() {
    let issue = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();
    let curves = setup_curves(issue);

    let loan = Loan::new(
        "LOAN-MIXED",
        Money::new(8_000_000.0, Currency::USD),
        issue,
        maturity,
        InterestSpec::CashPlusPIK {
            cash_rate: 0.04,
            pik_rate: 0.02,
        },
    );

    let flows = loan.build_schedule(&curves, issue).unwrap();
    assert!(!flows.is_empty());

    // Should have both cash and PIK components
    let value = loan.value(&curves, issue).unwrap();
    assert!(value.amount() > 0.0);
}
