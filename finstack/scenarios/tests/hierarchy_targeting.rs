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
