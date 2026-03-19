use crate::core::dates::date::JsDate;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::exotics::asian_option::{AsianOption, AveragingMethod};
use finstack_valuations::instruments::OptionType;
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

/// Averaging method for Asian options.
#[wasm_bindgen(js_name = AveragingMethod)]
#[derive(Clone, Copy, Debug)]
pub enum JsAveragingMethod {
    Arithmetic,
    Geometric,
}

impl From<AveragingMethod> for JsAveragingMethod {
    fn from(method: AveragingMethod) -> Self {
        match method {
            AveragingMethod::Arithmetic => JsAveragingMethod::Arithmetic,
            AveragingMethod::Geometric => JsAveragingMethod::Geometric,
        }
    }
}

impl From<JsAveragingMethod> for AveragingMethod {
    fn from(method: JsAveragingMethod) -> Self {
        match method {
            JsAveragingMethod::Arithmetic => AveragingMethod::Arithmetic,
            JsAveragingMethod::Geometric => AveragingMethod::Geometric,
        }
    }
}

#[wasm_bindgen(js_name = AsianOption)]
#[derive(Clone, Debug)]
pub struct JsAsianOption {
    pub(crate) inner: AsianOption,
}

impl InstrumentWrapper for JsAsianOption {
    type Inner = AsianOption;
    fn from_inner(inner: AsianOption) -> Self {
        JsAsianOption { inner }
    }
    fn inner(&self) -> AsianOption {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_name = AsianOptionBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsAsianOptionBuilder {
    instrument_id: String,
    ticker: Option<String>,
    strike: Option<f64>,
    expiry: Option<finstack_core::dates::Date>,
    fixing_dates: Option<Array>,
    notional: Option<finstack_core::money::Money>,
    discount_curve: Option<String>,
    spot_id: Option<String>,
    vol_surface: Option<String>,
    averaging_method: Option<String>,
    option_type: Option<String>,
    div_yield_id: Option<String>,
}

#[wasm_bindgen(js_class = AsianOptionBuilder)]
impl JsAsianOptionBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsAsianOptionBuilder {
        JsAsianOptionBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = ticker)]
    pub fn ticker(mut self, ticker: String) -> JsAsianOptionBuilder {
        self.ticker = Some(ticker);
        self
    }

    #[wasm_bindgen(js_name = strike)]
    pub fn strike(mut self, strike: f64) -> JsAsianOptionBuilder {
        self.strike = Some(strike);
        self
    }

    #[wasm_bindgen(js_name = expiry)]
    pub fn expiry(mut self, expiry: &JsDate) -> JsAsianOptionBuilder {
        self.expiry = Some(expiry.inner());
        self
    }

    #[wasm_bindgen(js_name = fixingDates)]
    pub fn fixing_dates(mut self, fixing_dates: Array) -> JsAsianOptionBuilder {
        self.fixing_dates = Some(fixing_dates);
        self
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsAsianOptionBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = discountCurve)]
    pub fn discount_curve(mut self, discount_curve: &str) -> JsAsianOptionBuilder {
        self.discount_curve = Some(discount_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = spotId)]
    pub fn spot_id(mut self, spot_id: &str) -> JsAsianOptionBuilder {
        self.spot_id = Some(spot_id.to_string());
        self
    }

    #[wasm_bindgen(js_name = volSurface)]
    pub fn vol_surface(mut self, vol_surface: &str) -> JsAsianOptionBuilder {
        self.vol_surface = Some(vol_surface.to_string());
        self
    }

    #[wasm_bindgen(js_name = averagingMethod)]
    pub fn averaging_method(mut self, averaging_method: String) -> JsAsianOptionBuilder {
        self.averaging_method = Some(averaging_method);
        self
    }

    #[wasm_bindgen(js_name = optionType)]
    pub fn option_type(mut self, option_type: String) -> JsAsianOptionBuilder {
        self.option_type = Some(option_type);
        self
    }

    #[wasm_bindgen(js_name = divYieldId)]
    pub fn div_yield_id(mut self, div_yield_id: String) -> JsAsianOptionBuilder {
        self.div_yield_id = Some(div_yield_id);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsAsianOption, JsValue> {
        use crate::core::error::js_error;
        use finstack_core::dates::DayCount;

        let ticker = self
            .ticker
            .as_deref()
            .ok_or_else(|| js_error("AsianOptionBuilder: ticker is required"))?;
        let strike = self
            .strike
            .ok_or_else(|| js_error("AsianOptionBuilder: strike is required"))?;
        let expiry = self
            .expiry
            .ok_or_else(|| js_error("AsianOptionBuilder: expiry is required"))?;
        let fixing_dates = self
            .fixing_dates
            .ok_or_else(|| js_error("AsianOptionBuilder: fixingDates is required"))?;
        let notional = self
            .notional
            .ok_or_else(|| js_error("AsianOptionBuilder: notional (money) is required"))?;
        let discount_curve = self
            .discount_curve
            .as_deref()
            .ok_or_else(|| js_error("AsianOptionBuilder: discountCurve is required"))?;
        let spot_id = self
            .spot_id
            .as_deref()
            .ok_or_else(|| js_error("AsianOptionBuilder: spotId is required"))?;
        let vol_surface = self
            .vol_surface
            .as_deref()
            .ok_or_else(|| js_error("AsianOptionBuilder: volSurface is required"))?;

        let mut fixing_dates_vec = Vec::new();
        for item in fixing_dates.iter() {
            let date_str = item
                .as_string()
                .ok_or_else(|| js_error("Fixing dates must be ISO date strings (YYYY-MM-DD)"))?;
            fixing_dates_vec.push(parse_iso_date(&date_str)?);
        }

        let avg_method = match self.averaging_method.as_deref() {
            None | Some("arithmetic") => AveragingMethod::Arithmetic,
            Some("geometric") => AveragingMethod::Geometric,
            Some(other) => return Err(js_error(format!("Unknown averaging method: {other}"))),
        };
        let opt_type = match self.option_type.as_deref() {
            None | Some("call") => OptionType::Call,
            Some("put") => OptionType::Put,
            Some(other) => return Err(js_error(format!("Unknown option type: {other}"))),
        };

        let mut builder = AsianOption::builder();
        builder = builder.id(instrument_id_from_str(&self.instrument_id));
        builder = builder.underlying_ticker(ticker.to_string());
        builder = builder.strike(strike);
        builder = builder.option_type(opt_type);
        builder = builder.averaging_method(avg_method);
        builder = builder.expiry(expiry);
        builder = builder.fixing_dates(fixing_dates_vec);
        builder = builder.notional(notional);
        builder = builder.day_count(DayCount::Act365F);
        builder = builder.discount_curve_id(curve_id_from_str(discount_curve));
        builder = builder.spot_id(spot_id.to_string().into());
        builder = builder.vol_surface_id(curve_id_from_str(vol_surface));
        builder = builder.pricing_overrides(Default::default());
        builder = builder.attributes(Default::default());
        if let Some(div) = self.div_yield_id {
            builder = builder.div_yield_id(curve_id_from_str(&div));
        }

        builder
            .build()
            .map(JsAsianOption::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }
}

#[wasm_bindgen(js_class = AsianOption)]
impl JsAsianOption {
    /// Create an Asian option (average price option) on an equity underlying.
    ///
    /// Conventions:
    /// - `strike` is an **absolute price level** (not percent/bps).
    /// - `fixing_dates` must be ISO strings (`"YYYY-MM-DD"`).
    /// - `option_type`: `"call"` or `"put"` (default `"call"`).
    /// - `averaging_method`: `"arithmetic"` or `"geometric"` (default `"arithmetic"`).
    ///
    /// @param instrument_id - Unique identifier
    /// @param ticker - Underlying ticker/symbol (used to look up spot/dividends/vol in `MarketContext`)
    /// @param strike - Strike price (absolute)
    /// @param expiry - Expiry date
    /// @param fixing_dates - Array of ISO date strings (`"YYYY-MM-DD"`)
    /// @param notional - Option notional (currency-tagged)
    /// @param discount_curve - Discount curve ID
    /// @param spot_id - Market scalar/price id for spot
    /// @param vol_surface - Vol surface ID
    /// @param averaging_method - Optional averaging method string
    /// @param option_type - Optional option type string
    /// @param div_yield_id - Optional dividend yield scalar/curve ID
    /// @returns A new `AsianOption`
    /// @throws {Error} If inputs are invalid (e.g., fixing dates not ISO)
    ///
    /// @example
    /// ```javascript
    /// import init, { AsianOption, Money, FsDate } from "finstack-wasm";
    ///
    /// await init();
    /// const opt = new AsianOption(
    ///   "asian_1",
    ///   "AAPL",
    ///   200.0,
    ///   new FsDate(2025, 6, 21),
    ///   ["2025-03-21", "2025-04-21", "2025-05-21"],
    ///   Money.fromCode(1_000_000, "USD"),
    ///   "USD-OIS",
    ///   "AAPL-SPOT",
    ///   "AAPL-VOL",
    ///   "arithmetic",
    ///   "call"
    /// );
    /// ```
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn builder(
        instrument_id: &str,
        ticker: &str,
        strike: f64,
        expiry: &JsDate,
        fixing_dates: Array,
        notional: &JsMoney,
        discount_curve: &str,
        spot_id: &str,
        vol_surface: &str,
        averaging_method: Option<String>,
        option_type: Option<String>,
        div_yield_id: Option<String>,
    ) -> Result<JsAsianOption, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "AsianOption constructor is deprecated; use AsianOptionBuilder instead.",
        ));
        use crate::core::error::js_error;
        use finstack_core::dates::DayCount;

        let mut fixing_dates_vec = Vec::new();
        for item in fixing_dates.iter() {
            let date_str = item
                .as_string()
                .ok_or_else(|| js_error("Fixing dates must be ISO date strings (YYYY-MM-DD)"))?;
            fixing_dates_vec.push(parse_iso_date(&date_str)?);
        }

        let avg_method = match averaging_method.as_deref() {
            None | Some("arithmetic") => AveragingMethod::Arithmetic,
            Some("geometric") => AveragingMethod::Geometric,
            Some(other) => {
                return Err(js_error(format!("Unknown averaging method: {other}")));
            }
        };

        let opt_type = match option_type.as_deref() {
            None | Some("call") => OptionType::Call,
            Some("put") => OptionType::Put,
            Some(other) => {
                return Err(js_error(format!("Unknown option type: {other}")));
            }
        };

        let mut builder = AsianOption::builder();
        builder = builder.id(instrument_id_from_str(instrument_id));
        builder = builder.underlying_ticker(ticker.to_string());
        builder = builder.strike(strike);
        builder = builder.option_type(opt_type);
        builder = builder.averaging_method(avg_method);
        builder = builder.expiry(expiry.inner());
        builder = builder.fixing_dates(fixing_dates_vec);
        builder = builder.notional(notional.inner());
        builder = builder.day_count(DayCount::Act365F);
        builder = builder.discount_curve_id(curve_id_from_str(discount_curve));
        builder = builder.spot_id(spot_id.to_string().into());
        builder = builder.vol_surface_id(curve_id_from_str(vol_surface));
        builder = builder.pricing_overrides(Default::default());
        builder = builder.attributes(Default::default());
        if let Some(div) = div_yield_id {
            builder = builder.div_yield_id(curve_id_from_str(&div));
        }

        builder
            .build()
            .map(JsAsianOption::from_inner)
            .map_err(|e| js_error(e.to_string()))
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

    #[wasm_bindgen(getter, js_name = optionType)]
    pub fn option_type(&self) -> String {
        match self.inner.option_type {
            OptionType::Call => "call",
            OptionType::Put => "put",
        }
        .to_string()
    }

    #[wasm_bindgen(getter, js_name = averagingMethod)]
    pub fn averaging_method(&self) -> JsAveragingMethod {
        self.inner.averaging_method.into()
    }

    #[wasm_bindgen(getter)]
    pub fn expiry(&self) -> JsDate {
        JsDate::from_core(self.inner.expiry)
    }

    #[wasm_bindgen(getter, js_name = fixingDates)]
    pub fn fixing_dates(&self) -> Array {
        let arr = Array::new();
        for date in &self.inner.fixing_dates {
            arr.push(&JsDate::from_core(*date).into());
        }
        arr
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(getter, js_name = discountCurve)]
    pub fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = spotId)]
    pub fn spot_id(&self) -> String {
        self.inner.spot_id.to_string()
    }

    #[wasm_bindgen(getter, js_name = volSurface)]
    pub fn vol_surface(&self) -> String {
        self.inner.vol_surface_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = dividendYieldId)]
    pub fn div_yield_id(&self) -> Option<String> {
        self.inner
            .div_yield_id
            .as_ref()
            .map(|id| id.as_str().to_string())
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsAsianOption, JsValue> {
        from_js_value(value).map(JsAsianOption::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> String {
        InstrumentType::AsianOption.to_string()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "AsianOption(id='{}', ticker='{}', strike={}, expiry={}, averaging_method='{:?}')",
            self.inner.id.as_str(),
            self.inner.underlying_ticker,
            self.inner.strike,
            self.inner.expiry,
            self.inner.averaging_method
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsAsianOption {
        JsAsianOption::from_inner(self.inner.clone())
    }
}

fn parse_iso_date(value: &str) -> Result<finstack_core::dates::Date, JsValue> {
    use crate::core::error::js_error;
    use time::Month;

    let parts: Vec<&str> = value.split('-').collect();
    if parts.len() != 3 {
        return Err(js_error(format!(
            "Date '{value}' must be in ISO format YYYY-MM-DD"
        )));
    }
    let year: i32 = parts[0]
        .parse()
        .map_err(|_| js_error(format!("Invalid year component in date '{value}'")))?;
    let month: u8 = parts[1]
        .parse()
        .map_err(|_| js_error(format!("Invalid month component in date '{value}'")))?;
    let day: u8 = parts[2]
        .parse()
        .map_err(|_| js_error(format!("Invalid day component in date '{value}'")))?;
    let month_enum = Month::try_from(month)
        .map_err(|_| js_error(format!("Month component must be 1-12 in date '{value}'")))?;
    finstack_core::dates::Date::from_calendar_date(year, month_enum, day)
        .map_err(|e| js_error(format!("Invalid date '{value}': {e}")))
}
