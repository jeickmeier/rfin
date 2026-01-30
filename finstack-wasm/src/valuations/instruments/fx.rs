use crate::core::currency::JsCurrency;
use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::fx::fx_option::{FxOption, FxOptionParams};
use finstack_valuations::instruments::fx::fx_spot::FxSpot;
use finstack_valuations::instruments::fx::fx_swap::FxSwap;
use finstack_valuations::instruments::OptionType;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

// ===========================
// FxSpot
// ===========================

#[wasm_bindgen(js_name = FxSpotBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsFxSpotBuilder {
    instrument_id: String,
    base_currency: Option<finstack_core::currency::Currency>,
    quote_currency: Option<finstack_core::currency::Currency>,
    settlement: Option<finstack_core::dates::Date>,
    spot_rate: Option<f64>,
    notional: Option<finstack_core::money::Money>,
}

#[wasm_bindgen(js_class = FxSpotBuilder)]
impl JsFxSpotBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsFxSpotBuilder {
        JsFxSpotBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = baseCurrency)]
    pub fn base_currency(mut self, base_currency: &JsCurrency) -> JsFxSpotBuilder {
        self.base_currency = Some(base_currency.inner());
        self
    }

    #[wasm_bindgen(js_name = quoteCurrency)]
    pub fn quote_currency(mut self, quote_currency: &JsCurrency) -> JsFxSpotBuilder {
        self.quote_currency = Some(quote_currency.inner());
        self
    }

    #[wasm_bindgen(js_name = settlement)]
    pub fn settlement(mut self, settlement: JsDate) -> JsFxSpotBuilder {
        self.settlement = Some(settlement.inner());
        self
    }

    #[wasm_bindgen(js_name = spotRate)]
    pub fn spot_rate(mut self, spot_rate: f64) -> JsFxSpotBuilder {
        self.spot_rate = Some(spot_rate);
        self
    }

    #[wasm_bindgen(js_name = notional)]
    pub fn notional(mut self, notional: JsMoney) -> JsFxSpotBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsFxSpot, JsValue> {
        let base = self
            .base_currency
            .ok_or_else(|| js_error("FxSpotBuilder: baseCurrency is required".to_string()))?;
        let quote = self
            .quote_currency
            .ok_or_else(|| js_error("FxSpotBuilder: quoteCurrency is required".to_string()))?;

        let mut inst = FxSpot::new(instrument_id_from_str(&self.instrument_id), base, quote);

        if let Some(date) = self.settlement {
            inst = inst.with_settlement(date);
        }
        if let Some(rate) = self.spot_rate {
            inst = inst.with_rate(rate).map_err(|e| js_error(e.to_string()))?;
        }
        if let Some(money) = self.notional {
            inst = inst
                .with_notional(money)
                .map_err(|e| js_error(e.to_string()))?;
        }

        Ok(JsFxSpot::from_inner(inst))
    }
}

#[wasm_bindgen(js_name = FxSpot)]
#[derive(Clone, Debug)]
pub struct JsFxSpot {
    pub(crate) inner: FxSpot,
}

impl InstrumentWrapper for JsFxSpot {
    type Inner = FxSpot;
    fn from_inner(inner: FxSpot) -> Self {
        JsFxSpot { inner }
    }
    fn inner(&self) -> FxSpot {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = FxSpot)]
impl JsFxSpot {
    /// Create an FX spot instrument.
    ///
    /// Conventions:
    /// - `spot_rate` is quoted as `quote_currency` per 1 unit of `base_currency` (e.g. EURUSD = USD per EUR).
    /// - If `notional` is provided, it represents the base-currency amount to exchange.
    ///
    /// @param instrument_id - Unique identifier
    /// @param base_currency - Base (foreign) currency
    /// @param quote_currency - Quote (domestic) currency
    /// @param settlement - Optional settlement date
    /// @param spot_rate - Optional spot rate override
    /// @param notional - Optional notional (currency-tagged; should be in base currency)
    /// @returns A new `FxSpot`
    /// @throws {Error} If notional currency is inconsistent
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        base_currency: &JsCurrency,
        quote_currency: &JsCurrency,
        settlement: Option<JsDate>,
        spot_rate: Option<f64>,
        notional: Option<JsMoney>,
    ) -> Result<JsFxSpot, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "FxSpot constructor is deprecated; use FxSpotBuilder instead.",
        ));
        let mut inst = FxSpot::new(
            instrument_id_from_str(instrument_id),
            base_currency.inner(),
            quote_currency.inner(),
        );

        if let Some(date) = settlement {
            inst = inst.with_settlement(date.inner());
        }
        if let Some(rate) = spot_rate {
            inst = inst.with_rate(rate).map_err(|e| js_error(e.to_string()))?;
        }
        if let Some(money) = notional {
            inst = inst
                .with_notional(money.inner())
                .map_err(|e| js_error(e.to_string()))?;
        }

        Ok(JsFxSpot::from_inner(inst))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = baseCurrency)]
    pub fn base_currency(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.base)
    }

    #[wasm_bindgen(getter, js_name = quoteCurrency)]
    pub fn quote_currency(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.quote)
    }

    #[wasm_bindgen(getter, js_name = pairName)]
    pub fn pair_name(&self) -> String {
        self.inner.pair_name()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::FxSpot as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "FxSpot(id='{}', pair='{}')",
            self.inner.id,
            self.inner.pair_name()
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsFxSpot {
        JsFxSpot::from_inner(self.inner.clone())
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsFxSpot, JsValue> {
        from_js_value(value).map(JsFxSpot::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

// ===========================
// FxOption
// ===========================

#[wasm_bindgen(js_name = FxOptionBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsFxOptionBuilder {
    instrument_id: String,
    base_currency: Option<finstack_core::currency::Currency>,
    quote_currency: Option<finstack_core::currency::Currency>,
    strike: Option<f64>,
    option_type: Option<String>,
    expiry: Option<finstack_core::dates::Date>,
    notional: Option<finstack_core::money::Money>,
    domestic_curve: Option<String>,
    foreign_curve: Option<String>,
    vol_surface: Option<String>,
}

#[wasm_bindgen(js_class = FxOptionBuilder)]
impl JsFxOptionBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsFxOptionBuilder {
        JsFxOptionBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = baseCurrency)]
    pub fn base_currency(mut self, base_currency: &JsCurrency) -> JsFxOptionBuilder {
        self.base_currency = Some(base_currency.inner());
        self
    }

    #[wasm_bindgen(js_name = quoteCurrency)]
    pub fn quote_currency(mut self, quote_currency: &JsCurrency) -> JsFxOptionBuilder {
        self.quote_currency = Some(quote_currency.inner());
        self
    }

    #[wasm_bindgen(js_name = strike)]
    pub fn strike(mut self, strike: f64) -> JsFxOptionBuilder {
        self.strike = Some(strike);
        self
    }

    #[wasm_bindgen(js_name = optionType)]
    pub fn option_type(mut self, option_type: String) -> JsFxOptionBuilder {
        self.option_type = Some(option_type);
        self
    }

    #[wasm_bindgen(js_name = expiry)]
    pub fn expiry(mut self, expiry: &JsDate) -> JsFxOptionBuilder {
        self.expiry = Some(expiry.inner());
        self
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsFxOptionBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = domesticCurve)]
    pub fn domestic_curve(mut self, domestic_curve: &str) -> JsFxOptionBuilder {
        self.domestic_curve = Some(domestic_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = foreignCurve)]
    pub fn foreign_curve(mut self, foreign_curve: &str) -> JsFxOptionBuilder {
        self.foreign_curve = Some(foreign_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = volSurface)]
    pub fn vol_surface(mut self, vol_surface: &str) -> JsFxOptionBuilder {
        self.vol_surface = Some(vol_surface.to_string());
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsFxOption, JsValue> {
        use finstack_valuations::instruments::FxUnderlyingParams;

        let base = self
            .base_currency
            .ok_or_else(|| js_error("FxOptionBuilder: baseCurrency is required".to_string()))?;
        let quote = self
            .quote_currency
            .ok_or_else(|| js_error("FxOptionBuilder: quoteCurrency is required".to_string()))?;
        let strike = self
            .strike
            .ok_or_else(|| js_error("FxOptionBuilder: strike is required".to_string()))?;
        let option_type = self
            .option_type
            .as_deref()
            .ok_or_else(|| js_error("FxOptionBuilder: optionType is required".to_string()))?;
        let expiry = self
            .expiry
            .ok_or_else(|| js_error("FxOptionBuilder: expiry is required".to_string()))?;
        let notional = self
            .notional
            .ok_or_else(|| js_error("FxOptionBuilder: notional (money) is required".to_string()))?;
        let domestic_curve = self
            .domestic_curve
            .as_deref()
            .ok_or_else(|| js_error("FxOptionBuilder: domesticCurve is required".to_string()))?;
        let foreign_curve = self
            .foreign_curve
            .as_deref()
            .ok_or_else(|| js_error("FxOptionBuilder: foreignCurve is required".to_string()))?;
        let vol_surface = self
            .vol_surface
            .as_deref()
            .ok_or_else(|| js_error("FxOptionBuilder: volSurface is required".to_string()))?;

        let option_params = match option_type.to_lowercase().as_str() {
            "call" => FxOptionParams::new(strike, expiry, OptionType::Call, notional),
            "put" => FxOptionParams::new(strike, expiry, OptionType::Put, notional),
            other => {
                return Err(js_error(format!(
                    "Invalid optionType '{other}'; expected 'call' or 'put'"
                )));
            }
        };
        let underlying = FxUnderlyingParams::new(
            base,
            quote,
            curve_id_from_str(domestic_curve),
            curve_id_from_str(foreign_curve),
        );
        let option = FxOption::new(
            instrument_id_from_str(&self.instrument_id),
            &option_params,
            &underlying,
            curve_id_from_str(vol_surface),
        );
        Ok(JsFxOption::from_inner(option))
    }
}

#[wasm_bindgen(js_name = FxOption)]
#[derive(Clone, Debug)]
pub struct JsFxOption {
    pub(crate) inner: FxOption,
}

impl InstrumentWrapper for JsFxOption {
    type Inner = FxOption;
    fn from_inner(inner: FxOption) -> Self {
        JsFxOption { inner }
    }
    fn inner(&self) -> FxOption {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = FxOption)]
impl JsFxOption {
    /// Create a European FX option.
    ///
    /// Conventions:
    /// - `strike` is quoted as `quote_currency` per 1 unit of `base_currency`.
    /// - `option_type`: `"call"` or `"put"`.
    ///
    /// @param instrument_id - Unique identifier
    /// @param base_currency - Base (foreign) currency
    /// @param quote_currency - Quote (domestic) currency
    /// @param strike - Strike FX rate
    /// @param option_type - `"call"` or `"put"`
    /// @param expiry - Expiry date
    /// @param notional - Notional (currency-tagged; typically in base currency)
    /// @param domestic_curve - Domestic (quote) discount curve ID
    /// @param foreign_curve - Foreign (base) discount curve ID
    /// @param vol_surface - Vol surface ID
    /// @returns A new `FxOption`
    #[allow(clippy::too_many_arguments)]
    #[wasm_bindgen(constructor)]
    pub fn new(
        instrument_id: &str,
        base_currency: &JsCurrency,
        quote_currency: &JsCurrency,
        strike: f64,
        option_type: &str,
        expiry: &JsDate,
        notional: &JsMoney,
        domestic_curve: &str,
        foreign_curve: &str,
        vol_surface: &str,
    ) -> Result<JsFxOption, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "FxOption constructor is deprecated; use FxOptionBuilder instead.",
        ));
        use finstack_valuations::instruments::FxUnderlyingParams;
        let option_params = match option_type.to_lowercase().as_str() {
            "call" => {
                FxOptionParams::new(strike, expiry.inner(), OptionType::Call, notional.inner())
            }
            "put" => FxOptionParams::new(strike, expiry.inner(), OptionType::Put, notional.inner()),
            other => {
                return Err(js_error(format!(
                    "Invalid option_type '{other}'; expected 'call' or 'put'"
                )));
            }
        };
        let underlying = FxUnderlyingParams::new(
            base_currency.inner(),
            quote_currency.inner(),
            curve_id_from_str(domestic_curve),
            curve_id_from_str(foreign_curve),
        );
        let option = FxOption::new(
            instrument_id_from_str(instrument_id),
            &option_params,
            &underlying,
            curve_id_from_str(vol_surface),
        );
        Ok(JsFxOption::from_inner(option))
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsFxOption, JsValue> {
        from_js_value(value).map(JsFxOption::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = baseCurrency)]
    pub fn base_currency(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.base_currency)
    }

    #[wasm_bindgen(getter, js_name = quoteCurrency)]
    pub fn quote_currency(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.quote_currency)
    }

    #[wasm_bindgen(getter)]
    pub fn strike(&self) -> f64 {
        self.inner.strike
    }

    #[wasm_bindgen(getter)]
    pub fn expiry(&self) -> JsDate {
        JsDate::from_core(self.inner.expiry)
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::FxOption as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "FxOption(id='{}', strike={:.4})",
            self.inner.id, self.inner.strike
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsFxOption {
        JsFxOption::from_inner(self.inner.clone())
    }
}

// ===========================
// FxSwap
// ===========================

#[wasm_bindgen(js_name = FxSwapBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsFxSwapBuilder {
    instrument_id: String,
    base_currency: Option<finstack_core::currency::Currency>,
    quote_currency: Option<finstack_core::currency::Currency>,
    notional: Option<finstack_core::money::Money>,
    near_date: Option<finstack_core::dates::Date>,
    far_date: Option<finstack_core::dates::Date>,
    domestic_curve: Option<String>,
    foreign_curve: Option<String>,
    near_rate: Option<f64>,
    far_rate: Option<f64>,
}

#[wasm_bindgen(js_class = FxSwapBuilder)]
impl JsFxSwapBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsFxSwapBuilder {
        JsFxSwapBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = baseCurrency)]
    pub fn base_currency(mut self, base_currency: &JsCurrency) -> JsFxSwapBuilder {
        self.base_currency = Some(base_currency.inner());
        self
    }

    #[wasm_bindgen(js_name = quoteCurrency)]
    pub fn quote_currency(mut self, quote_currency: &JsCurrency) -> JsFxSwapBuilder {
        self.quote_currency = Some(quote_currency.inner());
        self
    }

    #[wasm_bindgen(js_name = notional)]
    pub fn notional(mut self, notional: &JsMoney) -> JsFxSwapBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = nearDate)]
    pub fn near_date(mut self, near_date: &JsDate) -> JsFxSwapBuilder {
        self.near_date = Some(near_date.inner());
        self
    }

    #[wasm_bindgen(js_name = farDate)]
    pub fn far_date(mut self, far_date: &JsDate) -> JsFxSwapBuilder {
        self.far_date = Some(far_date.inner());
        self
    }

    #[wasm_bindgen(js_name = domesticCurve)]
    pub fn domestic_curve(mut self, domestic_curve: &str) -> JsFxSwapBuilder {
        self.domestic_curve = Some(domestic_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = foreignCurve)]
    pub fn foreign_curve(mut self, foreign_curve: &str) -> JsFxSwapBuilder {
        self.foreign_curve = Some(foreign_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = nearRate)]
    pub fn near_rate(mut self, near_rate: f64) -> JsFxSwapBuilder {
        self.near_rate = Some(near_rate);
        self
    }

    #[wasm_bindgen(js_name = farRate)]
    pub fn far_rate(mut self, far_rate: f64) -> JsFxSwapBuilder {
        self.far_rate = Some(far_rate);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsFxSwap, JsValue> {
        let base = self
            .base_currency
            .ok_or_else(|| js_error("FxSwapBuilder: baseCurrency is required".to_string()))?;
        let quote = self
            .quote_currency
            .ok_or_else(|| js_error("FxSwapBuilder: quoteCurrency is required".to_string()))?;
        let notional = self
            .notional
            .ok_or_else(|| js_error("FxSwapBuilder: notional is required".to_string()))?;
        let near_date = self
            .near_date
            .ok_or_else(|| js_error("FxSwapBuilder: nearDate is required".to_string()))?;
        let far_date = self
            .far_date
            .ok_or_else(|| js_error("FxSwapBuilder: farDate is required".to_string()))?;
        let domestic_curve = self
            .domestic_curve
            .as_deref()
            .ok_or_else(|| js_error("FxSwapBuilder: domesticCurve is required".to_string()))?;
        let foreign_curve = self
            .foreign_curve
            .as_deref()
            .ok_or_else(|| js_error("FxSwapBuilder: foreignCurve is required".to_string()))?;

        let mut builder = FxSwap::builder()
            .id(instrument_id_from_str(&self.instrument_id))
            .base_currency(base)
            .quote_currency(quote)
            .base_notional(notional)
            .near_date(near_date)
            .far_date(far_date)
            .domestic_discount_curve_id(curve_id_from_str(domestic_curve))
            .foreign_discount_curve_id(curve_id_from_str(foreign_curve));

        if let Some(rate) = self.near_rate {
            builder = builder.near_rate(rate);
        }
        if let Some(rate) = self.far_rate {
            builder = builder.far_rate(rate);
        }

        builder
            .build()
            .map(JsFxSwap::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }
}

#[wasm_bindgen(js_name = FxSwap)]
#[derive(Clone, Debug)]
pub struct JsFxSwap {
    pub(crate) inner: FxSwap,
}

impl InstrumentWrapper for JsFxSwap {
    type Inner = FxSwap;
    fn from_inner(inner: FxSwap) -> Self {
        JsFxSwap { inner }
    }
    fn inner(&self) -> FxSwap {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = FxSwap)]
impl JsFxSwap {
    /// Create an FX swap (near + far exchange of currencies).
    ///
    /// Conventions:
    /// - `near_rate` / `far_rate` are quoted as `quote_currency` per 1 unit of `base_currency`.
    /// - `notional` is the base-currency notional amount.
    ///
    /// @param instrument_id - Unique identifier
    /// @param base_currency - Base (foreign) currency
    /// @param quote_currency - Quote (domestic) currency
    /// @param notional - Base notional (currency-tagged)
    /// @param near_date - Near leg settlement date
    /// @param far_date - Far leg settlement date
    /// @param domestic_curve - Domestic (quote) discount curve ID
    /// @param foreign_curve - Foreign (base) discount curve ID
    /// @param near_rate - Optional near rate override
    /// @param far_rate - Optional far rate override
    /// @returns A new `FxSwap`
    /// @throws {Error} If dates are invalid or inputs inconsistent
    ///
    /// @example
    /// ```javascript
    /// import init, { FxSwap, Money, Currency, FsDate } from "finstack-wasm";
    ///
    /// await init();
    /// const swap = new FxSwap(
    ///   "eurusd_swap_1",
    ///   new Currency("EUR"),
    ///   new Currency("USD"),
    ///   Money.fromCode(1_000_000, "EUR"),
    ///   new FsDate(2024, 1, 4),
    ///   new FsDate(2024, 4, 4),
    ///   "USD-OIS",
    ///   "EUR-OIS",
    ///   1.10,
    ///   1.102
    /// );
    /// ```
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        base_currency: &JsCurrency,
        quote_currency: &JsCurrency,
        notional: &JsMoney,
        near_date: &JsDate,
        far_date: &JsDate,
        domestic_curve: &str,
        foreign_curve: &str,
        near_rate: Option<f64>,
        far_rate: Option<f64>,
    ) -> Result<JsFxSwap, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "FxSwap constructor is deprecated; use FxSwapBuilder instead.",
        ));
        let mut builder = FxSwap::builder()
            .id(instrument_id_from_str(instrument_id))
            .base_currency(base_currency.inner())
            .quote_currency(quote_currency.inner())
            .base_notional(notional.inner())
            .near_date(near_date.inner())
            .far_date(far_date.inner())
            .domestic_discount_curve_id(curve_id_from_str(domestic_curve))
            .foreign_discount_curve_id(curve_id_from_str(foreign_curve));

        if let Some(rate) = near_rate {
            builder = builder.near_rate(rate);
        }
        if let Some(rate) = far_rate {
            builder = builder.far_rate(rate);
        }

        builder
            .build()
            .map(JsFxSwap::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = baseCurrency)]
    pub fn base_currency(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.base_currency)
    }

    #[wasm_bindgen(getter, js_name = quoteCurrency)]
    pub fn quote_currency(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.quote_currency)
    }

    #[wasm_bindgen(getter, js_name = baseNotional)]
    pub fn base_notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.base_notional)
    }

    #[wasm_bindgen(getter, js_name = nearDate)]
    pub fn near_date(&self) -> JsDate {
        JsDate::from_core(self.inner.near_date)
    }

    #[wasm_bindgen(getter, js_name = farDate)]
    pub fn far_date(&self) -> JsDate {
        JsDate::from_core(self.inner.far_date)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::FxSwap as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "FxSwap(id='{}', near='{}', far='{}')",
            self.inner.id, self.inner.near_date, self.inner.far_date
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsFxSwap {
        JsFxSwap::from_inner(self.inner.clone())
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsFxSwap, JsValue> {
        from_js_value(value).map(JsFxSwap::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}
