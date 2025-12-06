use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
use finstack_scenarios::adapters::curves::apply_curve_node_shock;
use finstack_scenarios::{CurveKind, TenorMatchMode};
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

    // Apply 10bp Par CDS bump at 5Y
    // Delta Lambda = 10bp / 10000 / (1 - 0.4) = 0.001 / 0.6 = 0.001666...
    let nodes = vec![("5Y".to_string(), 10.0)];

    apply_curve_node_shock(
        &mut market,
        CurveKind::ParCDS,
        "USD-CDS",
        &nodes,
        TenorMatchMode::Exact,
    )
    .expect("Shock application should succeed");

    // Verify result
    let bumped = market.get_hazard("USD-CDS").unwrap();

    // Check lambda at 5.0
    let points: Vec<_> = bumped.knot_points().collect();
    let (_, l_5y) = points
        .iter()
        .find(|(t, _)| (*t - 5.0).abs() < 1e-6)
        .unwrap();

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
