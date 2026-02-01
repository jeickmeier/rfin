use crate::core::dates::date::JsDate;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::{
    curve_id_from_str, instrument_id_from_str, parameters::JsBarrierType,
};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::exotics::barrier_option::{
    BarrierOption, BarrierType as BarrierOptionType,
};
use finstack_valuations::instruments::OptionType;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = BarrierOptionBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsBarrierOptionBuilder {
    instrument_id: String,
    ticker: Option<String>,
    strike: Option<f64>,
    barrier: Option<f64>,
    option_type: Option<String>,
    barrier_type: Option<String>,
    expiry: Option<finstack_core::dates::Date>,
    notional: Option<finstack_core::money::Money>,
    discount_curve: Option<String>,
    spot_id: Option<String>,
    vol_surface: Option<String>,
    div_yield_id: Option<String>,
    use_gobet_miri: Option<bool>,
}

#[wasm_bindgen(js_class = BarrierOptionBuilder)]
impl JsBarrierOptionBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsBarrierOptionBuilder {
        JsBarrierOptionBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = ticker)]
    pub fn ticker(mut self, ticker: String) -> JsBarrierOptionBuilder {
        self.ticker = Some(ticker);
        self
    }

    #[wasm_bindgen(js_name = strike)]
    pub fn strike(mut self, strike: f64) -> JsBarrierOptionBuilder {
        self.strike = Some(strike);
        self
    }

    #[wasm_bindgen(js_name = barrier)]
    pub fn barrier(mut self, barrier: f64) -> JsBarrierOptionBuilder {
        self.barrier = Some(barrier);
        self
    }

    #[wasm_bindgen(js_name = optionType)]
    pub fn option_type(mut self, option_type: String) -> JsBarrierOptionBuilder {
        self.option_type = Some(option_type);
        self
    }

    #[wasm_bindgen(js_name = barrierType)]
    pub fn barrier_type(mut self, barrier_type: String) -> JsBarrierOptionBuilder {
        self.barrier_type = Some(barrier_type);
        self
    }

    #[wasm_bindgen(js_name = expiry)]
    pub fn expiry(mut self, expiry: &JsDate) -> JsBarrierOptionBuilder {
        self.expiry = Some(expiry.inner());
        self
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsBarrierOptionBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = discountCurve)]
    pub fn discount_curve(mut self, discount_curve: &str) -> JsBarrierOptionBuilder {
        self.discount_curve = Some(discount_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = spotId)]
    pub fn spot_id(mut self, spot_id: &str) -> JsBarrierOptionBuilder {
        self.spot_id = Some(spot_id.to_string());
        self
    }

    #[wasm_bindgen(js_name = volSurface)]
    pub fn vol_surface(mut self, vol_surface: &str) -> JsBarrierOptionBuilder {
        self.vol_surface = Some(vol_surface.to_string());
        self
    }

    #[wasm_bindgen(js_name = divYieldId)]
    pub fn div_yield_id(mut self, div_yield_id: String) -> JsBarrierOptionBuilder {
        self.div_yield_id = Some(div_yield_id);
        self
    }

    #[wasm_bindgen(js_name = useGobetMiri)]
    pub fn use_gobet_miri(mut self, use_gobet_miri: bool) -> JsBarrierOptionBuilder {
        self.use_gobet_miri = Some(use_gobet_miri);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsBarrierOption, JsValue> {
        use crate::core::error::js_error;
        use finstack_core::dates::DayCount;

        let ticker = self
            .ticker
            .as_deref()
            .ok_or_else(|| js_error("BarrierOptionBuilder: ticker is required"))?;
        let strike = self
            .strike
            .ok_or_else(|| js_error("BarrierOptionBuilder: strike is required"))?;
        let barrier = self
            .barrier
            .ok_or_else(|| js_error("BarrierOptionBuilder: barrier is required"))?;
        let option_type = self
            .option_type
            .as_deref()
            .ok_or_else(|| js_error("BarrierOptionBuilder: optionType is required"))?;
        let barrier_type = self
            .barrier_type
            .as_deref()
            .ok_or_else(|| js_error("BarrierOptionBuilder: barrierType is required"))?;
        let expiry = self
            .expiry
            .ok_or_else(|| js_error("BarrierOptionBuilder: expiry is required"))?;
        let notional = self
            .notional
            .ok_or_else(|| js_error("BarrierOptionBuilder: notional (money) is required"))?;
        let discount_curve = self
            .discount_curve
            .as_deref()
            .ok_or_else(|| js_error("BarrierOptionBuilder: discountCurve is required"))?;
        let spot_id = self
            .spot_id
            .as_deref()
            .ok_or_else(|| js_error("BarrierOptionBuilder: spotId is required"))?;
        let vol_surface = self
            .vol_surface
            .as_deref()
            .ok_or_else(|| js_error("BarrierOptionBuilder: volSurface is required"))?;

        let opt_type = match option_type.to_lowercase().as_str() {
            "call" => OptionType::Call,
            "put" => OptionType::Put,
            other => return Err(js_error(format!("Unknown option type: {other}"))),
        };

        let barrier_type_enum = match barrier_type.to_lowercase().replace('_', "").as_str() {
            "upandout" => BarrierOptionType::UpAndOut,
            "upandin" => BarrierOptionType::UpAndIn,
            "downandout" => BarrierOptionType::DownAndOut,
            "downandin" => BarrierOptionType::DownAndIn,
            other => return Err(js_error(format!("Unknown barrier type: {other}"))),
        };

        let strike_money = finstack_core::money::Money::new(strike, notional.currency());
        let barrier_money = finstack_core::money::Money::new(barrier, notional.currency());

        let mut builder = BarrierOption::builder();
        builder = builder.id(instrument_id_from_str(&self.instrument_id));
        builder = builder.underlying_ticker(ticker.to_string());
        builder = builder.strike(strike_money);
        builder = builder.barrier(barrier_money);
        builder = builder.option_type(opt_type);
        builder = builder.barrier_type(barrier_type_enum);
        builder = builder.expiry(expiry);
        builder = builder.notional(notional);
        builder = builder.day_count(DayCount::Act365F);
        builder = builder.use_gobet_miri(self.use_gobet_miri.unwrap_or(false));
        builder = builder.discount_curve_id(curve_id_from_str(discount_curve));
        builder = builder.spot_id(spot_id.to_string());
        builder = builder.vol_surface_id(curve_id_from_str(vol_surface));
        if let Some(div) = self.div_yield_id {
            builder = builder.div_yield_id(curve_id_from_str(&div));
        }
        builder = builder
            .pricing_overrides(finstack_valuations::instruments::PricingOverrides::default());
        builder = builder.attributes(finstack_valuations::instruments::Attributes::new());

        builder
            .build()
            .map(JsBarrierOption::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }
}

#[wasm_bindgen(js_name = BarrierOption)]
#[derive(Clone, Debug)]
pub struct JsBarrierOption {
    pub(crate) inner: BarrierOption,
}

impl InstrumentWrapper for JsBarrierOption {
    type Inner = BarrierOption;
    fn from_inner(inner: BarrierOption) -> Self {
        JsBarrierOption { inner }
    }
    fn inner(&self) -> BarrierOption {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = BarrierOption)]
impl JsBarrierOption {
    /// Create a barrier option on an equity underlying.
    ///
    /// Conventions:
    /// - `strike` and `barrier` are **absolute price levels**.
    /// - `option_type`: `"call"` or `"put"`.
    /// - `barrier_type`: `"UpAndOut"`, `"UpAndIn"`, `"DownAndOut"`, `"DownAndIn"` (case/underscore-insensitive).
    ///
    /// @param instrument_id - Unique identifier
    /// @param ticker - Underlying ticker/symbol
    /// @param strike - Strike price (absolute)
    /// @param barrier - Barrier price (absolute)
    /// @param option_type - `"call"` or `"put"`
    /// @param barrier_type - Barrier type string
    /// @param expiry - Expiry date
    /// @param notional - Option notional (currency-tagged)
    /// @param discount_curve - Discount curve ID
    /// @param spot_id - Market scalar/price id for spot
    /// @param vol_surface - Vol surface ID
    /// @param div_yield_id - Optional dividend yield id
    /// @param use_gobet_miri - Optional numerical method toggle
    /// @returns A new `BarrierOption`
    /// @throws {Error} If option/barrier type strings are invalid
    ///
    /// @example
    /// ```javascript
    /// import init, { BarrierOption, Money, FsDate } from "finstack-wasm";
    ///
    /// await init();
    /// const opt = new BarrierOption(
    ///   "barrier_1",
    ///   "AAPL",
    ///   200.0,
    ///   250.0,
    ///   "call",
    ///   "up_and_out",
    ///   new FsDate(2025, 6, 21),
    ///   Money.fromCode(1_000_000, "USD"),
    ///   "USD-OIS",
    ///   "AAPL-SPOT",
    ///   "AAPL-VOL"
    /// );
    /// ```
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        ticker: &str,
        strike: f64,
        barrier: f64,
        option_type: &str,
        barrier_type: &str,
        expiry: &JsDate,
        notional: &JsMoney,
        discount_curve: &str,
        spot_id: &str,
        vol_surface: &str,
        div_yield_id: Option<String>,
        use_gobet_miri: Option<bool>,
    ) -> Result<JsBarrierOption, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "BarrierOption constructor is deprecated; use BarrierOptionBuilder instead.",
        ));
        use crate::core::error::js_error;
        use finstack_core::dates::DayCount;

        let opt_type = match option_type.to_lowercase().as_str() {
            "call" => OptionType::Call,
            "put" => OptionType::Put,
            other => {
                return Err(js_error(format!("Unknown option type: {other}")));
            }
        };

        let barrier_type_enum = match barrier_type.to_lowercase().replace('_', "").as_str() {
            "upandout" => BarrierOptionType::UpAndOut,
            "upandin" => BarrierOptionType::UpAndIn,
            "downandout" => BarrierOptionType::DownAndOut,
            "downandin" => BarrierOptionType::DownAndIn,
            other => {
                return Err(js_error(format!("Unknown barrier type: {other}")));
            }
        };

        let strike_money = finstack_core::money::Money::new(strike, notional.inner().currency());
        let barrier_money = finstack_core::money::Money::new(barrier, notional.inner().currency());

        let mut builder = BarrierOption::builder();
        builder = builder.id(instrument_id_from_str(instrument_id));
        builder = builder.underlying_ticker(ticker.to_string());
        builder = builder.strike(strike_money);
        builder = builder.barrier(barrier_money);
        builder = builder.option_type(opt_type);
        builder = builder.barrier_type(barrier_type_enum);
        builder = builder.expiry(expiry.inner());
        builder = builder.notional(notional.inner());
        builder = builder.day_count(DayCount::Act365F);
        builder = builder.use_gobet_miri(use_gobet_miri.unwrap_or(false));
        builder = builder.discount_curve_id(curve_id_from_str(discount_curve));
        builder = builder.spot_id(spot_id.to_string());
        builder = builder.vol_surface_id(curve_id_from_str(vol_surface));
        if let Some(div) = div_yield_id {
            builder = builder.div_yield_id(curve_id_from_str(&div));
        }
        builder = builder
            .pricing_overrides(finstack_valuations::instruments::PricingOverrides::default());
        builder = builder.attributes(finstack_valuations::instruments::Attributes::new());

        builder
            .build()
            .map(JsBarrierOption::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsBarrierOption, JsValue> {
        from_js_value(value).map(JsBarrierOption::from_inner)
    }

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
    pub fn barrier(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.barrier)
    }

    #[wasm_bindgen(getter, js_name = optionType)]
    pub fn option_type(&self) -> String {
        match self.inner.option_type {
            OptionType::Call => "call",
            OptionType::Put => "put",
        }
        .to_string()
    }

    #[wasm_bindgen(getter, js_name = barrierType)]
    pub fn barrier_type(&self) -> JsBarrierType {
        JsBarrierType::from_inner(self.inner.barrier_type)
    }

    #[wasm_bindgen(getter)]
    pub fn expiry(&self) -> JsDate {
        JsDate::from_core(self.inner.expiry)
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
        self.inner.spot_id.clone()
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

    #[wasm_bindgen(getter, js_name = useGobetMiri)]
    pub fn use_gobet_miri(&self) -> bool {
        self.inner.use_gobet_miri
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::BarrierOption as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "BarrierOption(id='{}', ticker='{}', strike={}, barrier={}, barrier_type='{:?}')",
            self.inner.id.as_str(),
            self.inner.underlying_ticker,
            self.inner.strike.amount(),
            self.inner.barrier.amount(),
            self.inner.barrier_type
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsBarrierOption {
        JsBarrierOption::from_inner(self.inner.clone())
    }
}
