//! WASM bindings for FxTouchOption instrument.

use crate::utils::json::to_js_value;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::fx::fx_touch_option::{
    FxTouchOption, PayoutTiming, TouchType,
};
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

/// Touch type for FX touch options: one-touch or no-touch.
#[wasm_bindgen(js_name = TouchType)]
#[derive(Clone, Debug)]
pub struct JsTouchType {
    /// Inner touch type.
    inner: TouchType,
}

#[wasm_bindgen(js_class = TouchType)]
impl JsTouchType {
    /// One-touch: pays if the spot rate touches the barrier.
    #[wasm_bindgen(js_name = OneTouch)]
    pub fn one_touch() -> JsTouchType {
        JsTouchType {
            inner: TouchType::OneTouch,
        }
    }

    /// No-touch: pays if the spot rate does NOT touch the barrier.
    #[wasm_bindgen(js_name = NoTouch)]
    pub fn no_touch() -> JsTouchType {
        JsTouchType {
            inner: TouchType::NoTouch,
        }
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("{:?}", self.inner)
    }
}

/// Payout timing for FX touch options: at hit or at expiry.
#[wasm_bindgen(js_name = PayoutTiming)]
#[derive(Clone, Debug)]
pub struct JsPayoutTiming {
    /// Inner payout timing.
    inner: PayoutTiming,
}

#[wasm_bindgen(js_class = PayoutTiming)]
impl JsPayoutTiming {
    /// Payout occurs immediately when barrier is hit.
    #[wasm_bindgen(js_name = AtHit)]
    pub fn at_hit() -> JsPayoutTiming {
        JsPayoutTiming {
            inner: PayoutTiming::AtHit,
        }
    }

    /// Payout is deferred to expiry.
    #[wasm_bindgen(js_name = AtExpiry)]
    pub fn at_expiry() -> JsPayoutTiming {
        JsPayoutTiming {
            inner: PayoutTiming::AtExpiry,
        }
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("{:?}", self.inner)
    }
}

/// Builder for FX touch options (JSON-based).
#[wasm_bindgen(js_name = FxTouchOptionBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsFxTouchOptionBuilder {
    /// JSON string payload.
    json_str: Option<String>,
}

#[wasm_bindgen(js_class = FxTouchOptionBuilder)]
impl JsFxTouchOptionBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsFxTouchOptionBuilder {
        JsFxTouchOptionBuilder { json_str: None }
    }

    #[wasm_bindgen(js_name = jsonString)]
    pub fn json_string(mut self, json_str: String) -> JsFxTouchOptionBuilder {
        self.json_str = Some(json_str);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsFxTouchOption, JsValue> {
        let json_str = self
            .json_str
            .as_deref()
            .ok_or_else(|| JsValue::from_str("FxTouchOptionBuilder: jsonString is required"))?;
        JsFxTouchOption::from_json_str(json_str)
    }
}

/// FX touch option (American binary option).
///
/// Touch options pay a fixed amount if the spot rate touches a barrier
/// level at any time before expiry.
/// Configured via JSON payload matching the Rust model schema.
#[wasm_bindgen(js_name = FxTouchOption)]
#[derive(Clone, Debug)]
pub struct JsFxTouchOption {
    pub(crate) inner: FxTouchOption,
}

impl InstrumentWrapper for JsFxTouchOption {
    type Inner = FxTouchOption;
    fn from_inner(inner: FxTouchOption) -> Self {
        JsFxTouchOption { inner }
    }
    fn inner(&self) -> FxTouchOption {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = FxTouchOption)]
impl JsFxTouchOption {
    /// Parse from a JSON string.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json_str(json_str: &str) -> Result<JsFxTouchOption, JsValue> {
        use crate::core::error::js_error;
        serde_json::from_str(json_str)
            .map(JsFxTouchOption::from_inner)
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

    /// Get the barrier level.
    #[wasm_bindgen(getter, js_name = barrierLevel)]
    pub fn barrier_level(&self) -> f64 {
        self.inner.barrier_level
    }

    /// Get the base currency.
    #[wasm_bindgen(getter, js_name = baseCurrency)]
    pub fn base_currency(&self) -> String {
        self.inner.base_currency.to_string()
    }

    /// Get the quote currency.
    #[wasm_bindgen(getter, js_name = quoteCurrency)]
    pub fn quote_currency(&self) -> String {
        self.inner.quote_currency.to_string()
    }

    /// Touch options return an empty cashflow schedule.
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(&self) -> Array {
        Array::new()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> String {
        InstrumentType::FxTouchOption.to_string()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "FxTouchOption(id='{}', barrier={:.4})",
            self.inner.id, self.inner.barrier_level
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsFxTouchOption {
        JsFxTouchOption::from_inner(self.inner.clone())
    }
}
