use crate::core::dates::date::JsDate;
use crate::core::money::JsMoney;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::equity_option::EquityOption;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = EquityOption)]
#[derive(Clone, Debug)]
pub struct JsEquityOption {
    pub(crate) inner: EquityOption,
}

impl InstrumentWrapper for JsEquityOption {
    type Inner = EquityOption;
    fn from_inner(inner: EquityOption) -> Self {
        JsEquityOption { inner }
    }
    fn inner(&self) -> EquityOption {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = EquityOption)]
impl JsEquityOption {
    #[wasm_bindgen(js_name = europeanCall)]
    pub fn european_call(
        instrument_id: &str,
        ticker: &str,
        strike: f64,
        expiry: &JsDate,
        notional: &JsMoney,
        contract_size: Option<f64>,
    ) -> JsEquityOption {
        let option = EquityOption::european_call(
            instrument_id.to_string(),
            ticker,
            strike,
            expiry.inner(),
            notional.inner(),
            contract_size.unwrap_or(1.0),
        )
        .expect("EquityOption::european_call should succeed with valid parameters");
        JsEquityOption::from_inner(option)
    }

    #[wasm_bindgen(js_name = europeanPut)]
    pub fn european_put(
        instrument_id: &str,
        ticker: &str,
        strike: f64,
        expiry: &JsDate,
        notional: &JsMoney,
        contract_size: Option<f64>,
    ) -> JsEquityOption {
        let option = EquityOption::european_put(
            instrument_id.to_string(),
            ticker,
            strike,
            expiry.inner(),
            notional.inner(),
            contract_size.unwrap_or(1.0),
        )
        .expect("EquityOption::european_put should succeed with valid parameters");
        JsEquityOption::from_inner(option)
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn ticker(&self) -> String {
        self.inner.underlying_ticker.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn strike(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.strike)
    }

    #[wasm_bindgen(getter)]
    pub fn expiry(&self) -> JsDate {
        JsDate::from_core(self.inner.expiry)
    }

    #[wasm_bindgen(getter, js_name = contractSize)]
    pub fn contract_size(&self) -> f64 {
        self.inner.contract_size
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::EquityOption as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "EquityOption(id='{}', ticker='{}')",
            self.inner.id, self.inner.underlying_ticker
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsEquityOption {
        JsEquityOption::from_inner(self.inner.clone())
    }
}
