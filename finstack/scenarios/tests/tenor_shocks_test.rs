//! Tests for tenor-based curve node shocks.

use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::MarketContext;
use finstack_scenarios::{
    CurveKind, ExecutionContext, OperationSpec, ScenarioEngine, ScenarioSpec, TenorMatchMode,
};
use finstack_statements::FinancialModelSpec;
use time::Month;

#[test]
fn test_tenor_exact_match() {
    // Setup market with discount curve
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![
            (0.0, 1.0),
            (1.0, 0.98),  // 1Y pillar
            (5.0, 0.90),  // 5Y pillar
            (10.0, 0.80), // 10Y pillar
        ])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert_discount(curve);
    let mut model = FinancialModelSpec::new("test", vec![]);

    // Create scenario with exact tenor matching at 5Y
    let scenario = ScenarioSpec {
        id: "tenor_exact".into(),
        name: Some("Tenor Exact Match".into()),
        description: None,
        operations: vec![OperationSpec::CurveNodeBp {
            curve_kind: CurveKind::Discount,
            curve_id: "USD-OIS".into(),
            nodes: vec![("5Y".into(), 25.0)], // +25bp at 5Y
            match_mode: TenorMatchMode::Exact,
        }],
        priority: 0,
    };

    // Apply scenario
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
fn test_tenor_exact_not_found() {
    // Setup market with discount curve
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (1.0, 0.98), (5.0, 0.90)])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert_discount(curve);
    let mut model = FinancialModelSpec::new("test", vec![]);

    // Try to shock at 3Y which doesn't exist
    let scenario = ScenarioSpec {
        id: "tenor_not_found".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::CurveNodeBp {
            curve_kind: CurveKind::Discount,
            curve_id: "USD-OIS".into(),
            nodes: vec![("3Y".into(), 25.0)], // 3Y doesn't exist
            match_mode: TenorMatchMode::Exact,
        }],
        priority: 0,
    };

    // Apply scenario - should fail
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
    assert!(result.is_err(), "Expected error for non-existent tenor");
}

#[test]
fn test_tenor_interpolate_mode() {
    // Setup market with discount curve
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let curve = DiscountCurve::builder("USD-OIS")
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (1.0, 0.98), (5.0, 0.90), (10.0, 0.80)])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert_discount(curve);
    let mut model = FinancialModelSpec::new("test", vec![]);

    // Shock at 3Y using interpolation (between 1Y and 5Y)
    let scenario = ScenarioSpec {
        id: "tenor_interpolate".into(),
        name: Some("Tenor Interpolate".into()),
        description: None,
        operations: vec![OperationSpec::CurveNodeBp {
            curve_kind: CurveKind::Discount,
            curve_id: "USD-OIS".into(),
            nodes: vec![("3Y".into(), 50.0)], // +50bp at 3Y (interpolated)
            match_mode: TenorMatchMode::Interpolate,
        }],
        priority: 0,
    };

    // Apply scenario
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
