//! JSON-Schema generation for Finstack types.
//!
//! Provides schema getters for validation in pipelines, UI forms, and API contracts.
//!
//! **Note**: This module currently provides stub implementations.
//! Full schema generation with `schemars` derives will be added in a future release.

use serde_json::Value;

/// Get JSON-Schema for Bond configuration.
///
/// Returns a JSON-Schema Draft 7 schema that describes the Bond struct.
/// Useful for validation in pipelines or schema-driven UI forms.
///
/// **Note**: Currently returns a stub. Full schema generation coming soon.
pub fn bond_schema() -> Value {
    serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Bond",
        "description": "Fixed-rate or floating-rate bond instrument",
        "type": "object",
        "note": "Full schema generation requires JsonSchema derives - coming in future release"
    })
}

/// Get JSON-Schema for CalibrationConfig.
///
/// Returns schema for calibration configuration options.
///
/// **Note**: Currently returns a stub. Full schema generation coming soon.
pub fn calibration_config_schema() -> Value {
    serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "CalibrationConfig",
        "description": "Configuration for calibration processes",
        "type": "object",
        "note": "Full schema generation requires JsonSchema derives - coming in future release"
    })
}

/// Get JSON-Schema for ValuationResult.
///
/// Returns schema for valuation result envelope.
///
/// **Note**: Currently returns a stub. Full schema generation coming soon.
pub fn valuation_result_schema() -> Value {
    serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "ValuationResult",
        "description": "Valuation result with PV and metrics",
        "type": "object",
        "note": "Full schema generation requires JsonSchema derives - coming in future release"
    })
}

/// Get JSON-Schema for CalibrationReport.
///
/// Returns schema for calibration diagnostic report.
///
/// **Note**: Currently returns a stub. Full schema generation coming soon.
pub fn calibration_report_schema() -> Value {
    serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "CalibrationReport",
        "description": "Calibration diagnostic report",
        "type": "object",
        "note": "Full schema generation requires JsonSchema derives - coming in future release"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_stubs() {
        // Verify stub schemas are valid JSON and have expected structure
        let bond = bond_schema();
        assert_eq!(bond["$schema"], "http://json-schema.org/draft-07/schema#");
        assert_eq!(bond["title"], "Bond");

        let config = calibration_config_schema();
        assert_eq!(config["title"], "CalibrationConfig");

        let result = valuation_result_schema();
        assert_eq!(result["title"], "ValuationResult");

        let report = calibration_report_schema();
        assert_eq!(report["title"], "CalibrationReport");
    }
}

