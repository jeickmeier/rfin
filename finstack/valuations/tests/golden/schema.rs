//! Serde structs for the `finstack.golden/1` fixture schema.
//!
//! See `docs/2026-04-30-golden-tests-framework-design.md` section 5.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Top-level fixture envelope. One per JSON file under `tests/golden/data/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldenFixture {
    /// Schema version. Must equal `"finstack.golden/1"`.
    pub schema_version: String,
    /// Snake-case unique name within domain.
    pub name: String,
    /// Dotted domain path: `rates.irs`, `analytics.performance`, etc.
    pub domain: String,
    /// One-sentence description.
    pub description: String,
    /// Source, capture, and review metadata.
    pub provenance: Provenance,
    /// Domain-specific input object. The selected runner deserializes it further.
    pub inputs: serde_json::Value,
    /// Map of metric name to reference value.
    pub expected_outputs: BTreeMap<String, f64>,
    /// Map of metric name to tolerance entry. Must cover every expected output.
    pub tolerances: BTreeMap<String, ToleranceEntry>,
}

/// Fixture provenance and review metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    /// YYYY-MM-DD market date the reference values represent.
    pub as_of: String,
    /// Source mode: quantlib | bloomberg-api | bloomberg-screen | intex | formula | textbook.
    pub source: String,
    /// Free-form source details such as QuantLib version, Bloomberg screen, or textbook page.
    pub source_detail: String,
    /// Username at capture time.
    pub captured_by: String,
    /// YYYY-MM-DD when fixture was first written.
    pub captured_on: String,
    /// Username at last review.
    pub last_reviewed_by: String,
    /// YYYY-MM-DD when fixture was last reviewed.
    pub last_reviewed_on: String,
    /// Review interval in months. Defaults to 6 by convention.
    pub review_interval_months: u32,
    /// Exact command to regenerate. Empty allowed for formula or textbook sources.
    pub regen_command: String,
    /// Image evidence for manual sources. Required for bloomberg-screen and intex fixtures.
    #[serde(default)]
    pub screenshots: Vec<Screenshot>,
}

/// Screenshot evidence for manually captured external references.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Screenshot {
    /// Path relative to the fixture JSON.
    pub path: String,
    /// Bloomberg or Intex screen name.
    pub screen: String,
    /// YYYY-MM-DD capture date.
    pub captured_on: String,
    /// Free-form description.
    pub description: String,
}

/// Per-metric tolerance. A comparison passes if either `abs` or `rel` is satisfied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToleranceEntry {
    /// Absolute tolerance: `|actual - expected| <= abs`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abs: Option<f64>,
    /// Relative tolerance: `|actual - expected| / max(|expected|, 1e-12) <= rel`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rel: Option<f64>,
    /// Explanation for any fixture-specific tolerance override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tolerance_reason: Option<String>,
}

/// Current golden fixture schema version.
pub const SCHEMA_VERSION: &str = "finstack.golden/1";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_minimal_fixture() {
        let json = r#"{
          "schema_version": "finstack.golden/1",
          "name": "test_fixture",
          "domain": "rates.irs",
          "description": "Minimal smoke fixture",
          "provenance": {
            "as_of": "2026-04-30",
            "source": "quantlib",
            "source_detail": "QL 1.34",
            "captured_by": "test",
            "captured_on": "2026-04-30",
            "last_reviewed_by": "test",
            "last_reviewed_on": "2026-04-30",
            "review_interval_months": 6,
            "regen_command": "uv run scripts/goldens/regen.py --kind irs-par"
          },
          "inputs": {"foo": 1},
          "expected_outputs": {"npv": 100.0},
          "tolerances": {"npv": {"abs": 0.01}}
        }"#;

        let fixture: GoldenFixture = serde_json::from_str(json).expect("fixture parses");

        assert_eq!(fixture.schema_version, SCHEMA_VERSION);
        assert_eq!(fixture.name, "test_fixture");
        assert_eq!(fixture.expected_outputs.get("npv"), Some(&100.0));
        assert!(fixture.provenance.screenshots.is_empty());
    }
}
