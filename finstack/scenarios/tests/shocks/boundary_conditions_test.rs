//! Tests for boundary conditions and edge values.

use finstack_core::currency::Currency;
use finstack_core::dates::{build_periods, Date};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_scenarios::{
    CurveKind, ExecutionContext, OperationSpec, ScenarioEngine, ScenarioSpec,
};
use finstack_statements::{AmountOrScalar, FinancialModelSpec, NodeSpec, NodeType};
use indexmap::IndexMap;
use time::Month;

#[test]
fn test_zero_percent_shock() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mut market = MarketContext::new()
        .insert_price("SPY", MarketScalar::Price(Money::new(400.0, Currency::USD)));
    let mut model = FinancialModelSpec::new("test", vec![]);

    let scenario = ScenarioSpec {
        id: "zero_shock".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::EquityPricePct {
            ids: vec!["SPY".into()],
            pct: 0.0, // No change
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 1);

    let price = market.get_price("SPY").unwrap();
    match price {
        MarketScalar::Price(money) => {
            assert!((money.amount() - 400.0).abs() < 1e-6);
        }
        _ => panic!("Expected Price"),
    }
}

#[test]
fn test_negative_shock() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mut market = MarketContext::new()
        .insert_price("SPY", MarketScalar::Price(Money::new(100.0, Currency::USD)));
    let mut model = FinancialModelSpec::new("test", vec![]);

    let scenario = ScenarioSpec {
        id: "negative_shock".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::EquityPricePct {
            ids: vec!["SPY".into()],
            pct: -50.0, // Large negative shock
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 1);

    let price = market.get_price("SPY").unwrap();
    match price {
        MarketScalar::Price(money) => {
            assert!((money.amount() - 50.0).abs() < 1e-6);
        }
        _ => panic!("Expected Price"),
    }
}

#[test]
fn test_very_large_shock() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mut market = MarketContext::new()
        .insert_price("SPY", MarketScalar::Price(Money::new(100.0, Currency::USD)));
    let mut model = FinancialModelSpec::new("test", vec![]);

    let scenario = ScenarioSpec {
        id: "large_shock".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::EquityPricePct {
            ids: vec!["SPY".into()],
            pct: 500.0, // 5x increase
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 1);

    let price = market.get_price("SPY").unwrap();
    match price {
        MarketScalar::Price(money) => {
            assert!((money.amount() - 600.0).abs() < 1e-6);
        }
        _ => panic!("Expected Price"),
    }
}

#[test]
fn test_negative_100_percent_shock() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mut market = MarketContext::new()
        .insert_price("SPY", MarketScalar::Price(Money::new(100.0, Currency::USD)));
    let mut model = FinancialModelSpec::new("test", vec![]);

    let scenario = ScenarioSpec {
        id: "full_loss_shock".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::EquityPricePct {
            ids: vec!["SPY".into()],
            pct: -100.0, // 100% loss
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 1);

    // Price should be zero after -100% shock
    let price = market.get_price("SPY").unwrap();
    match price {
        MarketScalar::Price(money) => {
            assert!(
                money.amount().abs() < 1e-10,
                "Expected price to be 0 after -100% shock, got {}",
                money.amount()
            );
        }
        _ => panic!("Expected Price"),
    }
}

#[test]
fn test_shock_beyond_negative_100_percent() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mut market = MarketContext::new()
        .insert_price("SPY", MarketScalar::Price(Money::new(100.0, Currency::USD)));
    let mut model = FinancialModelSpec::new("test", vec![]);

    // -150% shock would result in negative price
    let scenario = ScenarioSpec {
        id: "beyond_full_loss".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::EquityPricePct {
            ids: vec!["SPY".into()],
            pct: -150.0,
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 1);

    // Document current behavior: price becomes negative
    // This may need validation in a future enhancement
    let price = market.get_price("SPY").unwrap();
    match price {
        MarketScalar::Price(money) => {
            // 100 * (1 - 1.5) = -50
            assert!(
                (money.amount() - (-50.0)).abs() < 1e-6,
                "Expected price -50 after -150% shock, got {}",
                money.amount()
            );
        }
        _ => panic!("Expected Price"),
    }
}

#[test]
fn test_shock_nonexistent_market_data() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mut market = MarketContext::new();
    let mut model = FinancialModelSpec::new("test", vec![]);

    let scenario = ScenarioSpec {
        id: "missing_data".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::EquityPricePct {
            ids: vec!["NONEXISTENT".into()],
            pct: -10.0,
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 0);
    assert!(!report.warnings.is_empty());
}

#[test]
fn test_shock_nonexistent_statement_node() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mut market = MarketContext::new();
    let mut model = FinancialModelSpec::new("test", vec![]);

    let scenario = ScenarioSpec {
        id: "missing_node".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::StmtForecastPercent {
            node_id: "NONEXISTENT".into(),
            pct: 10.0,
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: base_date,
    };

    let result = engine.apply(&scenario, &mut ctx);
    assert!(result.is_err(), "Should error on missing node");
}

#[test]
fn test_empty_tenor_string() {
    use finstack_scenarios::utils::parse_tenor_to_years;

    let result = parse_tenor_to_years("");
    assert!(result.is_err());
}

#[test]
fn test_empty_period_string() {
    use finstack_scenarios::utils::parse_period_to_days;

    let result = parse_period_to_days("");
    assert!(result.is_err());
}

#[test]
fn test_malformed_tenor_no_unit() {
    use finstack_scenarios::utils::parse_tenor_to_years;

    let result = parse_tenor_to_years("123");
    assert!(result.is_err());
}

#[test]
fn test_malformed_tenor_bad_unit() {
    use finstack_scenarios::utils::parse_tenor_to_years;

    let result = parse_tenor_to_years("5X");
    assert!(result.is_err());
}

#[test]
fn test_malformed_tenor_bad_number() {
    use finstack_scenarios::utils::parse_tenor_to_years;

    let result = parse_tenor_to_years("ABCY");
    assert!(result.is_err());
}

#[test]
fn test_malformed_period_no_unit() {
    use finstack_scenarios::utils::parse_period_to_days;

    let result = parse_period_to_days("30");
    assert!(result.is_err());
}

#[test]
fn test_malformed_period_bad_unit() {
    use finstack_scenarios::utils::parse_period_to_days;

    let result = parse_period_to_days("1Z");
    assert!(result.is_err());
}

#[test]
fn test_curve_parallel_shock_zero_bp() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let curve = DiscountCurve::builder("USD_SOFR")
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (1.0, 0.98)])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert(curve);
    let mut model = FinancialModelSpec::new("test", vec![]);

    let scenario = ScenarioSpec {
        id: "zero_bp".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::CurveParallelBp {
            curve_kind: CurveKind::Discount,
            curve_id: "USD_SOFR".into(),
            bp: 0.0,
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 1);
}

#[test]
fn test_statement_shock_negative_percent() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mut market = MarketContext::new();

    let period_plan = build_periods("2025Q1..Q2", None).unwrap();
    let periods = period_plan.periods;
    let mut model = FinancialModelSpec::new("test", periods.clone());

    let mut values = IndexMap::new();
    for period in &periods {
        values.insert(period.id, AmountOrScalar::Scalar(100.0));
    }

    model.add_node(NodeSpec::new("Revenue", NodeType::Value).with_values(values));

    let scenario = ScenarioSpec {
        id: "negative_stmt".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::StmtForecastPercent {
            node_id: "Revenue".into(),
            pct: -30.0,
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 1);

    let revenue = model.get_node("Revenue").unwrap();
    let val = revenue.values.as_ref().unwrap().values().next().unwrap();
    match val {
        AmountOrScalar::Scalar(s) => {
            assert!((s - 70.0).abs() < 1e-6);
        }
        _ => panic!("Expected scalar"),
    }
}

#[test]
fn test_curve_shock_nonexistent_curve() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mut market = MarketContext::new();
    let mut model = FinancialModelSpec::new("test", vec![]);

    let scenario = ScenarioSpec {
        id: "missing_curve".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::CurveParallelBp {
            curve_kind: CurveKind::Discount,
            curve_id: "NONEXISTENT_CURVE".into(),
            bp: 50.0,
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: base_date,
    };

    let result = engine.apply(&scenario, &mut ctx);
    assert!(result.is_err(), "Should error on missing curve");
}

#[test]
fn test_statement_assign_extreme_value() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mut market = MarketContext::new();

    let period_plan = build_periods("2025Q1..Q2", None).unwrap();
    let periods = period_plan.periods;
    let mut model = FinancialModelSpec::new("test", periods.clone());

    let mut values = IndexMap::new();
    for period in &periods {
        values.insert(period.id, AmountOrScalar::Scalar(100.0));
    }

    model.add_node(NodeSpec::new("TestNode", NodeType::Value).with_values(values));

    let scenario = ScenarioSpec {
        id: "extreme_assign".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::StmtForecastAssign {
            node_id: "TestNode".into(),
            value: 1_000_000_000.0, // Very large
        }],
        priority: 0,
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 1);

    let node = model.get_node("TestNode").unwrap();
    let val = node.values.as_ref().unwrap().values().next().unwrap();
    match val {
        AmountOrScalar::Scalar(s) => {
            assert!((s - 1_000_000_000.0).abs() < 1e-6);
        }
        _ => panic!("Expected scalar"),
    }
}
