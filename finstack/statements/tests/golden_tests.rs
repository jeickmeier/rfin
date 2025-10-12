//! Golden tests for serialization stability and end-to-end correctness.
//!
//! These tests verify that the wire format doesn't change and that
//! evaluation produces consistent results over time.

use finstack_core::dates::PeriodId;
use finstack_statements::evaluator::Evaluator;
use finstack_statements::types::FinancialModelSpec;
use indexmap::IndexMap;

#[test]
fn test_golden_basic_model() {
    // Load golden model JSON
    let json = include_str!("golden/basic_model.json");
    let spec: FinancialModelSpec =
        serde_json::from_str(json).expect("Failed to deserialize basic_model.json");

    // Verify model structure
    assert_eq!(spec.id, "basic_pl_golden");
    assert_eq!(spec.periods.len(), 2);
    assert_eq!(spec.nodes.len(), 4);

    // Evaluate the model
    let mut evaluator = Evaluator::new();
    let results = evaluator
        .evaluate(&spec)
        .expect("Failed to evaluate basic golden model");

    // Load expected results
    let expected_json = include_str!("golden/basic_model_results.json");
    let expected: IndexMap<String, IndexMap<PeriodId, f64>> = serde_json::from_str(expected_json)
        .expect("Failed to deserialize basic_model_results.json");

    // Compare results
    for (node_id, expected_periods) in &expected {
        for (period_id, expected_value) in expected_periods {
            let actual_value = results.get(node_id, period_id).unwrap_or_else(|| {
                panic!(
                    "Missing value for node '{}' at period '{}'",
                    node_id, period_id
                )
            });

            assert!(
                (actual_value - expected_value).abs() < 0.01,
                "Value mismatch for node '{}' at period '{}': expected {}, got {}",
                node_id,
                period_id,
                expected_value,
                actual_value
            );
        }
    }
}

#[test]
fn test_golden_model_serialization_stability() {
    // Load the golden model
    let json = include_str!("golden/basic_model.json");
    let spec: FinancialModelSpec =
        serde_json::from_str(json).expect("Failed to deserialize basic_model.json");

    // Serialize it back to JSON
    let reserialized = serde_json::to_string_pretty(&spec).expect("Failed to serialize model");

    // Deserialize again
    let spec2: FinancialModelSpec =
        serde_json::from_str(&reserialized).expect("Failed to deserialize reserialized model");

    // Should be identical
    assert_eq!(spec.id, spec2.id);
    assert_eq!(spec.periods.len(), spec2.periods.len());
    assert_eq!(spec.nodes.len(), spec2.nodes.len());
    assert_eq!(spec.schema_version, spec2.schema_version);

    // Verify node structure is preserved
    for (node_id, node_spec) in &spec.nodes {
        let node_spec2 = spec2.nodes.get(node_id).unwrap_or_else(|| {
            panic!(
                "Node '{}' missing after serialization roundtrip",
                node_id
            )
        });
        assert_eq!(node_spec.node_type, node_spec2.node_type);
        assert_eq!(node_spec.formula_text, node_spec2.formula_text);
    }
}
