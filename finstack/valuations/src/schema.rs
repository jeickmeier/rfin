//! JSON-Schema helpers for Finstack types.
//!
//! Schemas are generated from the crate's serde-friendly types and checked in
//! under `schemas/`. These helpers expose them as `serde_json::Value` for use
//! in validation, UI forms, and contract generation.

use serde_json::Value;

macro_rules! include_schema {
    ($path:literal) => {
        serde_json::from_str(include_str!($path)).expect(concat!("invalid schema JSON at ", $path))
    };
}

/// Get JSON-Schema for Bond configuration.
///
/// Sourced from the generated instrument schemas under `schemas/instruments/1/`.
pub fn bond_schema() -> Value {
    include_schema!("../schemas/instruments/1/bond.schema.json")
}

/// Get JSON-Schema for CalibrationConfig.
///
/// Returns schema for calibration configuration options.
pub fn calibration_config_schema() -> Value {
    include_schema!("../schemas/calibration/2/calibration_config_v2.schema.json")
}

/// Get JSON-Schema for ValuationResult.
///
/// Returns schema for valuation result envelope (PV + metrics).
pub fn valuation_result_schema() -> Value {
    include_schema!("../schemas/results/1/valuation_result.schema.json")
}

/// Get JSON-Schema for CalibrationReport.
///
/// Returns schema for calibration diagnostic report.
pub fn calibration_report_schema() -> Value {
    include_schema!("../schemas/calibration/2/calibration_result.schema.json")
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
