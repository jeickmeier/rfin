//! WASM bindings for the calibration engine.
//!
//! Mirrors the Python `calibrate` / `validate_calibration_json` surface plus
//! Phase 4 diagnostics (`dryRun`, `dependencyGraphJson`).
//!
//! On error, all four functions throw a JS `Error` with `name =
//! "CalibrationEnvelopeError"` and a structured `cause` property carrying
//! the serialized `EnvelopeError` payload. Standard `try/catch (e)` exposes
//! both via `e.name` and `e.cause`.

use crate::utils::to_js_err;
use finstack_valuations::calibration::api::engine::{self, ExecuteError};
use finstack_valuations::calibration::api::errors::EnvelopeError;
use finstack_valuations::calibration::api::schema::CalibrationEnvelope;
use finstack_valuations::calibration::api::validate;
use wasm_bindgen::prelude::*;

/// Validate a calibration plan JSON and return the canonical (pretty-printed) form.
#[wasm_bindgen(js_name = validateCalibrationJson)]
pub fn validate_calibration_json(json: &str) -> Result<String, JsValue> {
    let parsed: CalibrationEnvelope = serde_json::from_str(json).map_err(|e| {
        envelope_error_to_js(&EnvelopeError::JsonParse {
            message: e.to_string(),
            line: Some(e.line() as u32),
            col: Some(e.column() as u32),
        })
    })?;
    serde_json::to_string_pretty(&parsed).map_err(to_js_err)
}

/// Execute a calibration plan and return the full result envelope as JSON.
///
/// Accepts a serialized `CalibrationEnvelope` (plan + quote sets + optional
/// initial market state) and returns a serialized `CalibrationResultEnvelope`.
#[wasm_bindgen(js_name = calibrate)]
pub fn calibrate(envelope_json: &str) -> Result<String, JsValue> {
    let envelope: CalibrationEnvelope = serde_json::from_str(envelope_json).map_err(|e| {
        envelope_error_to_js(&EnvelopeError::JsonParse {
            message: e.to_string(),
            line: Some(e.line() as u32),
            col: Some(e.column() as u32),
        })
    })?;
    let result = engine::execute_with_diagnostics(&envelope).map_err(execute_error_to_js)?;
    serde_json::to_string(&result).map_err(to_js_err)
}

/// Pre-flight envelope validation without invoking the solver.
///
/// Returns a JSON-serialized `ValidationReport` listing every error found
/// plus the dependency graph. Microseconds.
#[wasm_bindgen(js_name = dryRun)]
pub fn dry_run(envelope_json: &str) -> Result<String, JsValue> {
    validate::dry_run(envelope_json).map_err(|e| envelope_error_to_js(&e))
}

/// Returns the static dependency graph of a calibration plan as JSON.
#[wasm_bindgen(js_name = dependencyGraphJson)]
pub fn dependency_graph_json(envelope_json: &str) -> Result<String, JsValue> {
    validate::dependency_graph_json(envelope_json).map_err(|e| envelope_error_to_js(&e))
}

/// Throw a JS `Error` with `name = "CalibrationEnvelopeError"` and a
/// structured `cause` property carrying the serialized payload.
fn envelope_error_to_js(err: &EnvelopeError) -> JsValue {
    let display = err.to_string();
    let cause_json = err.to_json();

    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::{Error as JsError, Reflect, JSON};

        let js_err = JsError::new(&display);
        js_err.set_name("CalibrationEnvelopeError");

        // Attach structured cause as a JS object (parsed from JSON) when
        // possible; fall back to the raw string if parsing fails.
        let cause_value: JsValue = match JSON::parse(&cause_json) {
            Ok(v) => v,
            Err(_) => JsValue::from_str(&cause_json),
        };
        let _ = Reflect::set(&js_err, &JsValue::from_str("cause"), &cause_value);

        js_err.into()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (display, cause_json);
        JsValue::NULL
    }
}

/// Map an [`ExecuteError`] (returned by `engine::execute_with_diagnostics`)
/// to a JS-side error, preserving the structured envelope payload when present.
fn execute_error_to_js(err: ExecuteError) -> JsValue {
    match err {
        ExecuteError::Envelope(env) => envelope_error_to_js(&env),
        ExecuteError::Other(other) => to_js_err(other),
    }
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
            schema_url: None,
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

    #[test]
    fn dry_run_accepts_empty_plan() {
        let json = empty_envelope_json();
        let report_json = dry_run(&json).expect("dry_run");
        let parsed: serde_json::Value = serde_json::from_str(&report_json).expect("json");
        assert!(parsed.get("errors").is_some());
        assert!(parsed.get("dependency_graph").is_some());
    }

    #[test]
    fn dependency_graph_json_for_empty_plan() {
        let json = empty_envelope_json();
        let graph_json = dependency_graph_json(&json).expect("dep graph");
        let parsed: serde_json::Value = serde_json::from_str(&graph_json).expect("json");
        assert!(parsed.get("initial_ids").is_some());
        assert!(parsed.get("nodes").is_some());
    }

    #[test]
    fn dry_run_rejects_malformed_json() {
        // Native target returns JsValue::NULL for the error path; the
        // important assertion is that we return Err, not panic.
        assert!(dry_run("not json").is_err());
    }
}
