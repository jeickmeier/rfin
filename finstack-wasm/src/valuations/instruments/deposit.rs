use crate::core::dates::date::JsDate;
use crate::core::dates::daycount::JsDayCount;
use crate::core::money::JsMoney;
use crate::core::utils::js_error;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use finstack_valuations::instruments::deposit::Deposit;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = Deposit)]
#[derive(Clone, Debug)]
pub struct JsDeposit {
    inner: Deposit,
}

impl JsDeposit {
    pub(crate) fn from_inner(inner: Deposit) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> Deposit {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = Deposit)]
impl JsDeposit {
    #[wasm_bindgen(constructor)]
    pub fn new(
        instrument_id: &str,
        notional: &JsMoney,
        start: &JsDate,
        end: &JsDate,
        day_count: &JsDayCount,
        discount_curve: &str,
        quote_rate: Option<f64>,
    ) -> Result<JsDeposit, JsValue> {
        Deposit::builder()
            .id(instrument_id_from_str(instrument_id))
            .notional(notional.inner())
            .start(start.inner())
            .end(end.inner())
            .day_count(day_count.inner())
            .disc_id(curve_id_from_str(discount_curve))
            .quote_rate_opt(quote_rate)
            .build()
            .map(JsDeposit::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(getter)]
    pub fn start(&self) -> JsDate {
        JsDate::from_core(self.inner.start)
    }

    #[wasm_bindgen(getter)]
    pub fn end(&self) -> JsDate {
        JsDate::from_core(self.inner.end)
    }

    #[wasm_bindgen(getter, js_name = dayCount)]
    pub fn day_count(&self) -> String {
        format!("{:?}", self.inner.day_count)
    }

    #[wasm_bindgen(getter, js_name = quoteRate)]
    pub fn quote_rate(&self) -> Option<f64> {
        self.inner.quote_rate
    }

    #[wasm_bindgen(getter, js_name = discountCurve)]
    pub fn discount_curve(&self) -> String {
        self.inner.disc_id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::Deposit as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "Deposit(id='{}', start='{}', end='{}', quote_rate={:?})",
            self.inner.id, self.inner.start, self.inner.end, self.inner.quote_rate
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsDeposit {
        JsDeposit::from_inner(self.inner.clone())
    }
}
