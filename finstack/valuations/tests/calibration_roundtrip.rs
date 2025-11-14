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
fn test_full_market_pipeline_roundtrip() {
    // Load pipeline example
    let json = fs::read_to_string("tests/calibration/json_examples/full_market_pipeline.json")
        .expect("Failed to read example file");

    // Parse envelope
    let envelope =
        CalibrationEnvelope::from_json(&json).expect("Failed to parse calibration envelope");

    assert_eq!(envelope.schema, "finstack.calibration/1");

    // Serialize back
    let reserialized =
        serde_json::to_string_pretty(&envelope).expect("Failed to serialize envelope");

    // Re-parse
    let _reparsed: CalibrationEnvelope =
        serde_json::from_str(&reserialized).expect("Failed to re-parse envelope");
}

#[test]
fn test_rates_only_pipeline_roundtrip() {
    // Load rates example
    let json = fs::read_to_string("tests/calibration/json_examples/rates_only_pipeline.json")
        .expect("Failed to read example file");

    // Parse envelope
    let envelope =
        CalibrationEnvelope::from_json(&json).expect("Failed to parse calibration envelope");

    assert_eq!(envelope.schema, "finstack.calibration/1");

    // Serialize back
    let reserialized =
        serde_json::to_string_pretty(&envelope).expect("Failed to serialize envelope");

    // Re-parse
    let _reparsed: CalibrationEnvelope =
        serde_json::from_str(&reserialized).expect("Failed to re-parse envelope");
}

#[test]
fn test_credit_pipeline_roundtrip() {
    // Load credit example
    let json = fs::read_to_string("tests/calibration/json_examples/credit_pipeline.json")
        .expect("Failed to read example file");

    // Parse envelope
    let envelope =
        CalibrationEnvelope::from_json(&json).expect("Failed to parse calibration envelope");

    assert_eq!(envelope.schema, "finstack.calibration/1");

    // Serialize back
    let reserialized =
        serde_json::to_string_pretty(&envelope).expect("Failed to serialize envelope");

    // Re-parse
    let _reparsed: CalibrationEnvelope =
        serde_json::from_str(&reserialized).expect("Failed to re-parse envelope");
}

#[test]
fn test_vol_pipeline_roundtrip() {
    // Load vol surface example
    let json = fs::read_to_string("tests/calibration/json_examples/vol_pipeline.json")
        .expect("Failed to read example file");

    // Parse envelope
    let envelope =
        CalibrationEnvelope::from_json(&json).expect("Failed to parse calibration envelope");

    assert_eq!(envelope.schema, "finstack.calibration/1");

    // Serialize back
    let reserialized =
        serde_json::to_string_pretty(&envelope).expect("Failed to serialize envelope");

    // Re-parse
    let _reparsed: CalibrationEnvelope =
        serde_json::from_str(&reserialized).expect("Failed to re-parse envelope");
}

#[test]
#[cfg(feature = "slow")]
fn test_rates_pipeline_execution() {
    // Load and execute a rates-only pipeline calibration
    let json = fs::read_to_string("tests/calibration/json_examples/rates_only_pipeline.json")
        .expect("Failed to read example file");

    let envelope =
        CalibrationEnvelope::from_json(&json).expect("Failed to parse calibration envelope");

    // Execute calibration
    use finstack_valuations::calibration::CalibrationResultEnvelope;
    let result_envelope = envelope
        .execute(None)
        .expect("Calibration execution failed");

    assert_eq!(result_envelope.schema, "finstack.calibration/1");

    // Verify final market has at least one curve
    assert!(!result_envelope.result.final_market.curves.is_empty());

    // Serialize result to JSON
    let result_json = result_envelope
        .to_string()
        .expect("Failed to serialize result");

    // Deserialize result back
    let reparsed_result =
        CalibrationResultEnvelope::from_json(&result_json).expect("Failed to reparse result");

    // Verify structural equality
    assert_eq!(reparsed_result.schema, result_envelope.schema);
    assert_eq!(
        reparsed_result.result.final_market.curves.len(),
        result_envelope.result.final_market.curves.len()
    );
}

#[test]
#[cfg(feature = "slow")]
fn test_credit_pipeline_execution() {
    // Load and execute a credit pipeline calibration
    let json = fs::read_to_string("tests/calibration/json_examples/credit_pipeline.json")
        .expect("Failed to read example file");

    let envelope =
        CalibrationEnvelope::from_json(&json).expect("Failed to parse calibration envelope");

    // Execute calibration
    let result_envelope = envelope
        .execute(None)
        .expect("Calibration execution failed");

    assert_eq!(result_envelope.schema, "finstack.calibration/1");

    // Verify we have multiple curves (discount + hazard curves)
    assert!(result_envelope.result.final_market.curves.len() >= 2);

    // Verify step reports exist
    assert!(!result_envelope.result.step_reports.is_empty());
}
