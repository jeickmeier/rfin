//! Tests for hierarchy-targeted scenario operations.

use finstack_scenarios::{OperationSpec, ScenarioSpec};

/// Existing direct-targeted JSON must still deserialize (backwards compatibility).
#[test]
fn existing_direct_target_json_round_trips() {
    let json = r#"{
        "id": "test",
        "operations": [
            {
                "kind": "curve_parallel_bp",
                "curve_kind": "discount",
                "curve_id": "USD-OIS",
                "bp": 50.0
            }
        ]
    }"#;
    let spec: ScenarioSpec = serde_json::from_str(json).unwrap();
    assert_eq!(spec.operations.len(), 1);
    match &spec.operations[0] {
        OperationSpec::CurveParallelBp { curve_id, bp, .. } => {
            assert_eq!(curve_id.as_str(), "USD-OIS");
            assert!((bp - 50.0).abs() < f64::EPSILON);
        }
        other => panic!("Expected CurveParallelBp, got: {:?}", other),
    }
}

/// JSON with explicit `resolution_mode` deserializes correctly.
#[test]
fn resolution_mode_field_deserializes_from_json() {
    let json = r#"{
        "id": "test",
        "operations": [],
        "resolution_mode": "cumulative"
    }"#;
    let spec: ScenarioSpec = serde_json::from_str(json).unwrap();
    use finstack_core::market_data::hierarchy::ResolutionMode;
    assert_eq!(spec.resolution_mode, ResolutionMode::Cumulative);
}

/// JSON without `resolution_mode` defaults to `MostSpecificWins`.
#[test]
fn resolution_mode_defaults_to_most_specific_wins() {
    let json = r#"{"id": "test", "operations": []}"#;
    let spec: ScenarioSpec = serde_json::from_str(json).unwrap();
    use finstack_core::market_data::hierarchy::ResolutionMode;
    assert_eq!(spec.resolution_mode, ResolutionMode::MostSpecificWins);
}

/// Engine smoke test: adding `resolution_mode` to `ScenarioSpec` does not break
/// the existing engine flow for a direct curve-targeted operation.
#[test]
fn engine_works_with_resolution_mode_field() {
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::hierarchy::MarketDataHierarchy;
    use finstack_core::market_data::hierarchy::ResolutionMode;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_scenarios::{CurveKind, ExecutionContext, ScenarioEngine};
    use finstack_statements::FinancialModelSpec;
    use time::macros::date;

    let h = MarketDataHierarchy::builder()
        .add_node("Rates/USD/OIS")
        .curve_ids(&["USD-OIS"])
        .add_node("Rates/USD/Treasury")
        .curve_ids(&["USD-TSY"])
        .build()
        .unwrap();

    let base = date!(2025 - 01 - 01);
    let ois = DiscountCurve::builder("USD-OIS")
        .base_date(base)
        .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.78)])
        .build()
        .unwrap();
    let tsy = DiscountCurve::builder("USD-TSY")
        .base_date(base)
        .knots([(0.0, 1.0), (1.0, 0.96), (5.0, 0.80)])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert(ois).insert(tsy);
    market.set_hierarchy(h);

    let mut model = FinancialModelSpec::new("test", vec![]);

    // Bump USD-OIS directly with resolution_mode explicitly set
    let scenario = ScenarioSpec {
        id: "test_hierarchy".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::CurveParallelBp {
            curve_kind: CurveKind::Discount,
            curve_id: "USD-OIS".into(),
            bp: 50.0,
        }],
        priority: 0,
        resolution_mode: ResolutionMode::default(),
    };

    let engine = ScenarioEngine::new();
    let mut ctx = ExecutionContext {
        market: &mut market,
        model: &mut model,
        instruments: None,
        rate_bindings: None,
        calendar: None,
        as_of: base,
    };

    let report = engine.apply(&scenario, &mut ctx).unwrap();
    assert!(
        report.operations_applied >= 1,
        "Expected at least 1 operation applied, got {}",
        report.operations_applied
    );
}
