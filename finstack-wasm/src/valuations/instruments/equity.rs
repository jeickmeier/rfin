use crate::core::currency::JsCurrency;
use finstack_valuations::instruments::equity::Equity;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = Equity)]
#[derive(Clone, Debug)]
pub struct JsEquity {
    inner: Equity,
}

impl JsEquity {
    pub(crate) fn from_inner(inner: Equity) -> Self {
        Self { inner }
    }

    pub(crate) fn inner(&self) -> Equity {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = Equity)]
impl JsEquity {
    #[wasm_bindgen(constructor)]
    pub fn new(
        instrument_id: &str,
        ticker: &str,
        currency: &JsCurrency,
        shares: Option<f64>,
        price: Option<f64>,
    ) -> JsEquity {
        let mut equity = Equity::new(
            instrument_id.to_string(),
            ticker,
            currency.inner(),
        );
        if let Some(qty) = shares {
            equity = equity.with_shares(qty);
        }
        if let Some(px) = price {
            equity = equity.with_price(px);
        }
        JsEquity::from_inner(equity)
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn ticker(&self) -> String {
        self.inner.ticker.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn currency(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.currency)
    }

    #[wasm_bindgen(getter)]
    pub fn shares(&self) -> f64 {
        self.inner.effective_shares()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::Equity as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "Equity(id='{}', ticker='{}', shares={})",
            self.inner.id, self.inner.ticker, self.shares()
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsEquity {
        JsEquity::from_inner(self.inner.clone())
    }
}

