use crate::utils::json::to_js_value;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::rates::range_accrual::RangeAccrual;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

/// Range accrual note (JSON-serializable).
///
/// This instrument is configured via a JSON payload (matching the Rust model schema).
/// Use `fromJson()` to construct it and `toJsonString()` to inspect the canonical representation.
#[wasm_bindgen(js_name = RangeAccrual)]
#[derive(Clone, Debug)]
pub struct JsRangeAccrual {
    pub(crate) inner: RangeAccrual,
}

impl InstrumentWrapper for JsRangeAccrual {
    type Inner = RangeAccrual;
    fn from_inner(inner: RangeAccrual) -> Self {
        JsRangeAccrual { inner }
    }
    fn inner(&self) -> RangeAccrual {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = RangeAccrual)]
impl JsRangeAccrual {
    /// Parse a range accrual instrument from a JSON string.
    ///
    /// @param json_str - JSON payload matching the range accrual schema
    /// @returns A new `RangeAccrual`
    /// @throws {Error} If the JSON cannot be parsed or is invalid
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsRangeAccrual, JsValue> {
        use crate::core::error::js_error;
        serde_json::from_str(json_str)
            .map(JsRangeAccrual::from_inner)
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

    /// Serialize this instrument to a pretty-printed JSON string.
    ///
    /// @returns JSON string
    /// @throws {Error} If serialization fails
    #[wasm_bindgen(js_name = toJsonString)]
    pub fn to_json_string(&self) -> Result<String, JsValue> {
        use crate::core::error::js_error;
        serde_json::to_string_pretty(&self.inner).map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::RangeAccrual as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("RangeAccrual(id='{}')", self.inner.id.as_str())
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsRangeAccrual {
        JsRangeAccrual::from_inner(self.inner.clone())
    }
}
