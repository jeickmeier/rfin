use crate::utils::json::to_js_value;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::rates::cms_option::CmsOption;
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

/// CMS option (option on a swap rate) (JSON-serializable).
///
/// This instrument is configured via a JSON payload (matching the Rust model schema).
/// Use `fromJson()` to construct it and `toJsonString()` to inspect the canonical representation.
#[wasm_bindgen(js_name = CmsOption)]
#[derive(Clone, Debug)]
pub struct JsCmsOption {
    pub(crate) inner: CmsOption,
}

impl InstrumentWrapper for JsCmsOption {
    type Inner = CmsOption;
    fn from_inner(inner: CmsOption) -> Self {
        JsCmsOption { inner }
    }
    fn inner(&self) -> CmsOption {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = CmsOption)]
impl JsCmsOption {
    /// Parse a CMS option from a JSON string.
    ///
    /// @param json_str - JSON payload matching the CMS option schema
    /// @returns A new `CmsOption`
    /// @throws {Error} If the JSON cannot be parsed or is invalid
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(json_str: &str) -> Result<JsCmsOption, JsValue> {
        use crate::core::error::js_error;
        serde_json::from_str(json_str)
            .map(JsCmsOption::from_inner)
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

    /// Get a cashflow view for this CMS option.
    ///
    /// Option payoffs are path-dependent on the underlying swap rate; this returns an empty
    /// schedule placeholder for API consistency.
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(&self) -> Array {
        Array::new()
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
        InstrumentType::CmsOption as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!("CmsOption(id='{}')", self.inner.id.as_str())
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsCmsOption {
        JsCmsOption::from_inner(self.inner.clone())
    }
}
