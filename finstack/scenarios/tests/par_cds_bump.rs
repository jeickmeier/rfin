use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
use finstack_scenarios::{
    CurveKind, ExecutionContext, OperationSpec, ScenarioEngine, ScenarioSpec, TenorMatchMode,
};
use finstack_statements::FinancialModelSpec;
use time::Month;

#[test]
fn test_par_cds_bump_integration() {
    // Setup market with hazard curve
    let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
    let hazard = HazardCurve::builder("USD-CDS")
        .base_date(base_date)
        .recovery_rate(0.4)
        .knots(vec![(1.0, 0.01), (5.0, 0.02)])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert_hazard(hazard);
    let mut model = FinancialModelSpec::new("test", vec![]);

    // Apply 10bp Par CDS bump at 5Y
    let scenario = ScenarioSpec {
        id: "par_cds_bump".into(),
        name: Some("Par CDS Bump".into()),
        description: None,
        operations: vec![OperationSpec::CurveNodeBp {
            curve_kind: CurveKind::ParCDS,
            curve_id: "USD-CDS".into(),
            nodes: vec![("5Y".to_string(), 10.0)],
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
        calendar: None,
        as_of: base_date,
    };

    engine
        .apply(&scenario, &mut ctx)
        .expect("Shock application should succeed");

    // Verify result
    let bumped = market.get_hazard("USD-CDS").unwrap();

    // Check lambda at 5.0
    let points: Vec<_> = bumped.knot_points().collect();
    let (_, l_5y) = points
        .iter()
        .find(|(t, _)| (*t - 5.0).abs() < 1e-6)
        .unwrap();

    // Delta Lambda = 10bp / 10000 / (1 - 0.4) = 0.001 / 0.6 = 0.001666...
    let expected_delta = 0.001 / 0.6;
    let expected_lambda = 0.02 + expected_delta;

    println!(
        "Original: 0.02, Bumped: {}, Expected: {}",
        l_5y, expected_lambda
    );
    assert!(
        (l_5y - expected_lambda).abs() < 1e-6,
        "Expected lambda {}, got {}",
        expected_lambda,
        l_5y
    );
}
