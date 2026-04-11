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
