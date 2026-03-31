//! WASM bindings for CommoditySwaption instrument.

use crate::utils::json::to_js_value;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::commodity::commodity_swaption::CommoditySwaption;
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

/// Builder for commodity swaptions (JSON-based).
#[wasm_bindgen(js_name = CommoditySwaptionBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsCommoditySwaptionBuilder {
    /// JSON string payload.
    json_str: Option<String>,
}

#[wasm_bindgen(js_class = CommoditySwaptionBuilder)]
impl JsCommoditySwaptionBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsCommoditySwaptionBuilder {
        JsCommoditySwaptionBuilder { json_str: None }
    }

    #[wasm_bindgen(js_name = jsonString)]
    pub fn json_string(mut self, json_str: String) -> JsCommoditySwaptionBuilder {
        self.json_str = Some(json_str);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsCommoditySwaption, JsValue> {
        let json_str = self.json_str.as_deref().ok_or_else(|| {
            JsValue::from_str("CommoditySwaptionBuilder: jsonString is required")
        })?;
        JsCommoditySwaption::from_json_str(json_str)
    }
}

/// Commodity swaption (option on a fixed-for-floating commodity swap).
///
/// Configured via JSON payload matching the Rust model schema.
#[wasm_bindgen(js_name = CommoditySwaption)]
#[derive(Clone, Debug)]
pub struct JsCommoditySwaption {
    pub(crate) inner: CommoditySwaption,
}

impl InstrumentWrapper for JsCommoditySwaption {
    type Inner = CommoditySwaption;
    fn from_inner(inner: CommoditySwaption) -> Self {
        JsCommoditySwaption { inner }
    }
    fn inner(&self) -> CommoditySwaption {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = CommoditySwaption)]
impl JsCommoditySwaption {
    /// Parse from a JSON string.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json_str(json_str: &str) -> Result<JsCommoditySwaption, JsValue> {
        use crate::core::error::js_error;
        serde_json::from_str(json_str)
            .map(JsCommoditySwaption::from_inner)
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
        use crate::core::error::js_error;
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    /// Get the fixed price (strike) of the underlying swap.
    #[wasm_bindgen(getter, js_name = fixedPrice)]
    pub fn fixed_price(&self) -> f64 {
        self.inner.fixed_price
    }

    /// Get the notional quantity per period.
    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> f64 {
        self.inner.notional
    }

    /// Commodity swaptions return an empty cashflow schedule.
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(&self) -> Array {
        Array::new()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> String {
        InstrumentType::CommoditySwaption.to_string()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "CommoditySwaption(id='{}', fixedPrice={:.2})",
            self.inner.id, self.inner.fixed_price
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsCommoditySwaption {
        JsCommoditySwaption::from_inner(self.inner.clone())
    }
}
