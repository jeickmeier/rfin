//! WASM bindings for FxDigitalOption instrument.

use crate::utils::json::to_js_value;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::fx::fx_digital_option::{DigitalPayoutType, FxDigitalOption};
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

/// Digital (binary) payout type for FX digital options.
#[wasm_bindgen(js_name = DigitalPayoutType)]
#[derive(Clone, Debug)]
pub struct JsDigitalPayoutType {
    /// Inner payout type.
    inner: DigitalPayoutType,
}

#[wasm_bindgen(js_class = DigitalPayoutType)]
impl JsDigitalPayoutType {
    /// Cash-or-nothing: pays a fixed cash amount if ITM at expiry.
    #[wasm_bindgen(js_name = CashOrNothing)]
    pub fn cash_or_nothing() -> JsDigitalPayoutType {
        JsDigitalPayoutType {
            inner: DigitalPayoutType::CashOrNothing,
        }
    }

    /// Asset-or-nothing: pays one unit of foreign currency if ITM at expiry.
    #[wasm_bindgen(js_name = AssetOrNothing)]
    pub fn asset_or_nothing() -> JsDigitalPayoutType {
        JsDigitalPayoutType {
            inner: DigitalPayoutType::AssetOrNothing,
        }
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("{:?}", self.inner)
    }
}

/// Builder for FX digital options (JSON-based).
#[wasm_bindgen(js_name = FxDigitalOptionBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsFxDigitalOptionBuilder {
    /// JSON string payload.
    json_str: Option<String>,
}

#[wasm_bindgen(js_class = FxDigitalOptionBuilder)]
impl JsFxDigitalOptionBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsFxDigitalOptionBuilder {
        JsFxDigitalOptionBuilder { json_str: None }
    }

    #[wasm_bindgen(js_name = jsonString)]
    pub fn json_string(mut self, json_str: String) -> JsFxDigitalOptionBuilder {
        self.json_str = Some(json_str);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsFxDigitalOption, JsValue> {
        let json_str = self
            .json_str
            .as_deref()
            .ok_or_else(|| JsValue::from_str("FxDigitalOptionBuilder: jsonString is required"))?;
        JsFxDigitalOption::from_json_str(json_str)
    }
}

/// FX digital (binary) option instrument.
///
/// Pays a fixed amount if the option expires in-the-money.
/// Configured via JSON payload matching the Rust model schema.
#[wasm_bindgen(js_name = FxDigitalOption)]
#[derive(Clone, Debug)]
pub struct JsFxDigitalOption {
    pub(crate) inner: FxDigitalOption,
}

impl InstrumentWrapper for JsFxDigitalOption {
    type Inner = FxDigitalOption;
    fn from_inner(inner: FxDigitalOption) -> Self {
        JsFxDigitalOption { inner }
    }
    fn inner(&self) -> FxDigitalOption {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = FxDigitalOption)]
impl JsFxDigitalOption {
    /// Parse from a JSON string.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json_str(json_str: &str) -> Result<JsFxDigitalOption, JsValue> {
        use crate::core::error::js_error;
        serde_json::from_str(json_str)
            .map(JsFxDigitalOption::from_inner)
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

    /// Get the strike exchange rate.
    #[wasm_bindgen(getter)]
    pub fn strike(&self) -> f64 {
        self.inner.strike
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

    /// Digital options return an empty cashflow schedule.
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(&self) -> Array {
        Array::new()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> String {
        InstrumentType::FxDigitalOption.to_string()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "FxDigitalOption(id='{}', strike={:.4})",
            self.inner.id, self.inner.strike
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsFxDigitalOption {
        JsFxDigitalOption::from_inner(self.inner.clone())
    }
}
