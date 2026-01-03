use crate::core::currency::JsCurrency;
use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::fx::fx_option::{FxOption, FxOptionParams};
use finstack_valuations::instruments::fx::fx_spot::FxSpot;
use finstack_valuations::instruments::fx::fx_swap::FxSwap;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

// ===========================
// FxSpot
// ===========================

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
        let mut inst = FxSpot::new(
            instrument_id_from_str(instrument_id),
            base_currency.inner(),
            quote_currency.inner(),
        );

        if let Some(date) = settlement {
            inst = inst.with_settlement(date.inner());
        }
        if let Some(rate) = spot_rate {
            inst = inst.with_rate(rate);
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
}

// ===========================
// FxOption
// ===========================

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
    /// Create a European FX call option.
    ///
    /// Conventions:
    /// - `strike` is quoted as `quote_currency` per 1 unit of `base_currency`.
    ///
    /// @param instrument_id - Unique identifier
    /// @param base_currency - Base (foreign) currency
    /// @param quote_currency - Quote (domestic) currency
    /// @param strike - Strike FX rate
    /// @param expiry - Expiry date
    /// @param notional - Notional (currency-tagged; typically in base currency)
    /// @param domestic_curve - Domestic (quote) discount curve ID
    /// @param foreign_curve - Foreign (base) discount curve ID
    /// @param vol_surface - Vol surface ID
    /// @returns A new `FxOption`
    #[wasm_bindgen(js_name = europeanCall)]
    #[allow(clippy::too_many_arguments)]
    pub fn european_call(
        instrument_id: &str,
        base_currency: &JsCurrency,
        quote_currency: &JsCurrency,
        strike: f64,
        expiry: &JsDate,
        notional: &JsMoney,
        domestic_curve: &str,
        foreign_curve: &str,
        vol_surface: &str,
    ) -> JsFxOption {
        use finstack_valuations::instruments::FxUnderlyingParams;
        let option_params = FxOptionParams::european_call(strike, expiry.inner(), notional.inner());
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
        JsFxOption::from_inner(option)
    }

    /// Create a European FX put option.
    ///
    /// Conventions:
    /// - `strike` is quoted as `quote_currency` per 1 unit of `base_currency`.
    ///
    /// @param instrument_id - Unique identifier
    /// @param base_currency - Base (foreign) currency
    /// @param quote_currency - Quote (domestic) currency
    /// @param strike - Strike FX rate
    /// @param expiry - Expiry date
    /// @param notional - Notional (currency-tagged; typically in base currency)
    /// @param domestic_curve - Domestic (quote) discount curve ID
    /// @param foreign_curve - Foreign (base) discount curve ID
    /// @param vol_surface - Vol surface ID
    /// @returns A new `FxOption`
    #[wasm_bindgen(js_name = europeanPut)]
    #[allow(clippy::too_many_arguments)]
    pub fn european_put(
        instrument_id: &str,
        base_currency: &JsCurrency,
        quote_currency: &JsCurrency,
        strike: f64,
        expiry: &JsDate,
        notional: &JsMoney,
        domestic_curve: &str,
        foreign_curve: &str,
        vol_surface: &str,
    ) -> JsFxOption {
        use finstack_valuations::instruments::FxUnderlyingParams;
        let option_params = FxOptionParams::european_put(strike, expiry.inner(), notional.inner());
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
        JsFxOption::from_inner(option)
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
}
