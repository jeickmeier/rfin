//! WASM bindings for the `finstack-statements` crate.
//!
//! Exposes JSON round-trip for financial model specifications.

use crate::utils::to_js_err;
use wasm_bindgen::prelude::*;

/// Validate a `FinancialModelSpec` JSON string.
///
/// Deserializes the input against the model schema and returns
/// the canonical (re-serialized) JSON.
#[wasm_bindgen(js_name = validateFinancialModelJson)]
pub fn validate_financial_model_json(json: &str) -> Result<String, JsValue> {
    let model: finstack_statements::FinancialModelSpec =
        serde_json::from_str(json).map_err(to_js_err)?;
    serde_json::to_string(&model).map_err(to_js_err)
}

/// Get the node identifiers from a model specification JSON.
///
/// Returns a JS array of node ID strings in declaration order.
#[wasm_bindgen(js_name = modelNodeIds)]
pub fn model_node_ids(json: &str) -> Result<JsValue, JsValue> {
    let model: finstack_statements::FinancialModelSpec =
        serde_json::from_str(json).map_err(to_js_err)?;
    let ids: Vec<&str> = model.nodes.keys().map(|k| k.as_str()).collect();
    serde_wasm_bindgen::to_value(&ids).map_err(to_js_err)
}

/// Validate a `CheckSuiteSpec` JSON string.
///
/// Deserializes the spec, re-serializes to canonical form, and
/// returns the JSON string. Useful for client-side validation.
#[wasm_bindgen(js_name = validateCheckSuiteSpec)]
pub fn validate_check_suite_spec(json: &str) -> Result<String, JsValue> {
    let spec: finstack_statements::checks::CheckSuiteSpec =
        serde_json::from_str(json).map_err(to_js_err)?;
    serde_json::to_string(&spec).map_err(to_js_err)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn validate_financial_model_json_accepts_minimal_model() {
        let model = finstack_statements::FinancialModelSpec::new("test", vec![]);
        let Ok(json) = serde_json::to_string(&model) else {
            panic!("model should serialize to JSON");
        };
        let Ok(out) = validate_financial_model_json(&json) else {
            panic!("validate_financial_model_json should accept minimal model");
        };
        let Ok(round_trip) = serde_json::from_str::<finstack_statements::FinancialModelSpec>(&out)
        else {
            panic!("validated JSON should deserialize");
        };
        assert_eq!(round_trip.id, "test");
        assert!(round_trip.nodes.is_empty());
    }

    #[test]
    fn validate_check_suite_spec_roundtrip() {
        let spec = finstack_statements::checks::CheckSuiteSpec {
            name: "test".to_string(),
            description: None,
            builtin_checks: vec![],
            formula_checks: vec![],
            config: finstack_statements::checks::CheckConfig::default(),
        };
        let json = serde_json::to_string(&spec).expect("serialize");
        let Ok(out) = validate_check_suite_spec(&json) else {
            panic!("should accept valid spec");
        };
        let Ok(rt) = serde_json::from_str::<finstack_statements::checks::CheckSuiteSpec>(&out)
        else {
            panic!("should roundtrip");
        };
        assert_eq!(rt.name, "test");
    }

    // -- Boundary tests ------------------------------------------------
    // Error paths create JsValue, which panics on native targets.
    // Test the underlying serde deserialization instead.

    #[test]
    fn validate_rejects_invalid_json() {
        assert!(
            serde_json::from_str::<finstack_statements::FinancialModelSpec>("not json").is_err()
        );
    }

    #[test]
    fn validate_rejects_empty_string() {
        assert!(serde_json::from_str::<finstack_statements::FinancialModelSpec>("").is_err());
    }
}
