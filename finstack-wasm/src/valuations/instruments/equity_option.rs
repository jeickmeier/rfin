use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::equity::equity_option::EquityOption;
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
    /// Create a European equity option.
    ///
    /// Conventions:
    /// - `strike` is an **absolute price level** (not bps/percent).
    /// - The strike currency is assumed to be the same currency as `notional`.
    /// - `contract_size` defaults to `1.0` if omitted.
    /// - `option_type`: `"call"` or `"put"`.
    ///
    /// @param instrument_id - Unique identifier
    /// @param ticker - Underlying ticker/symbol (used to look up spot/dividends/vol in `MarketContext`)
    /// @param strike - Strike price (absolute)
    /// @param option_type - `"call"` or `"put"`
    /// @param expiry - Expiry date
    /// @param notional - Option notional (currency-tagged)
    /// @param contract_size - Optional contract size multiplier (default 1.0)
    /// @returns A new `EquityOption`
    /// @throws {Error} If `option_type` is invalid
    ///
    /// @example
    /// ```javascript
    /// import init, { EquityOption, Money, FsDate } from "finstack-wasm";
    ///
    /// await init();
    /// const opt = new EquityOption(
    ///   "eqopt_1",
    ///   "AAPL",
    ///   200.0,
    ///   "call",
    ///   new FsDate(2025, 6, 21),
    ///   Money.fromCode(1_000_000, "USD"),
    ///   100
    /// );
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(
        instrument_id: &str,
        ticker: &str,
        strike: f64,
        option_type: &str,
        expiry: &JsDate,
        notional: &JsMoney,
        contract_size: Option<f64>,
    ) -> Result<JsEquityOption, JsValue> {
        let cs = contract_size.unwrap_or(1.0);
        let option = match option_type.to_lowercase().as_str() {
            "call" => EquityOption::european_call(
                instrument_id.to_string(),
                ticker,
                strike,
                expiry.inner(),
                notional.inner(),
                cs,
            ),
            "put" => EquityOption::european_put(
                instrument_id.to_string(),
                ticker,
                strike,
                expiry.inner(),
                notional.inner(),
                cs,
            ),
            other => {
                return Err(js_error(format!(
                    "Invalid option_type '{other}'; expected 'call' or 'put'"
                )));
            }
        }
        .map(JsEquityOption::from_inner)
        .map_err(|e| js_error(e.to_string()))?;

        Ok(option)
    }

    /// Parse an equity option from a JSON value (as produced by `toJson`).
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsEquityOption, JsValue> {
        from_js_value(value).map(JsEquityOption::from_inner)
    }

    /// Serialize this equity option to a JSON value.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
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
