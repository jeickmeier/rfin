//! Integration tests for covenant and workout functionality.

use finstack_core::prelude::*;
use finstack_core::F;
use finstack_valuations::covenants::{
    CovenantEngine, CovenantSpec, CovenantWindow, CovenantBreach, InstrumentMutator,
};
use finstack_valuations::workout::{
    WorkoutEngine, WorkoutState, WorkoutPolicy, WorkoutStrategy,
    RateModification, PrincipalModification, RecoveryWaterfall, RecoveryTier,
    ClaimAmount,
};
use finstack_valuations::instruments::loan::covenants::{
    Covenant, CovenantType, CovenantConsequence, ThresholdTest,
};
use finstack_valuations::instruments::loan::term_loan::{Loan, InterestSpec};
use finstack_valuations::metrics::{MetricId, MetricContext};
use finstack_valuations::policy::{
    GridMarginPolicy, IndexFallbackPolicy, DSCRSweepPolicy,
};
use finstack_core::market_data::multicurve::CurveSet;
use std::collections::HashMap;
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
        consequences: vec![
            CovenantConsequence::RateIncrease { bp_increase: 100.0 },
        ],
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
    let curves = Arc::new(CurveSet::new());
    let mut context = MetricContext::new(
        Arc::new(finstack_valuations::instruments::Instrument::Bond(
            finstack_valuations::instruments::bond::Bond {
                id: "TEST".to_string(),
                notional: Money::new(1_000_000.0, Currency::USD),
                coupon: 0.05,
                freq: finstack_core::dates::Frequency::semi_annual(),
                dc: DayCount::Act365F,
                issue: as_of,
                maturity: Date::from_calendar_date(2030, Month::January, 1).unwrap(),
                disc_id: "USD-OIS",
                quoted_clean: None,
                call_put: None,
                amortization: None,
                custom_cashflows: None,
                attributes: finstack_valuations::traits::Attributes::new(),
            }
        )),
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
    let applications = engine.apply_consequences(&mut loan, &[breach], as_of).unwrap();
    
    // Check consequences were applied
    assert_eq!(applications.len(), 2);
    assert_eq!(loan.rate, 0.07); // 5% + 200bps
    assert!(loan.distributions_blocked);
}

#[test]
fn test_workout_state_transitions() {
    let policy = WorkoutPolicy {
        name: "Standard".to_string(),
        stress_thresholds: HashMap::new(),
        default_triggers: vec![],
        workout_strategies: HashMap::new(),
        recovery_waterfall: RecoveryWaterfall { tiers: vec![] },
    };
    
    let mut engine = WorkoutEngine::new(policy);
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    
    // Valid transitions
    assert!(engine.transition(
        WorkoutState::Stressed { indicators: vec!["covenant_breach".to_string()] },
        as_of,
        "Covenant breach detected",
    ).is_ok());
    
    assert!(engine.transition(
        WorkoutState::Default {
            default_date: as_of,
            reason: "Payment missed".to_string(),
        },
        as_of,
        "Payment default",
    ).is_ok());
    
    // Invalid transition (can't go from Default back to Performing directly)
    assert!(engine.transition(
        WorkoutState::Performing,
        as_of,
        "Invalid transition",
    ).is_err());
}

#[test]
fn test_workout_strategy_application() {
    let mut strategies = HashMap::new();
    strategies.insert("forbearance".to_string(), WorkoutStrategy {
        name: "forbearance".to_string(),
        forbearance_months: Some(6),
        rate_modification: Some(RateModification::ReduceBy { bps: 100.0 }),
        principal_modification: Some(PrincipalModification::Defer { percentage: 0.2 }),
        maturity_extension_months: Some(12),
        additional_collateral: None,
        exit_fee_pct: Some(0.02),
    });
    
    let policy = WorkoutPolicy {
        name: "Standard".to_string(),
        stress_thresholds: HashMap::new(),
        default_triggers: vec![],
        workout_strategies: strategies,
        recovery_waterfall: RecoveryWaterfall { tiers: vec![] },
    };
    
    let mut engine = WorkoutEngine::new(policy);
    
    // Create a loan
    let mut loan = Loan::new(
        "TEST",
        Money::new(1_000_000.0, Currency::USD),
        Date::from_calendar_date(2025, Month::January, 1).unwrap(),
        Date::from_calendar_date(2030, Month::January, 1).unwrap(),
        InterestSpec::Fixed { rate: 0.06, step_ups: None },
    );
    
    // Move to default state first
    engine.transition(
        WorkoutState::Default {
            default_date: Date::from_calendar_date(2025, Month::June, 1).unwrap(),
            reason: "Covenant breach".to_string(),
        },
        Date::from_calendar_date(2025, Month::June, 1).unwrap(),
        "Default on covenant",
    ).unwrap();
    
    // Apply workout strategy
    let as_of = Date::from_calendar_date(2025, Month::July, 1).unwrap();
    let application = engine.apply(&mut loan, "forbearance", as_of).unwrap();
    
    // Check modifications
    assert_eq!(application.strategy_name, "forbearance");
    assert_eq!(application.modifications.len(), 3);
    assert!(application.exit_fee.is_some());
    assert!(application.forbearance_end.is_some());
    
    // Check loan modifications (use approximate equality for floating point)
    if let InterestSpec::Fixed { rate, .. } = loan.interest {
        assert!((rate - 0.05).abs() < 0.001); // Reduced by 100bps
    }
    assert_eq!(loan.outstanding.amount(), 800_000.0); // 20% deferred
}

#[test]
fn test_recovery_waterfall() {
    let tiers = vec![
        RecoveryTier {
            name: "Senior Expenses".to_string(),
            claim_type: "expenses".to_string(),
            claim_amount: ClaimAmount::Fixed(Money::new(50_000.0, Currency::USD)),
            recovery_pct: 1.0,
        },
        RecoveryTier {
            name: "Senior Debt".to_string(),
            claim_type: "senior_debt".to_string(),
            claim_amount: ClaimAmount::PercentOfOutstanding(1.0),
            recovery_pct: 0.8,
        },
        RecoveryTier {
            name: "Subordinated Debt".to_string(),
            claim_type: "subordinated_debt".to_string(),
            claim_amount: ClaimAmount::PercentOfOutstanding(0.2),
            recovery_pct: 0.4,
        },
    ];
    
    let policy = WorkoutPolicy {
        name: "Standard".to_string(),
        stress_thresholds: HashMap::new(),
        default_triggers: vec![],
        workout_strategies: HashMap::new(),
        recovery_waterfall: RecoveryWaterfall { tiers },
    };
    
    let mut engine = WorkoutEngine::new(policy);
    
    let outstanding = Money::new(1_000_000.0, Currency::USD);
    let collateral = Money::new(600_000.0, Currency::USD);
    let as_of = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    
    let analysis = engine.generate_recovery_flows(outstanding, collateral, as_of).unwrap();
    
    // Check recovery calculation
    assert_eq!(analysis.tier_recoveries.len(), 3);
    
    // Senior expenses: min(50k * 100%, 600k) = 50k
    assert_eq!(analysis.tier_recoveries[0].1.amount(), 50_000.0);
    
    // Senior debt: min(1M * 80%, 550k) = 550k
    assert_eq!(analysis.tier_recoveries[1].1.amount(), 550_000.0);
    
    // Subordinated: min(200k * 40%, 0) = 0
    assert_eq!(analysis.tier_recoveries[2].1.amount(), 0.0);
    
    assert_eq!(analysis.expected_recovery.amount(), 600_000.0);
    assert_eq!(analysis.recovery_rate, 0.6);
}

#[test]
fn test_grid_margin_policy_integration() {
    let policy = GridMarginPolicy::new();
    
    // Test different rating/sector combinations
    let spread_aaa_tech = policy.calculate_spread("AAA", Some("Technology"), 5.0);
    assert_eq!(spread_aaa_tech, 35.0); // 10 base - 25 tech + 50 maturity
    
    let spread_bb_energy = policy.calculate_spread("BB", Some("Energy"), 10.0);
    assert_eq!(spread_bb_energy, 350.0); // 200 base + 50 energy + 100 maturity
    
    let spread_ccc = policy.calculate_spread("CCC", None, 1.0);
    assert_eq!(spread_ccc, 800.0); // 800 base + 0 maturity
    
    // Test default rating with long maturity (no maturity bucket beyond 10Y)
    let spread_default = policy.calculate_spread("D", None, 20.0);
    assert_eq!(spread_default, 600.0); // 500 default + 100 maturity (highest bucket)
}

#[test]
fn test_index_fallback_policy() {
    let policy = IndexFallbackPolicy::new();
    
    let mut available_rates = HashMap::new();
    
    // Test direct rate available
    available_rates.insert("USD-LIBOR-3M".to_string(), 0.055);
    let rate = policy.get_rate("USD-LIBOR-3M", &available_rates).unwrap();
    assert_eq!(rate, 0.055);
    
    // Test fallback to SOFR with spread
    available_rates.clear();
    available_rates.insert("USD-SOFR-3M".to_string(), 0.045);
    let rate = policy.get_rate("USD-LIBOR-3M", &available_rates).unwrap();
    assert!((rate - 0.0476161).abs() < 0.0001); // 0.045 + 26.161bps
    
    // Test static fallback
    available_rates.clear();
    let rate = policy.get_rate("USD-LIBOR-3M", &available_rates).unwrap();
    assert_eq!(rate, 0.05);
    
    // Test missing index
    let result = policy.get_rate("EUR-EURIBOR-3M", &available_rates);
    assert!(result.is_err());
}

#[test]
fn test_dscr_sweep_policy() {
    let policy = DSCRSweepPolicy::new();
    
    // Test various DSCR levels
    let cases = vec![
        (0.8, 0.0),   // Below minimum
        (1.1, 0.25),  // 25% sweep
        (1.4, 0.50),  // 50% sweep  
        (1.6, 0.75),  // 75% sweep
        (1.8, 0.75),  // Still 75%
        (2.5, 1.0),   // 100% sweep
    ];
    
    for (dscr, expected_sweep) in cases {
        let sweep = policy.calculate_sweep(dscr);
        assert_eq!(sweep, expected_sweep, "DSCR {} should give sweep {}", dscr, expected_sweep);
    }
    
    // Test sweep amount calculation
    let excess = Money::new(500_000.0, Currency::USD);
    let minimum = Money::new(100_000.0, Currency::USD);
    
    let sweep_amount = policy.calculate_sweep_amount(1.5, excess, minimum);
    assert_eq!(sweep_amount.amount(), 200_000.0); // (500k - 100k) * 0.5
    
    // Test currency mismatch
    let excess_eur = Money::new(500_000.0, Currency::EUR);
    let sweep_amount = policy.calculate_sweep_amount(1.5, excess_eur, minimum);
    assert_eq!(sweep_amount.amount(), 0.0);
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
        covenants: vec![CovenantSpec::with_metric(q1_covenant, MetricId::custom("interest_coverage"))],
        is_grace_period: false,
    });
    
    engine.add_window(CovenantWindow {
        start: Date::from_calendar_date(2025, Month::April, 1).unwrap(),
        end: Date::from_calendar_date(2025, Month::June, 30).unwrap(),
        covenants: vec![CovenantSpec::with_metric(q2_covenant, MetricId::custom("interest_coverage"))],
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

#[test]
fn test_workout_penalty_flows() {
    let policy = WorkoutPolicy {
        name: "Standard".to_string(),
        stress_thresholds: HashMap::new(),
        default_triggers: vec![],
        workout_strategies: HashMap::new(),
        recovery_waterfall: RecoveryWaterfall { tiers: vec![] },
    };
    
    let mut engine = WorkoutEngine::new(policy);
    
    // Create a loan
    let loan = Loan::new(
        "TEST",
        Money::new(1_000_000.0, Currency::USD),
        Date::from_calendar_date(2025, Month::January, 1).unwrap(),
        Date::from_calendar_date(2030, Month::January, 1).unwrap(),
        InterestSpec::Fixed { rate: 0.06, step_ups: None },
    );
    
    // Move to default state
    engine.state = WorkoutState::Default {
        default_date: Date::from_calendar_date(2025, Month::June, 1).unwrap(),
        reason: "Payment default".to_string(),
    };
    
    let as_of = Date::from_calendar_date(2025, Month::July, 1).unwrap();
    let penalty_flows = engine.generate_penalty_flows(&loan, as_of).unwrap();
    
    // Should have default interest penalty
    assert_eq!(penalty_flows.len(), 1);
    assert_eq!(penalty_flows[0].kind, finstack_valuations::cashflow::primitives::CFKind::Fee);
    assert!(penalty_flows[0].amount.amount() > 0.0);
}
