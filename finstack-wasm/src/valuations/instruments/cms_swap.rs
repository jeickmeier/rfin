//! WASM bindings for CmsSwap instrument.

use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::to_js_value;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::rates::cms_swap::CmsSwap;
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

/// Builder for CMS swaps (JSON-based).
#[wasm_bindgen(js_name = CmsSwapBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsCmsSwapBuilder {
    /// JSON string payload.
    json_str: Option<String>,
}

#[wasm_bindgen(js_class = CmsSwapBuilder)]
impl JsCmsSwapBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsCmsSwapBuilder {
        JsCmsSwapBuilder { json_str: None }
    }

    #[wasm_bindgen(js_name = jsonString)]
    pub fn json_string(mut self, json_str: String) -> JsCmsSwapBuilder {
        self.json_str = Some(json_str);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsCmsSwap, JsValue> {
        let json_str = self
            .json_str
            .as_deref()
            .ok_or_else(|| JsValue::from_str("CmsSwapBuilder: jsonString is required"))?;
        JsCmsSwap::from_json_str(json_str)
    }
}

/// CMS (Constant Maturity Swap) swap instrument.
///
/// One leg pays a CMS rate (par swap rate for a reference tenor) and the other
/// leg pays a fixed or floating rate. Configured via JSON payload.
#[wasm_bindgen(js_name = CmsSwap)]
#[derive(Clone, Debug)]
pub struct JsCmsSwap {
    pub(crate) inner: CmsSwap,
}

impl InstrumentWrapper for JsCmsSwap {
    type Inner = CmsSwap;
    fn from_inner(inner: CmsSwap) -> Self {
        JsCmsSwap { inner }
    }
    fn inner(&self) -> CmsSwap {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = CmsSwap)]
impl JsCmsSwap {
    /// Parse from a JSON string.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json_str(json_str: &str) -> Result<JsCmsSwap, JsValue> {
        serde_json::from_str(json_str)
            .map(JsCmsSwap::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    /// Serialize to a pretty-printed JSON string.
    #[wasm_bindgen(js_name = toJsonString)]
    pub fn to_json_string(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    /// Get the notional amount.
    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    /// Get the CMS tenor in years (e.g., 10.0 for 10Y swap rate).
    #[wasm_bindgen(getter, js_name = cmsTenor)]
    pub fn cms_tenor(&self) -> f64 {
        self.inner.cms_tenor
    }

    /// Get cashflows for this CMS swap (requires market context for CMS rate projection).
    ///
    /// Returns an empty array since CMS flows depend on market data.
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(&self) -> Array {
        Array::new()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> String {
        InstrumentType::CmsSwap.to_string()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "CmsSwap(id='{}', cmsTenor={:.0}Y)",
            self.inner.id, self.inner.cms_tenor
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsCmsSwap {
        JsCmsSwap::from_inner(self.inner.clone())
    }
}
