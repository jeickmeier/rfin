use crate::core::dates::date::JsDate;
use crate::core::money::JsMoney;
use crate::utils::json::to_js_value;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::rates::range_accrual::RangeAccrual;
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = RangeAccrualBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsRangeAccrualBuilder {
    json_str: Option<String>,
}

#[wasm_bindgen(js_class = RangeAccrualBuilder)]
impl JsRangeAccrualBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsRangeAccrualBuilder {
        JsRangeAccrualBuilder { json_str: None }
    }

    #[wasm_bindgen(js_name = jsonString)]
    pub fn json_string(mut self, json_str: String) -> JsRangeAccrualBuilder {
        self.json_str = Some(json_str);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsRangeAccrual, JsValue> {
        let json_str = self
            .json_str
            .as_deref()
            .ok_or_else(|| JsValue::from_str("RangeAccrualBuilder: jsonString is required"))?;
        JsRangeAccrual::from_json(json_str)
    }
}

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
        web_sys::console::warn_1(&JsValue::from_str(
            "RangeAccrual.fromJson is deprecated; use RangeAccrualBuilder instead.",
        ));
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

    /// Get a simple cashflow view for this range accrual.
    ///
    /// This returns a single placeholder cashflow at the payment date (or last observation date),
    /// since the realized in-range fraction depends on the path of the underlying.
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(&self) -> Result<Array, JsValue> {
        let pay_date = if let Some(date) = self.inner.payment_date {
            date
        } else if let Some(last_obs) = self.inner.observation_dates.last() {
            *last_obs
        } else {
            return Err(JsValue::from_str(
                "RangeAccrual has no payment date and no observation dates",
            ));
        };

        let entry = Array::new();
        entry.push(&JsDate::from_core(pay_date).into());
        entry.push(
            &JsMoney::from_inner(finstack_core::money::Money::new(
                0.0,
                self.inner.notional.currency(),
            ))
            .into(),
        );
        entry.push(&JsValue::from_str("RangeAccrualPayoff"));
        entry.push(&JsValue::NULL);

        let result = Array::new();
        result.push(&entry);
        Ok(result)
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
