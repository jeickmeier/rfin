//! Integration tests for scenarios crate.

use finstack_core::dates::Date;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;
use finstack_core::currency::Currency;
use finstack_scenarios::{
    CurveKind, ExecutionContext, OperationSpec, ScenarioEngine, ScenarioSpec,
};
use finstack_statements::FinancialModelSpec;
use time::Month;

#[test]
fn test_curve_parallel_shock() {
    // Setup market with discount curve
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (1.0, 0.98), (5.0, 0.90)])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert_discount(curve);

    // Setup empty model
    let mut model = FinancialModelSpec::new("test", vec![]);

    // Create scenario with parallel curve shock
    let scenario = ScenarioSpec {
        id: "rate_shock".into(),
        name: Some("Rate Shock".into()),
        description: None,
        operations: vec![OperationSpec::CurveParallelBp {
            curve_kind: CurveKind::Discount,
            curve_id: "USD-OIS".into(),
            bp: 50.0, // +50bp
        }],
        priority: 0,
    };

    // Apply scenario
    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        rate_bindings: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 1);

    // Verify the bumped curve exists
    assert!(market.get_discount("USD-OIS_bump_50bp").is_ok());
}

#[test]
fn test_equity_price_shock() {
    // Setup market with equity price
    let mut market = MarketContext::new()
        .insert_price("SPY", MarketScalar::Price(Money::new(450.0, Currency::USD)));

    // Setup empty model
    let mut model = FinancialModelSpec::new("test", vec![]);

    // Create scenario with equity shock
    let scenario = ScenarioSpec {
        id: "equity_shock".into(),
        name: Some("Equity Shock".into()),
        description: None,
        operations: vec![OperationSpec::EquityPricePct {
            ids: vec!["SPY".into()],
            pct: -10.0, // -10%
        }],
        priority: 0,
    };

    // Apply scenario
    let engine = ScenarioEngine::new();
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        rate_bindings: None,
        as_of: base_date,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert_eq!(report.operations_applied, 1);

    // Verify shocked price
    let shocked = market.price("SPY").unwrap();
    match shocked {
        MarketScalar::Price(money) => {
            let expected = 450.0 * 0.9; // -10%
            assert!((money.amount() - expected).abs() < 1e-6);
        }
        _ => panic!("Expected Price scalar"),
    }
}

#[test]
fn test_scenario_composition() {
    // Create two scenarios
    let s1 = ScenarioSpec {
        id: "base".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::CurveParallelBp {
            curve_kind: CurveKind::Discount,
            curve_id: "USD-OIS".into(),
            bp: 25.0,
        }],
        priority: 0,
    };

    let s2 = ScenarioSpec {
        id: "overlay".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::CurveParallelBp {
            curve_kind: CurveKind::Discount,
            curve_id: "EUR-OIS".into(),
            bp: 30.0,
        }],
        priority: 1,
    };

    // Compose scenarios
    let engine = ScenarioEngine::new();
    let composed = engine.compose(vec![s1, s2]);

    assert_eq!(composed.operations.len(), 2);
    assert_eq!(composed.id, "composed");
}

