use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::equity::equity_option::EquityOption;
use finstack_valuations::instruments::{
    Attributes, EquityUnderlyingParams, ExerciseStyle, OptionType, PricingOverrides, SettlementType,
};
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

fn build_equity_option(
    instrument_id: &str,
    ticker: &str,
    strike: f64,
    option_type: OptionType,
    expiry: finstack_core::dates::Date,
    notional_amount: f64,
) -> Result<EquityOption, JsValue> {
    // Match the prior convenience constructor conventions.
    let underlying = EquityUnderlyingParams::new(ticker, "EQUITY-SPOT", Currency::USD)
        .with_dividend_yield("EQUITY-DIVYIELD");

    EquityOption::builder()
        .id(InstrumentId::new(instrument_id))
        .underlying_ticker(underlying.ticker)
        .strike(strike)
        .option_type(option_type)
        .exercise_style(ExerciseStyle::European)
        .expiry(expiry)
        .notional(Money::new(notional_amount, underlying.currency))
        .day_count(finstack_core::dates::DayCount::Act365F)
        .settlement(SettlementType::Cash)
        .discount_curve_id(CurveId::new("USD-OIS"))
        .spot_id(underlying.spot_id)
        .vol_surface_id(CurveId::new("EQUITY-VOL"))
        .div_yield_id_opt(underlying.div_yield_id)
        .pricing_overrides(PricingOverrides::default())
        .attributes(Attributes::new())
        .build()
        .map_err(|e| js_error(e.to_string()))
}

#[wasm_bindgen(js_name = EquityOptionBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsEquityOptionBuilder {
    instrument_id: String,
    ticker: Option<String>,
    strike: Option<f64>,
    option_type: Option<String>,
    expiry: Option<finstack_core::dates::Date>,
    notional_amount: Option<f64>,
}

#[wasm_bindgen(js_class = EquityOptionBuilder)]
impl JsEquityOptionBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsEquityOptionBuilder {
        JsEquityOptionBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = ticker)]
    pub fn ticker(mut self, ticker: String) -> JsEquityOptionBuilder {
        self.ticker = Some(ticker);
        self
    }

    #[wasm_bindgen(js_name = strike)]
    pub fn strike(mut self, strike: f64) -> JsEquityOptionBuilder {
        self.strike = Some(strike);
        self
    }

    #[wasm_bindgen(js_name = optionType)]
    pub fn option_type(mut self, option_type: String) -> JsEquityOptionBuilder {
        self.option_type = Some(option_type);
        self
    }

    #[wasm_bindgen(js_name = expiry)]
    pub fn expiry(mut self, expiry: &JsDate) -> JsEquityOptionBuilder {
        self.expiry = Some(expiry.inner());
        self
    }

    #[wasm_bindgen(js_name = notionalAmount)]
    pub fn notional_amount(mut self, notional_amount: f64) -> JsEquityOptionBuilder {
        self.notional_amount = Some(notional_amount);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsEquityOption, JsValue> {
        let ticker = self
            .ticker
            .as_deref()
            .ok_or_else(|| js_error("EquityOptionBuilder: ticker is required".to_string()))?;
        let strike = self
            .strike
            .ok_or_else(|| js_error("EquityOptionBuilder: strike is required".to_string()))?;
        let option_type = self
            .option_type
            .as_deref()
            .ok_or_else(|| js_error("EquityOptionBuilder: optionType is required".to_string()))?;
        let expiry = self
            .expiry
            .ok_or_else(|| js_error("EquityOptionBuilder: expiry is required".to_string()))?;
        let notional_amount = self.notional_amount.unwrap_or(1.0);

        let option_type = match option_type.to_lowercase().as_str() {
            "call" => OptionType::Call,
            "put" => OptionType::Put,
            other => {
                return Err(js_error(format!(
                    "Invalid optionType '{other}'; expected 'call' or 'put'"
                )));
            }
        };

        build_equity_option(
            &self.instrument_id,
            ticker,
            strike,
            option_type,
            expiry,
            notional_amount,
        )
        .map(JsEquityOption::from_inner)
    }
}

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
    /// - `notional_amount` defaults to `1.0` if omitted.
    /// - `option_type`: `"call"` or `"put"`.
    ///
    /// @param instrument_id - Unique identifier
    /// @param ticker - Underlying ticker/symbol (used to look up spot/dividends/vol in `MarketContext`)
    /// @param strike - Strike price (absolute)
    /// @param option_type - `"call"` or `"put"`
    /// @param expiry - Expiry date
    /// @param notional_amount - Optional notional amount multiplier (default 1.0)
    /// @returns A new `EquityOption`
    /// @throws {Error} If `option_type` is invalid
    ///
    /// @example
    /// ```javascript
    /// import init, { EquityOption, FsDate } from "finstack-wasm";
    ///
    /// await init();
    /// const opt = new EquityOption(
    ///   "eqopt_1",
    ///   "AAPL",
    ///   200.0,
    ///   "call",
    ///   new FsDate(2025, 6, 21),
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
        notional_amount: Option<f64>,
    ) -> Result<JsEquityOption, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "EquityOption constructor is deprecated; use EquityOptionBuilder instead.",
        ));
        let notional_amount = notional_amount.unwrap_or(1.0);
        let option_type = match option_type.to_lowercase().as_str() {
            "call" => OptionType::Call,
            "put" => OptionType::Put,
            other => {
                return Err(js_error(format!(
                    "Invalid option_type '{other}'; expected 'call' or 'put'"
                )));
            }
        };

        build_equity_option(
            instrument_id,
            ticker,
            strike,
            option_type,
            expiry.inner(),
            notional_amount,
        )
        .map(JsEquityOption::from_inner)
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
    pub fn strike(&self) -> f64 {
        self.inner.strike
    }

    #[wasm_bindgen(getter)]
    pub fn expiry(&self) -> JsDate {
        JsDate::from_core(self.inner.expiry)
    }

    #[wasm_bindgen(getter, js_name = notionalAmount)]
    pub fn notional_amount(&self) -> f64 {
        self.inner.notional.amount()
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
