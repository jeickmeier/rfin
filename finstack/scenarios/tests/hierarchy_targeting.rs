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

/// Hierarchy-targeted `HierarchyCurveParallelBp` resolves to one bump per matched curve.
#[test]
fn hierarchy_curve_parallel_bp_resolves_to_individual_bumps() {
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::hierarchy::HierarchyTarget;
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

    // Use hierarchy-targeted operation targeting the "Rates/USD" subtree,
    // which resolves to both USD-OIS and USD-TSY.
    let scenario = ScenarioSpec {
        id: "hierarchy_bump".into(),
        name: None,
        description: None,
        operations: vec![OperationSpec::HierarchyCurveParallelBp {
            curve_kind: CurveKind::Discount,
            target: HierarchyTarget {
                path: vec!["Rates".into(), "USD".into()],
                tag_filter: None,
            },
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
    // Should have applied 2 bumps (one per resolved curve: USD-OIS, USD-TSY)
    assert_eq!(
        report.operations_applied, 2,
        "Expected 2 operations applied (one per resolved curve), got {}. Warnings: {:?}",
        report.operations_applied, report.warnings
    );
}

/// Cumulative mode stacks all matching shocks from every hierarchy level.
///
/// Two operations target overlapping subtrees:
///   - +50bp on all of "Credit" (matches JPM-5Y and OXY-5Y)
///   - +100bp on "Credit/US/HY" (matches OXY-5Y only)
///
/// With Cumulative mode the engine expands both operations independently, so
/// OXY-5Y receives two bumps and JPM-5Y receives one. Total: 3 operations applied.
#[test]
fn cumulative_mode_stacks_shocks_down_tree() {
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::hierarchy::HierarchyTarget;
    use finstack_core::market_data::hierarchy::MarketDataHierarchy;
    use finstack_core::market_data::hierarchy::ResolutionMode;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_scenarios::{CurveKind, ExecutionContext, ScenarioEngine};
    use finstack_statements::FinancialModelSpec;
    use time::macros::date;

    let h = MarketDataHierarchy::builder()
        .add_node("Credit/US/IG/Financials")
        .curve_ids(&["JPM-5Y"])
        .add_node("Credit/US/HY/Energy")
        .curve_ids(&["OXY-5Y"])
        .build()
        .unwrap();

    let base = date!(2025 - 01 - 01);
    let jpm = DiscountCurve::builder("JPM-5Y")
        .base_date(base)
        .knots([(0.0, 1.0), (5.0, 0.90)])
        .build()
        .unwrap();
    let oxy = DiscountCurve::builder("OXY-5Y")
        .base_date(base)
        .knots([(0.0, 1.0), (5.0, 0.85)])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert(jpm).insert(oxy);
    market.set_hierarchy(h);

    let mut model = FinancialModelSpec::new("test", vec![]);

    // Op1: +50bp on all Credit (matches JPM-5Y and OXY-5Y → 2 expansions)
    // Op2: +100bp on Credit/US/HY (matches OXY-5Y only → 1 expansion)
    // Cumulative: all 3 expansions applied → operations_applied == 3
    let scenario = ScenarioSpec {
        id: "cumulative_test".into(),
        name: None,
        description: None,
        operations: vec![
            OperationSpec::HierarchyCurveParallelBp {
                curve_kind: CurveKind::Discount,
                target: HierarchyTarget {
                    path: vec!["Credit".into()],
                    tag_filter: None,
                },
                bp: 50.0,
            },
            OperationSpec::HierarchyCurveParallelBp {
                curve_kind: CurveKind::Discount,
                target: HierarchyTarget {
                    path: vec!["Credit".into(), "US".into(), "HY".into()],
                    tag_filter: None,
                },
                bp: 100.0,
            },
        ],
        priority: 0,
        resolution_mode: ResolutionMode::Cumulative,
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
    // Credit-wide op: JPM-5Y + OXY-5Y = 2 bumps
    // HY-specific op: OXY-5Y = 1 bump
    // Cumulative total: 3 operations applied
    assert_eq!(
        report.operations_applied, 3,
        "Expected 3 operations (2 from Credit-wide + 1 from HY-specific), got {}. Warnings: {:?}",
        report.operations_applied, report.warnings
    );
}

/// MostSpecificWins keeps only the deepest matching shock per curve.
///
/// Two operations target overlapping subtrees:
///   - +50bp on all of "Credit" (path depth 1, matches JPM-5Y and OXY-5Y)
///   - +100bp on "Credit/US/HY" (path depth 3, matches OXY-5Y only)
///
/// With MostSpecificWins the engine deduplicates by curve ID keeping the deepest
/// match. OXY-5Y is matched by depth-3 and depth-1; depth-3 wins. JPM-5Y is
/// matched only by depth-1. Total: 2 operations applied (one per curve).
#[test]
fn most_specific_wins_keeps_only_deepest_shock() {
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::hierarchy::HierarchyTarget;
    use finstack_core::market_data::hierarchy::MarketDataHierarchy;
    use finstack_core::market_data::hierarchy::ResolutionMode;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_scenarios::{CurveKind, ExecutionContext, ScenarioEngine};
    use finstack_statements::FinancialModelSpec;
    use time::macros::date;

    let h = MarketDataHierarchy::builder()
        .add_node("Credit/US/IG/Financials")
        .curve_ids(&["JPM-5Y"])
        .add_node("Credit/US/HY/Energy")
        .curve_ids(&["OXY-5Y"])
        .build()
        .unwrap();

    let base = date!(2025 - 01 - 01);
    let jpm = DiscountCurve::builder("JPM-5Y")
        .base_date(base)
        .knots([(0.0, 1.0), (5.0, 0.90)])
        .build()
        .unwrap();
    let oxy = DiscountCurve::builder("OXY-5Y")
        .base_date(base)
        .knots([(0.0, 1.0), (5.0, 0.85)])
        .build()
        .unwrap();

    let mut market = MarketContext::new().insert(jpm).insert(oxy);
    market.set_hierarchy(h);

    let mut model = FinancialModelSpec::new("test", vec![]);

    // Op1 path depth 1 (Credit):     JPM-5Y → depth 1, OXY-5Y → depth 1
    // Op2 path depth 3 (Credit/US/HY): OXY-5Y → depth 3
    // MostSpecificWins: max depth per curve_id:
    //   JPM-5Y → 1, OXY-5Y → 3
    // Kept: JPM-5Y from Op1 (depth 1 == max 1), OXY-5Y from Op2 (depth 3 == max 3)
    // Dropped: OXY-5Y from Op1 (depth 1 < max 3)
    // operations_applied == 2
    let scenario = ScenarioSpec {
        id: "msw_test".into(),
        name: None,
        description: None,
        operations: vec![
            OperationSpec::HierarchyCurveParallelBp {
                curve_kind: CurveKind::Discount,
                target: HierarchyTarget {
                    path: vec!["Credit".into()],
                    tag_filter: None,
                },
                bp: 50.0,
            },
            OperationSpec::HierarchyCurveParallelBp {
                curve_kind: CurveKind::Discount,
                target: HierarchyTarget {
                    path: vec!["Credit".into(), "US".into(), "HY".into()],
                    tag_filter: None,
                },
                bp: 100.0,
            },
        ],
        priority: 0,
        resolution_mode: ResolutionMode::MostSpecificWins,
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
    // JPM-5Y: only matched at depth 1 → +50bp applied
    // OXY-5Y: depth-3 match wins over depth-1 match → +100bp applied
    // Total: 2 operations (one per curve, deepest match only)
    assert_eq!(
        report.operations_applied, 2,
        "Expected 2 operations (one per curve, deepest match wins), got {}. Warnings: {:?}",
        report.operations_applied, report.warnings
    );
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
