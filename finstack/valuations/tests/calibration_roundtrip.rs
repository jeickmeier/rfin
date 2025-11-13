//! JSON round-trip tests for calibration specs and results.
//!
//! Verifies that calibration specifications can be:
//! 1. Loaded from JSON
//! 2. Executed to produce results
//! 3. Serialized back to JSON
//! 4. Deserialized again with identical structure

use finstack_valuations::calibration::CalibrationEnvelope;
use std::fs;

#[test]
fn test_simple_rates_only_roundtrip() {
    // Load example
    let json = fs::read_to_string(
        "tests/calibration/json_examples/simple_rates_only.json",
    )
    .expect("Failed to read example file");

    // Parse envelope
    let envelope = CalibrationEnvelope::from_str(&json)
        .expect("Failed to parse calibration envelope");

    assert_eq!(envelope.schema, "finstack.calibration/1");

    // Serialize back to JSON
    let reserialized = serde_json::to_string_pretty(&envelope)
        .expect("Failed to serialize envelope");

    // Deserialize again
    let reparsed: CalibrationEnvelope = serde_json::from_str(&reserialized)
        .expect("Failed to re-parse envelope");

    assert_eq!(reparsed.schema, envelope.schema);
}

#[test]
#[cfg(feature = "slow")]
fn test_simple_rates_calibration_execution() {
    // Load and execute a simple rates-only calibration
    let json = fs::read_to_string(
        "tests/calibration/json_examples/simple_rates_only.json",
    )
    .expect("Failed to read example file");

    let envelope = CalibrationEnvelope::from_str(&json)
        .expect("Failed to parse calibration envelope");

    // Execute calibration
    use finstack_valuations::calibration::CalibrationResultEnvelope;
    let result_envelope = envelope.execute(None)
        .expect("Calibration execution failed");

    assert_eq!(result_envelope.schema, "finstack.calibration/1");
    
    // Verify final market has at least one curve
    assert!(!result_envelope.result.final_market.curves.is_empty());

    // Serialize result to JSON
    let result_json = result_envelope.to_string()
        .expect("Failed to serialize result");

    // Deserialize result back
    let reparsed_result = CalibrationResultEnvelope::from_str(&result_json)
        .expect("Failed to reparse result");

    // Verify structural equality
    assert_eq!(reparsed_result.schema, result_envelope.schema);
    assert_eq!(
        reparsed_result.result.final_market.curves.len(),
        result_envelope.result.final_market.curves.len()
    );
}

#[test]
fn test_hazard_aapl_roundtrip() {
    // Load hazard example
    let json = fs::read_to_string(
        "tests/calibration/json_examples/hazard_aapl.json",
    )
    .expect("Failed to read example file");

    // Parse envelope
    let envelope = CalibrationEnvelope::from_str(&json)
        .expect("Failed to parse calibration envelope");

    // Serialize back
    let reserialized = serde_json::to_string_pretty(&envelope)
        .expect("Failed to serialize envelope");

    // Re-parse
    let _reparsed: CalibrationEnvelope = serde_json::from_str(&reserialized)
        .expect("Failed to re-parse envelope");
}

#[test]
fn test_vol_equity_roundtrip() {
    // Load vol surface example
    let json = fs::read_to_string(
        "tests/calibration/json_examples/vol_equity.json",
    )
    .expect("Failed to read example file");

    // Parse envelope
    let envelope = CalibrationEnvelope::from_str(&json)
        .expect("Failed to parse calibration envelope");

    // Serialize back
    let reserialized = serde_json::to_string_pretty(&envelope)
        .expect("Failed to serialize envelope");

    // Re-parse
    let _reparsed: CalibrationEnvelope = serde_json::from_str(&reserialized)
        .expect("Failed to re-parse envelope");
}

#[test]
fn test_full_market_pipeline_roundtrip() {
    // Load pipeline example
    let json = fs::read_to_string(
        "tests/calibration/json_examples/full_market_pipeline.json",
    )
    .expect("Failed to read example file");

    // Parse envelope
    let envelope = CalibrationEnvelope::from_str(&json)
        .expect("Failed to parse calibration envelope");

    // Verify it's a pipeline
    match envelope.calibration {
        finstack_valuations::calibration::CalibrationSpec::Pipeline { .. } => (),
        _ => panic!("Expected pipeline calibration"),
    }

    // Serialize back
    let reserialized = serde_json::to_string_pretty(&envelope)
        .expect("Failed to serialize envelope");

    // Re-parse
    let _reparsed: CalibrationEnvelope = serde_json::from_str(&reserialized)
        .expect("Failed to re-parse envelope");
}

