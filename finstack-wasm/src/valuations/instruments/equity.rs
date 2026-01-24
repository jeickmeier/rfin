use crate::core::currency::JsCurrency;
use crate::core::error::js_error;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::equity::Equity;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = EquityBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsEquityBuilder {
    instrument_id: String,
    ticker: Option<String>,
    currency: Option<finstack_core::currency::Currency>,
    shares: Option<f64>,
    price: Option<f64>,
}

#[wasm_bindgen(js_class = EquityBuilder)]
impl JsEquityBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsEquityBuilder {
        JsEquityBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = ticker)]
    pub fn ticker(mut self, ticker: String) -> JsEquityBuilder {
        self.ticker = Some(ticker);
        self
    }

    #[wasm_bindgen(js_name = currency)]
    pub fn currency(mut self, currency: &JsCurrency) -> JsEquityBuilder {
        self.currency = Some(currency.inner());
        self
    }

    #[wasm_bindgen(js_name = shares)]
    pub fn shares(mut self, shares: f64) -> JsEquityBuilder {
        self.shares = Some(shares);
        self
    }

    #[wasm_bindgen(js_name = price)]
    pub fn price(mut self, price: f64) -> JsEquityBuilder {
        self.price = Some(price);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsEquity, JsValue> {
        let ticker = self
            .ticker
            .as_deref()
            .ok_or_else(|| js_error("EquityBuilder: ticker is required".to_string()))?;
        let currency = self
            .currency
            .ok_or_else(|| js_error("EquityBuilder: currency is required".to_string()))?;

        let mut equity = Equity::new(self.instrument_id.to_string(), ticker, currency);
        if let Some(qty) = self.shares {
            equity = equity.with_shares(qty);
        }
        if let Some(px) = self.price {
            equity = equity.with_price(px);
        }
        Ok(JsEquity::from_inner(equity))
    }
}

#[wasm_bindgen(js_name = Equity)]
#[derive(Clone, Debug)]
pub struct JsEquity {
    pub(crate) inner: Equity,
}

impl InstrumentWrapper for JsEquity {
    type Inner = Equity;
    fn from_inner(inner: Equity) -> Self {
        JsEquity { inner }
    }
    fn inner(&self) -> Equity {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = Equity)]
impl JsEquity {
    /// Create an equity spot position.
    ///
    /// Conventions:
    /// - `shares` is the quantity of shares (defaults to 1.0 if omitted by the underlying instrument type).
    /// - `price` is an optional spot price override (absolute).
    ///
    /// @param instrument_id - Unique identifier
    /// @param ticker - Underlying ticker/symbol
    /// @param currency - Reporting currency of the equity
    /// @param shares - Optional share quantity
    /// @param price - Optional price override (absolute)
    /// @returns A new `Equity`
    ///
    /// @example
    /// ```javascript
    /// import init, { Equity, Currency } from "finstack-wasm";
    ///
    /// await init();
    /// const eq = new Equity("eq_1", "AAPL", new Currency("USD"), 100, 200.0);
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(
        instrument_id: &str,
        ticker: &str,
        currency: &JsCurrency,
        shares: Option<f64>,
        price: Option<f64>,
    ) -> JsEquity {
        web_sys::console::warn_1(&JsValue::from_str(
            "Equity constructor is deprecated; use EquityBuilder instead.",
        ));
        let mut equity = Equity::new(instrument_id.to_string(), ticker, currency.inner());
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

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsEquity, JsValue> {
        from_js_value(value).map(JsEquity::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::Equity as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "Equity(id='{}', ticker='{}', shares={})",
            self.inner.id,
            self.inner.ticker,
            self.shares()
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsEquity {
        JsEquity::from_inner(self.inner.clone())
    }
}
