//! WASM bindings for explainability infrastructure.
//!
//! Provides JavaScript-friendly wrappers for ExplanationTrace and related types.

use finstack_core::explain::ExplanationTrace;
use wasm_bindgen::prelude::*;

/// WASM wrapper for ExplanationTrace.
///
/// Provides access to detailed execution traces from calibration, pricing,
/// and waterfall computations.
///
/// # Example (JavaScript)
///
/// ```javascript
/// const result = calibrateCurve(quotes, market, opts, true); // explain=true
/// if (result.explanation) {
///     const trace = result.explanation;
///     console.log(trace.traceType);  // "calibration"
///     console.log(trace.entries.length);  // Number of trace entries
///
///     // Convert to JSON for inspection
///     const json = trace.toJson();
///     console.log(JSON.stringify(json, null, 2));
/// }
/// ```
#[wasm_bindgen]
pub struct WasmExplanationTrace {
    inner: ExplanationTrace,
}

impl WasmExplanationTrace {
    pub(crate) fn new(inner: ExplanationTrace) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen]
impl WasmExplanationTrace {
    /// Get the trace type (e.g., "calibration", "pricing", "waterfall").
    #[wasm_bindgen(getter, js_name = traceType)]
    pub fn trace_type(&self) -> String {
        self.inner.trace_type.clone()
    }

    /// Get the number of trace entries.
    #[wasm_bindgen(getter, js_name = entryCount)]
    pub fn entry_count(&self) -> usize {
        self.inner.entries.len()
    }

    /// Check if the trace was truncated due to size limits.
    #[wasm_bindgen(getter, js_name = isTruncated)]
    pub fn is_truncated(&self) -> bool {
        self.inner.is_truncated()
    }

    /// Convert the entire trace to a JavaScript object.
    ///
    /// Returns a JS object with the full trace structure that can be
    /// inspected, logged, or sent to analytics.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    /// Get the trace as a pretty-printed JSON string.
    ///
    /// Useful for logging or debugging.
    #[wasm_bindgen(js_name = toJsonString)]
    pub fn to_json_string(&self) -> Result<String, JsValue> {
        self.inner
            .to_json_pretty()
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::explain::{ExplanationTrace, TraceEntry};

    #[test]
    fn test_wasm_explanation_trace_wrapper() {
        let mut trace = ExplanationTrace::new("calibration");
        trace.push(
            TraceEntry::CalibrationIteration {
                iteration: 0,
                residual: 0.005,
                knots_updated: vec!["2.5y".to_string()],
                converged: false,
            },
            Some(1000),
        );

        let wasm_trace = WasmExplanationTrace::new(trace);
        assert_eq!(wasm_trace.trace_type(), "calibration");
        assert_eq!(wasm_trace.entry_count(), 1);
        assert!(!wasm_trace.is_truncated());
    }

    #[test]
    fn test_json_string_generation() {
        let mut trace = ExplanationTrace::new("pricing");
        trace.push(
            TraceEntry::CashflowPV {
                date: "2025-06-15".to_string(),
                cashflow_amount: 50000.0,
                cashflow_currency: "USD".to_string(),
                discount_factor: 0.95,
                pv_amount: 47500.0,
                pv_currency: "USD".to_string(),
                curve_id: "USD_GOVT".to_string(),
            },
            None,
        );

        let wasm_trace = WasmExplanationTrace::new(trace);
        let json_result = wasm_trace.to_json_string();
        assert!(json_result.is_ok());
        let json_str = json_result.unwrap_or_default();
        assert!(json_str.contains("\"type\": \"pricing\""));
        assert!(json_str.contains("USD_GOVT"));
    }
}
