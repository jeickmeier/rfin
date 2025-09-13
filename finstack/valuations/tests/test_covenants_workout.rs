//! Integration tests for covenant and workout functionality.

use finstack_core::market_data::MarketContext;
use finstack_core::prelude::*;
use finstack_core::F;
use finstack_valuations::covenants::{
    CovenantBreach, CovenantEngine, CovenantSpec, CovenantWindow, InstrumentMutator,
};
use finstack_valuations::instruments::fixed_income::loan::covenants::{
    Covenant, CovenantConsequence, CovenantType, ThresholdTest,
};
// Removed workout usage; keep only needed imports
use finstack_valuations::metrics::{MetricContext, MetricId};
// policy and workout modules removed from valuations scope
// HashMap no longer required after removing workout/policy tests
use std::sync::Arc;
use time::Month;

// Mock instrument mutator for testing
struct MockLoan {
    #[allow(dead_code)]
    pub id: String,
    pub rate: F,
    pub maturity: Date,
    pub is_default: bool,
    pub cash_sweep_pct: F,
    pub distributions_blocked: bool,
}

impl InstrumentMutator for MockLoan {
    fn set_default_status(&mut self, is_default: bool, _as_of: Date) -> finstack_core::Result<()> {
        self.is_default = is_default;
        Ok(())
    }

    fn increase_rate(&mut self, increase: F) -> finstack_core::Result<()> {
        self.rate += increase;
        Ok(())
    }

    fn set_cash_sweep(&mut self, percentage: F) -> finstack_core::Result<()> {
        self.cash_sweep_pct = percentage;
        Ok(())
    }

    fn set_distribution_block(&mut self, blocked: bool) -> finstack_core::Result<()> {
        self.distributions_blocked = blocked;
        Ok(())
    }

    fn set_maturity(&mut self, new_maturity: Date) -> finstack_core::Result<()> {
        self.maturity = new_maturity;
        Ok(())
    }
}

#[test]
fn test_covenant_evaluation() {
    // Setup covenant engine
    let mut engine = CovenantEngine::new();

    // Add DSCR covenant
    let dscr_covenant = Covenant {
        covenant_type: CovenantType::Custom {
            metric: "dscr".to_string(),
            test: ThresholdTest::Minimum(1.25),
        },
        test_frequency: finstack_core::dates::Frequency::quarterly(),
        cure_period_days: Some(30),
        consequences: vec![CovenantConsequence::RateIncrease { bp_increase: 100.0 }],
        is_active: true,
    };

    engine.add_spec(CovenantSpec::with_metric(
        dscr_covenant.clone(),
        MetricId::custom("dscr"),
    ));

    // Register custom DSCR calculator
    engine.register_metric("dscr", |_context| {
        Ok(1.15) // Below threshold
    });

    // Setup context
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let curves = Arc::new(MarketContext::new());
    let mut context = MetricContext::new(
        Arc::new(finstack_valuations::instruments::fixed_income::bond::Bond {
            id: "TEST".to_string().into(),
            notional: Money::new(1_000_000.0, Currency::USD),
            coupon: 0.05,
            freq: finstack_core::dates::Frequency::semi_annual(),
            dc: DayCount::Act365F,
            issue: as_of,
            maturity: Date::from_calendar_date(2030, Month::January, 1).unwrap(),
            disc_id: "USD-OIS".into(),
            quoted_clean: None,
            call_put: None,
            amortization: None,
            custom_cashflows: None,
            attributes: finstack_valuations::instruments::traits::Attributes::new(),
        }),
        curves,
        as_of,
        Money::new(1_000_000.0, Currency::USD),
    );

    // Evaluate covenants
    let reports = engine.evaluate(&mut context, as_of).unwrap();

    // Check that DSCR covenant failed
    assert!(!reports.is_empty());
    let dscr_report = reports.values().next().unwrap();
    assert!(!dscr_report.passed);
    assert_eq!(dscr_report.actual_value, Some(1.15));
    assert_eq!(dscr_report.threshold, Some(1.25));
}

#[test]
fn test_covenant_consequences() {
    let mut engine = CovenantEngine::new();

    // Create a breach
    let breach = CovenantBreach {
        covenant_type: "Debt/EBITDA <= 5.00".to_string(),
        breach_date: Date::from_calendar_date(2025, Month::January, 1).unwrap(),
        actual_value: Some(5.5),
        threshold: Some(5.0),
        cure_deadline: None, // No cure period
        is_cured: false,
        applied_consequences: vec![],
    };

    // Add corresponding covenant spec
    engine.add_spec(CovenantSpec::with_metric(
        Covenant {
            covenant_type: CovenantType::MaxDebtToEBITDA { threshold: 5.0 },
            test_frequency: finstack_core::dates::Frequency::quarterly(),
            cure_period_days: None,
            consequences: vec![
                CovenantConsequence::RateIncrease { bp_increase: 200.0 },
                CovenantConsequence::BlockDistributions,
            ],
            is_active: true,
        },
        MetricId::custom("debt_to_ebitda"),
    ));

    // Apply consequences to mock loan
    let mut loan = MockLoan {
        id: "TEST".to_string(),
        rate: 0.05,
        maturity: Date::from_calendar_date(2030, Month::January, 1).unwrap(),
        is_default: false,
        cash_sweep_pct: 0.0,
        distributions_blocked: false,
    };

    let as_of = Date::from_calendar_date(2025, Month::January, 15).unwrap();
    let applications = engine
        .apply_consequences(&mut loan, &[breach], as_of)
        .unwrap();

    // Check consequences were applied
    assert_eq!(applications.len(), 2);
    assert_eq!(loan.rate, 0.07); // 5% + 200bps
    assert!(loan.distributions_blocked);
}

#[test]
fn test_covenant_windows() {
    let mut engine = CovenantEngine::new();

    // Create covenants for different windows
    let q1_covenant = Covenant {
        covenant_type: CovenantType::MinInterestCoverage { threshold: 2.0 },
        test_frequency: finstack_core::dates::Frequency::quarterly(),
        cure_period_days: Some(30),
        consequences: vec![],
        is_active: true,
    };

    let q2_covenant = Covenant {
        covenant_type: CovenantType::MinInterestCoverage { threshold: 2.5 },
        test_frequency: finstack_core::dates::Frequency::quarterly(),
        cure_period_days: Some(30),
        consequences: vec![],
        is_active: true,
    };

    // Add windows
    engine.add_window(CovenantWindow {
        start: Date::from_calendar_date(2025, Month::January, 1).unwrap(),
        end: Date::from_calendar_date(2025, Month::March, 31).unwrap(),
        covenants: vec![CovenantSpec::with_metric(
            q1_covenant,
            MetricId::custom("interest_coverage"),
        )],
        is_grace_period: false,
    });

    engine.add_window(CovenantWindow {
        start: Date::from_calendar_date(2025, Month::April, 1).unwrap(),
        end: Date::from_calendar_date(2025, Month::June, 30).unwrap(),
        covenants: vec![CovenantSpec::with_metric(
            q2_covenant,
            MetricId::custom("interest_coverage"),
        )],
        is_grace_period: false,
    });

    // Test that correct covenants apply in each window
    let q1_date = Date::from_calendar_date(2025, Month::February, 15).unwrap();
    let q1_specs = engine.get_applicable_specs(q1_date);
    assert_eq!(q1_specs.len(), 1);

    let q2_date = Date::from_calendar_date(2025, Month::May, 15).unwrap();
    let q2_specs = engine.get_applicable_specs(q2_date);
    assert_eq!(q2_specs.len(), 1);
}

// Removed workout and policy tests to align with valuations scope
