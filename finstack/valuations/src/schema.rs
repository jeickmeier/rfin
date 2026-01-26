//! JSON-Schema helpers for Finstack types.
//!
//! Schemas are generated from the crate's serde-friendly types and checked in
//! under `schemas/`. These helpers expose them as `serde_json::Value` for use
//! in validation, UI forms, and contract generation.
//!
//! # Error Handling
//!
//! All schema accessors return `Result<&'static Value>` instead of panicking,
//! allowing callers to handle schema loading failures gracefully.

use serde_json::Value;
use std::sync::OnceLock;

/// Parse embedded JSON schema at compile time, returning a Result.
/// The JSON is embedded via `include_str!` so the content is always present,
/// but parsing can still fail if the JSON is malformed.
macro_rules! try_include_schema {
    ($path:literal) => {
        serde_json::from_str::<Value>(include_str!($path))
            .map_err(|e| format!("invalid schema JSON at {}: {}", $path, e))
    };
}

/// Get JSON-Schema for Bond configuration.
///
/// Sourced from the generated instrument schemas under `schemas/instruments/1/`.
///
/// # Errors
///
/// Returns `Error::Validation` if the embedded schema JSON is malformed.
#[allow(dead_code)]
pub fn bond_schema() -> finstack_core::Result<&'static Value> {
    static SCHEMA: OnceLock<Result<Value, String>> = OnceLock::new();
    SCHEMA
        .get_or_init(|| {
            try_include_schema!("../schemas/instruments/1/fixed_income/bond.schema.json")
        })
        .as_ref()
        .map_err(|e| finstack_core::Error::Validation(e.clone()))
}

/// Get JSON-Schema for ValuationResult.
///
/// Returns schema for valuation result envelope (PV + metrics).
///
/// # Errors
///
/// Returns `Error::Validation` if the embedded schema JSON is malformed.
#[allow(dead_code)]
pub fn valuation_result_schema() -> finstack_core::Result<&'static Value> {
    static SCHEMA: OnceLock<Result<Value, String>> = OnceLock::new();
    SCHEMA
        .get_or_init(|| try_include_schema!("../schemas/results/1/valuation_result.schema.json"))
        .as_ref()
        .map_err(|e| finstack_core::Error::Validation(e.clone()))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_stubs() {
        // Verify stub schemas are valid JSON and have expected structure
        let bond = bond_schema().expect("bond schema should parse");
        assert_eq!(bond["$schema"], "http://json-schema.org/draft-07/schema#");
        assert_eq!(bond["title"], "Bond");

        let result = valuation_result_schema().expect("valuation result schema should parse");
        assert_eq!(result["title"], "ValuationResult");
    }

    #[test]
    fn test_all_schemas_parse_successfully() {
        // Ensure all embedded schemas parse without error.
        // This test catches invalid JSON at CI time rather than runtime.
        assert!(bond_schema().is_ok(), "bond_schema() should return Ok");
        assert!(
            valuation_result_schema().is_ok(),
            "valuation_result_schema() should return Ok"
        );
    }
}
