//! Calibration report bindings for WASM.

use crate::core::explain::WasmExplanationTrace;
use crate::utils::json::{from_js_value, to_js_value};
use finstack_valuations::calibration::CalibrationReport;
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Calibration report with convergence details.
#[wasm_bindgen(js_name = CalibrationReport)]
#[derive(Clone, Debug)]
pub struct JsCalibrationReport {
    inner: CalibrationReport,
}

impl JsCalibrationReport {
    pub(crate) fn from_inner(inner: CalibrationReport) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> CalibrationReport {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = CalibrationReport)]
impl JsCalibrationReport {
    /// Whether the calibration succeeded.
    #[wasm_bindgen(getter)]
    pub fn success(&self) -> bool {
        self.inner.success
    }

    /// Number of iterations performed.
    #[wasm_bindgen(getter)]
    pub fn iterations(&self) -> usize {
        self.inner.iterations
    }

    /// Final objective value.
    #[wasm_bindgen(getter, js_name = objectiveValue)]
    pub fn objective_value(&self) -> f64 {
        self.inner.objective_value
    }

    /// Maximum residual across all calibration instruments.
    #[wasm_bindgen(getter, js_name = maxResidual)]
    pub fn max_residual(&self) -> f64 {
        self.inner.max_residual
    }

    /// Root mean square error.
    #[wasm_bindgen(getter)]
    pub fn rmse(&self) -> f64 {
        self.inner.rmse
    }

    /// Convergence reason message.
    #[wasm_bindgen(getter, js_name = convergenceReason)]
    pub fn convergence_reason(&self) -> String {
        self.inner.convergence_reason.clone()
    }

    /// Optional explanation trace if explain=true was passed.
    ///
    /// Returns detailed iteration-by-iteration diagnostics including
    /// residuals, knots updated, and convergence status.
    ///
    /// @returns {ExplanationTrace | null} Trace object or null if explanation was disabled
    ///
    /// @example
    /// ```javascript
    /// const report = calibrator.calibrate(quotes, market);
    /// if (report.explanation) {
    ///     console.log('Trace type:', report.explanation.traceType);
    ///     console.log('Iterations:', report.explanation.entryCount);
    ///     console.log(report.explanation.toJsonString());
    /// }
    /// ```
    #[wasm_bindgen(getter)]
    pub fn explanation(&self) -> Option<WasmExplanationTrace> {
        self.inner
            .explanation
            .as_ref()
            .map(|trace| WasmExplanationTrace::new(trace.clone()))
    }

    /// Get explanation trace as JSON string.
    ///
    /// Convenience method for logging or debugging.
    ///
    /// @returns {string | null} Pretty-printed JSON or null if no explanation
    #[wasm_bindgen(js_name = explainJson)]
    pub fn explain_json(&self) -> Option<String> {
        self.inner
            .explanation
            .as_ref()
            .and_then(|trace| trace.to_json_pretty().ok())
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsCalibrationReport, JsValue> {
        from_js_value(value).map(JsCalibrationReport::from_inner)
    }

    /// Convert report to JSON object.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        #[derive(Serialize)]
        struct ReportData {
            success: bool,
            iterations: usize,
            objective_value: f64,
            max_residual: f64,
            rmse: f64,
            convergence_reason: String,
            residuals: std::collections::BTreeMap<String, f64>,
            metadata: std::collections::BTreeMap<String, String>,
        }

        let data = ReportData {
            success: self.inner.success,
            iterations: self.inner.iterations,
            objective_value: self.inner.objective_value,
            max_residual: self.inner.max_residual,
            rmse: self.inner.rmse,
            convergence_reason: self.inner.convergence_reason.clone(),
            residuals: self.inner.residuals.clone(),
            metadata: self.inner.metadata.clone(),
        };

        to_js_value(&data)
    }

    /// Get residual for a specific instrument.
    #[wasm_bindgen(js_name = getResidual)]
    pub fn get_residual(&self, instrument_id: &str) -> Option<f64> {
        self.inner.residuals.get(instrument_id).copied()
    }

    /// Get metadata value.
    #[wasm_bindgen(js_name = getMetadata)]
    pub fn get_metadata(&self, key: &str) -> Option<String> {
        self.inner.metadata.get(key).cloned()
    }
}
