//! Tests for time roll-forward with carry/theta.

use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_scenarios::{ExecutionContext, OperationSpec, ScenarioEngine, ScenarioSpec};
use finstack_statements::FinancialModelSpec;
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
