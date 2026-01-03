use crate::core::dates::date::JsDate;
use crate::core::money::JsMoney;
use crate::utils::json::to_js_value;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::rates::range_accrual::RangeAccrual;
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
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

    /// Get a simple cashflow view for this range accrual.
    ///
    /// This returns a single placeholder cashflow at the payment date (or last observation date),
    /// since the realized in-range fraction depends on the path of the underlying.
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(&self) -> Result<Array, JsValue> {
        let pay_date = self
            .inner
            .payment_date
            .unwrap_or_else(|| *self.inner.observation_dates.last().unwrap());

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
