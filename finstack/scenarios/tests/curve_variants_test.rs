//! Tests for all curve type variants (Forecast, Hazard, Inflation).

use finstack_core::dates::Date;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
use finstack_core::market_data::term_structures::inflation::InflationCurve;
use finstack_core::market_data::MarketContext;
use finstack_scenarios::{
    CurveKind, ExecutionContext, OperationSpec, ScenarioEngine, ScenarioSpec, TenorMatchMode,
};
use finstack_statements::FinancialModelSpec;
use time::Month;

#[test]
fn test_forecast_curve_parallel_shock() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    
    // Create forward curve (3M = 0.25 years)
    let curve = ForwardCurve::builder("USD_LIBOR_3M", 0.25)
        .base_date(base_date)
        .knots(vec![
            (0.0, 0.05),
            (1.0, 0.055),
            (5.0, 0.06),
        ])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert_forward(curve);
    let mut model = FinancialModelSpec::new("test", vec![]);

    let scenario = ScenarioSpec {
        id: "forward_shock".into(),
        name: Some("Forward Curve Shock".into()),
        description: None,
        operations: vec![OperationSpec::CurveParallelBp {
            curve_kind: CurveKind::Forecast,
            curve_id: "USD_LIBOR_3M".into(),
            bp: 25.0, // +25bp
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

    // Verify curve was bumped
    let bumped_curve = market.get_forward_ref("USD_LIBOR_3M").unwrap();
    let forwards = bumped_curve.forwards();
    assert!(forwards[0] > 0.05, "Forward rate should be bumped");
}

#[test]
fn test_hazard_curve_parallel_shock() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    
    // Create hazard curve
    let curve = HazardCurve::builder("CORP_BBB")
        .base_date(base_date)
        .recovery_rate(0.4)
        .knots(vec![
            (0.0, 0.0),
            (1.0, 0.02),
            (5.0, 0.025),
        ])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert_hazard(curve);
    let mut model = FinancialModelSpec::new("test", vec![]);

    let scenario = ScenarioSpec {
        id: "hazard_shock".into(),
        name: Some("Hazard Curve Shock".into()),
        description: None,
        operations: vec![OperationSpec::CurveParallelBp {
            curve_kind: CurveKind::Hazard,
            curve_id: "CORP_BBB".into(),
            bp: 50.0, // +50bp credit spread widening
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

    // Verify curve exists with original ID
    let bumped_curve = market.get_hazard_ref("CORP_BBB").unwrap();
    assert_eq!(bumped_curve.id().as_str(), "CORP_BBB");
}

#[test]
fn test_inflation_curve_parallel_shock() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    
    // Create inflation curve
    let curve = InflationCurve::builder("US_CPI")
        .base_cpi(300.0)
        .knots(vec![
            (0.0, 300.0),
            (1.0, 306.0), // ~2% inflation
            (5.0, 330.0),
        ])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert_inflation(curve);
    let mut model = FinancialModelSpec::new("test", vec![]);

    let scenario = ScenarioSpec {
        id: "inflation_shock".into(),
        name: Some("Inflation Curve Shock".into()),
        description: None,
        operations: vec![OperationSpec::CurveParallelBp {
            curve_kind: CurveKind::Inflation,
            curve_id: "US_CPI".into(),
            bp: 100.0, // +100bp = +1% inflation
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

    // Verify curve was bumped
    let bumped_curve = market.get_inflation_ref("US_CPI").unwrap();
    let cpi_levels = bumped_curve.cpi_levels();
    assert!(cpi_levels[1] > 306.0, "CPI level should be bumped");
}

#[test]
fn test_forecast_curve_node_shock() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    
    let curve = ForwardCurve::builder("USD_LIBOR_3M", 0.25)
        .base_date(base_date)
        .knots(vec![
            (0.0, 0.05),
            (1.0, 0.055),
            (5.0, 0.06),
        ])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert_forward(curve);
    let mut model = FinancialModelSpec::new("test", vec![]);

    let scenario = ScenarioSpec {
        id: "forward_node".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::CurveNodeBp {
            curve_kind: CurveKind::Forecast,
            curve_id: "USD_LIBOR_3M".into(),
            nodes: vec![("1Y".into(), 50.0)],
            match_mode: TenorMatchMode::Exact,
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
}

#[test]
fn test_hazard_curve_node_shock() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    
    let curve = HazardCurve::builder("CORP_BBB")
        .base_date(base_date)
        .recovery_rate(0.4)
        .knots(vec![
            (0.0, 0.0),
            (1.0, 0.02),
            (5.0, 0.025),
        ])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert_hazard(curve);
    let mut model = FinancialModelSpec::new("test", vec![]);

    let scenario = ScenarioSpec {
        id: "hazard_node".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::CurveNodeBp {
            curve_kind: CurveKind::Hazard,
            curve_id: "CORP_BBB".into(),
            nodes: vec![("5Y".into(), 25.0)],
            match_mode: TenorMatchMode::Interpolate,
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
}

#[test]
fn test_inflation_curve_node_shock() {
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    
    let curve = InflationCurve::builder("US_CPI")
        .base_cpi(300.0)
        .knots(vec![
            (0.0, 300.0),
            (1.0, 306.0),
            (5.0, 330.0),
        ])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert_inflation(curve);
    let mut model = FinancialModelSpec::new("test", vec![]);

    let scenario = ScenarioSpec {
        id: "inflation_node".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::CurveNodeBp {
            curve_kind: CurveKind::Inflation,
            curve_id: "US_CPI".into(),
            nodes: vec![("1Y".into(), 50.0)],
            match_mode: TenorMatchMode::Exact,
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
}

#[test]
fn test_discount_curve_id_preservation() {
    // Regression test: ensure curve ID is preserved after shock
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let curve = DiscountCurve::builder("USD_SOFR")
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (1.0, 0.98), (5.0, 0.90)])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert_discount(curve);
    let mut model = FinancialModelSpec::new("test", vec![]);

    let scenario = ScenarioSpec {
        id: "test".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::CurveParallelBp {
            curve_kind: CurveKind::Discount,
            curve_id: "USD_SOFR".into(),
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
        as_of: base_date,
    };

    engine.apply(&scenario, &mut ctx).unwrap();

    // Verify original ID is preserved
    let bumped = market.get_discount_ref("USD_SOFR").unwrap();
    assert_eq!(bumped.id().as_str(), "USD_SOFR");
}

#[test]
fn test_all_curve_types_in_one_scenario() {
    // Test applying shocks to multiple curve types simultaneously
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    
    let disc = DiscountCurve::builder("USD_SOFR")
        .base_date(base_date)
        .knots(vec![(0.0, 1.0), (1.0, 0.98)])
        .build()
        .unwrap();
    
    let fwd = ForwardCurve::builder("USD_LIBOR_3M", 0.25)
        .base_date(base_date)
        .knots(vec![(0.0, 0.05), (1.0, 0.055)])
        .build()
        .unwrap();
    
    let hazard = HazardCurve::builder("CORP_BBB")
        .base_date(base_date)
        .recovery_rate(0.4)
        .knots(vec![(0.0, 0.0), (1.0, 0.02)])
        .build()
        .unwrap();
    
    let inflation = InflationCurve::builder("US_CPI")
        .base_cpi(300.0)
        .knots(vec![(0.0, 300.0), (1.0, 306.0)])
        .build()
        .unwrap();

    let mut market = MarketContext::new()
        .insert_discount(disc)
        .insert_forward(fwd)
        .insert_hazard(hazard)
        .insert_inflation(inflation);
    
    let mut model = FinancialModelSpec::new("test", vec![]);

    let scenario = ScenarioSpec {
        id: "all_curves".into(),
        name: None,
        description: None,
        operations: vec![
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Discount,
                curve_id: "USD_SOFR".into(),
                bp: 25.0,
            },
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Forecast,
                curve_id: "USD_LIBOR_3M".into(),
                bp: 30.0,
            },
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Hazard,
                curve_id: "CORP_BBB".into(),
                bp: 50.0,
            },
            OperationSpec::CurveParallelBp {
                curve_kind: CurveKind::Inflation,
                curve_id: "US_CPI".into(),
                bp: 100.0,
            },
        ],
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
    assert_eq!(report.operations_applied, 4);

    // Verify all curves still accessible with original IDs
    assert!(market.get_discount_ref("USD_SOFR").is_ok());
    assert!(market.get_forward_ref("USD_LIBOR_3M").is_ok());
    assert!(market.get_hazard_ref("CORP_BBB").is_ok());
    assert!(market.get_inflation_ref("US_CPI").is_ok());
}

