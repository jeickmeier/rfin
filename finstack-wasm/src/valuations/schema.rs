//! JSON schema accessors for instrument and valuation types.
//!
//! These bindings expose embedded JSON schemas from the `finstack-valuations`
//! crate so JavaScript consumers can validate instrument configurations,
//! generate UI forms, and inspect available instrument types.

use crate::core::error::{core_to_js, js_error};
use finstack_valuations::schema;
use wasm_bindgen::prelude::*;

/// Get JSON-Schema for Bond configuration.
///
/// Returns the embedded schema as a JavaScript object.
#[wasm_bindgen(js_name = bondSchema)]
pub fn bond_schema() -> Result<JsValue, JsValue> {
    let schema_val = schema::bond_schema().map_err(core_to_js)?;
    serde_wasm_bindgen::to_value(schema_val)
        .map_err(|e| js_error(format!("Schema serialization failed: {}", e)))
}

/// Get the JSON Schema for the instrument envelope.
///
/// Returns the top-level schema that validates instrument JSON envelopes.
#[wasm_bindgen(js_name = instrumentEnvelopeSchema)]
pub fn instrument_envelope_schema() -> Result<JsValue, JsValue> {
    let schema_val = schema::instrument_envelope_schema().map_err(core_to_js)?;
    serde_wasm_bindgen::to_value(schema_val)
        .map_err(|e| js_error(format!("Schema serialization failed: {}", e)))
}

/// Return the list of supported instrument type discriminators.
///
/// Returns an array of strings (e.g., `["bond", "deposit", "interest_rate_swap", ...]`).
#[wasm_bindgen(js_name = instrumentTypes)]
pub fn instrument_types() -> Result<JsValue, JsValue> {
    let types = schema::instrument_types().map_err(core_to_js)?;
    serde_wasm_bindgen::to_value(&types)
        .map_err(|e| js_error(format!("Serialization failed: {}", e)))
}

/// Get the JSON Schema for a specific instrument type.
///
/// Returns the dedicated schema if available, or a fallback for recognized types.
#[wasm_bindgen(js_name = instrumentSchema)]
pub fn instrument_schema(instrument_type: &str) -> Result<JsValue, JsValue> {
    let schema_val = schema::instrument_schema(instrument_type).map_err(core_to_js)?;
    serde_wasm_bindgen::to_value(&schema_val)
        .map_err(|e| js_error(format!("Schema serialization failed: {}", e)))
}

/// Get JSON-Schema for ValuationResult.
///
/// Returns the schema describing the structure of valuation result envelopes.
#[wasm_bindgen(js_name = valuationResultSchema)]
pub fn valuation_result_schema() -> Result<JsValue, JsValue> {
    let schema_val = schema::valuation_result_schema().map_err(core_to_js)?;
    serde_wasm_bindgen::to_value(schema_val)
        .map_err(|e| js_error(format!("Schema serialization failed: {}", e)))
}
