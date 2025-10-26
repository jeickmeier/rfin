use crate::core::dates::date::JsDate;
use crate::core::explain::WasmExplanationTrace;
use crate::core::money::JsMoney;
use finstack_core::config::ResultsMeta;
use finstack_valuations::results::ValuationResult;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

/// Results metadata wrapper for WASM.
///
/// Contains information about numeric mode, rounding context,
/// FX policy, timestamp, and library version.
#[wasm_bindgen(js_name = ResultsMeta)]
#[derive(Clone)]
pub struct JsResultsMeta {
    inner: ResultsMeta,
}

impl JsResultsMeta {
    pub(crate) fn new(inner: ResultsMeta) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = ResultsMeta)]
impl JsResultsMeta {
    /// Numeric engine mode (e.g., "f64").
    #[wasm_bindgen(getter, js_name = numericMode)]
    pub fn numeric_mode(&self) -> String {
        format!("{:?}", self.inner.numeric_mode).to_lowercase()
    }

    /// FX policy applied (if any).
    #[wasm_bindgen(getter, js_name = fxPolicyApplied)]
    pub fn fx_policy_applied(&self) -> Option<String> {
        self.inner.fx_policy_applied.clone()
    }

    /// Timestamp when result was computed (ISO 8601).
    #[wasm_bindgen(getter)]
    pub fn timestamp(&self) -> Option<String> {
        self.inner.timestamp.clone()
    }

    /// Library version used to produce this result.
    #[wasm_bindgen(getter)]
    pub fn version(&self) -> Option<String> {
        self.inner.version.clone()
    }

    /// Convert to JSON object.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }
}

#[wasm_bindgen(js_name = ValuationResult)]
#[derive(Clone)]
pub struct JsValuationResult {
    inner: ValuationResult,
}

impl JsValuationResult {
    pub(crate) fn new(inner: ValuationResult) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &ValuationResult {
        &self.inner
    }
}

#[wasm_bindgen(js_class = ValuationResult)]
impl JsValuationResult {
    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.instrument_id.clone()
    }

    #[wasm_bindgen(getter, js_name = asOf)]
    pub fn as_of(&self) -> JsDate {
        JsDate::from_core(self.inner.as_of)
    }

    #[wasm_bindgen(getter, js_name = presentValue)]
    pub fn present_value(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.value)
    }

    /// Get results metadata (timestamp, version, rounding context, etc.).
    #[wasm_bindgen(getter)]
    pub fn meta(&self) -> JsResultsMeta {
        JsResultsMeta::new(self.inner.meta.clone())
    }

    #[wasm_bindgen(js_name = metric)]
    pub fn metric(&self, name: &str) -> Option<f64> {
        self.inner.measures.get(name).copied()
    }

    #[wasm_bindgen(getter, js_name = measures)]
    pub fn measures(&self) -> js_sys::Map {
        let map = js_sys::Map::new();
        for (key, value) in &self.inner.measures {
            map.set(&JsValue::from_str(key), &JsValue::from_f64(*value));
        }
        map
    }

    /// Optional explanation trace if explain=true was passed.
    ///
    /// Returns detailed cashflow-level PV breakdown showing
    /// each cashflow, discount factor, and present value.
    ///
    /// @returns {ExplanationTrace | null} Trace object or null if explanation was disabled
    ///
    /// @example
    /// ```javascript
    /// const result = pricer.price(bond, market, asOf, true); // explain=true
    /// if (result.explanation) {
    ///     const trace = result.explanation;
    ///     console.log('Cashflows traced:', trace.entryCount);
    ///     
    ///     // Get full details
    ///     const details = trace.toJson();
    ///     for (const entry of details.entries) {
    ///         console.log(`${entry.date}: ${entry.cashflow_amount} -> PV: ${entry.pv_amount}`);
    ///     }
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
    /// @returns {string | null} Pretty-printed JSON or null if no explanation
    #[wasm_bindgen(js_name = explainJson)]
    pub fn explain_json(&self) -> Option<String> {
        self.inner
            .explanation
            .as_ref()
            .and_then(|trace| trace.to_json_pretty().ok())
    }
}
