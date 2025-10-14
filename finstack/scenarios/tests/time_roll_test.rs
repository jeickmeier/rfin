//! Tests for time roll-forward with carry/theta.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_scenarios::{ExecutionContext, OperationSpec, ScenarioEngine, ScenarioSpec};
use finstack_statements::FinancialModelSpec;
use finstack_valuations::instruments::common::traits::{Attributes, Instrument};
use finstack_valuations::instruments::pricing_overrides::PricingOverrides;
use finstack_valuations::instruments::Bond;
use time::Month;

#[test]
fn test_time_roll_1_day() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mut market = MarketContext::new();
    let mut model = FinancialModelSpec::new("test", vec![]);

    let scenario = ScenarioSpec {
        id: "roll_1d".into(),
        name: Some("Roll 1 Day".into()),
        description: None,
        operations: vec![OperationSpec::TimeRollForward {
            period: "1D".into(),
            apply_shocks: false,
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        as_of: base_date,
    };

    let original_date = ctx.as_of;
    let report = engine.apply(&scenario, &mut ctx).unwrap();

    assert_eq!(report.operations_applied, 1);

    // Verify date advanced by 1 day
    let expected_date = base_date + time::Duration::days(1);
    assert_eq!(ctx.as_of, expected_date);
    assert_ne!(ctx.as_of, original_date);
}

#[test]
fn test_time_roll_1_month() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mut market = MarketContext::new();
    let mut model = FinancialModelSpec::new("test", vec![]);

    let scenario = ScenarioSpec {
        id: "roll_1m".into(),
        name: Some("Roll 1 Month".into()),
        description: None,
        operations: vec![OperationSpec::TimeRollForward {
            period: "1M".into(),
            apply_shocks: false,
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 1);

    // Verify date advanced by ~30 days
    let expected_date = base_date + time::Duration::days(30);
    assert_eq!(ctx.as_of, expected_date);
}

#[test]
fn test_time_roll_1_year() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mut market = MarketContext::new();
    let mut model = FinancialModelSpec::new("test", vec![]);

    let scenario = ScenarioSpec {
        id: "roll_1y".into(),
        name: Some("Roll 1 Year".into()),
        description: None,
        operations: vec![OperationSpec::TimeRollForward {
            period: "1Y".into(),
            apply_shocks: false,
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 1);

    // Verify date advanced by 365 days
    let expected_date = base_date + time::Duration::days(365);
    assert_eq!(ctx.as_of, expected_date);
}

#[test]
fn test_time_roll_with_bond_carry() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    
    // Setup discount curve
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (1.0, 0.98), (5.0, 0.90)])
        .build()
        .unwrap();
    
    let mut market = MarketContext::new().insert_discount(curve);
    let mut model = FinancialModelSpec::new("test", vec![]);

    // Create a bond instrument
    let mut instruments: Vec<Box<dyn Instrument>> = vec![
        Box::new(
            Bond::builder()
                .id("BOND1".into())
                .notional(finstack_core::money::Money::new(100.0, Currency::USD))
                .coupon(0.05)
                .issue(base_date)
                .maturity(base_date + time::Duration::days(730))
                .freq(finstack_core::dates::Frequency::annual())
                .dc(finstack_core::dates::DayCount::Thirty360)
                .bdc(finstack_core::dates::BusinessDayConvention::Following)
                .calendar_id_opt(None)
                .stub(finstack_core::dates::StubKind::None)
                .disc_id(finstack_core::types::CurveId::new("USD-OIS"))
                .hazard_id_opt(None)
                .pricing_overrides(PricingOverrides::default())
                .attributes(Attributes::new())
                .build()
                .unwrap(),
        ),
    ];

    let scenario = ScenarioSpec {
        id: "roll_with_carry".into(),
        name: Some("Roll 1 Month with Carry".into()),
        description: None,
        operations: vec![OperationSpec::TimeRollForward {
            period: "1M".into(),
            apply_shocks: false,
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: Some(&mut instruments),
        rate_bindings: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 1);

    // Verify date rolled
    let expected_date = base_date + time::Duration::days(30);
    assert_eq!(ctx.as_of, expected_date);
}
