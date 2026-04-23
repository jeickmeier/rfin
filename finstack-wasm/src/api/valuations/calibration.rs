//! WASM bindings for the calibration engine.
//!
//! Mirrors the Python `calibrate` / `validate_calibration_json` surface.
//! JSON-in / JSON-out: the caller passes a serialized `CalibrationEnvelope`
//! and receives the serialized `CalibrationResultEnvelope`.

use crate::utils::to_js_err;
use finstack_valuations::calibration::api::engine;
use finstack_valuations::calibration::api::schema::CalibrationEnvelope;
use wasm_bindgen::prelude::*;

/// Validate a calibration plan JSON and return the canonical (pretty-printed) form.
#[wasm_bindgen(js_name = validateCalibrationJson)]
pub fn validate_calibration_json(json: &str) -> Result<String, JsValue> {
    let parsed: CalibrationEnvelope = serde_json::from_str(json).map_err(to_js_err)?;
    serde_json::to_string_pretty(&parsed).map_err(to_js_err)
}

/// Execute a calibration plan and return the full result envelope as JSON.
///
/// Accepts a serialized `CalibrationEnvelope` (plan + quote sets + optional
/// initial market state) and returns a serialized `CalibrationResultEnvelope`.
#[wasm_bindgen(js_name = calibrate)]
pub fn calibrate(envelope_json: &str) -> Result<String, JsValue> {
    let envelope: CalibrationEnvelope = serde_json::from_str(envelope_json).map_err(to_js_err)?;
    let result = engine::execute(&envelope).map_err(to_js_err)?;
    serde_json::to_string(&result).map_err(to_js_err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::HashMap;
    use finstack_valuations::calibration::api::schema::{CalibrationPlan, CALIBRATION_SCHEMA};

    fn empty_envelope_json() -> String {
        let plan = CalibrationPlan {
            id: "empty".to_string(),
            description: None,
            quote_sets: HashMap::default(),
            steps: Vec::new(),
            settings: Default::default(),
        };
        let envelope = CalibrationEnvelope {
            schema: CALIBRATION_SCHEMA.to_string(),
            plan,
            initial_market: None,
        };
        serde_json::to_string(&envelope).expect("serialize")
    }

    #[test]
    fn validate_calibration_json_accepts_empty_plan() {
        let json = empty_envelope_json();
        let canonical = validate_calibration_json(&json).expect("validate");
        assert!(!canonical.is_empty());
    }

    #[test]
    fn calibrate_empty_plan_succeeds() {
        let json = empty_envelope_json();
        let result_json = calibrate(&json).expect("execute");
        let parsed: serde_json::Value = serde_json::from_str(&result_json).expect("json");
        assert!(parsed.is_object());
    }
}
