//! FX Variance Swap WASM bindings.

use crate::core::currency::JsCurrency;
use crate::core::dates::daycount::JsTenor;
use crate::core::dates::FsDate;
use crate::core::market_data::context::JsMarketContext;
use crate::utils::json::{from_js_value, to_js_value};
use finstack_core::dates::DayCount;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::fx::fx_variance_swap::FxVarianceSwap;
use finstack_valuations::instruments::Attributes;
use finstack_valuations::instruments::PayReceive;
use finstack_valuations::prelude::Instrument;
use wasm_bindgen::prelude::*;

/// Pay/receive side for variance swap.
#[wasm_bindgen(js_name = VarianceSwapSide)]
#[derive(Clone, Copy)]
pub struct JsVarianceSwapSide {
    inner: PayReceive,
}

#[wasm_bindgen(js_class = VarianceSwapSide)]
impl JsVarianceSwapSide {
    /// Pay variance (short variance).
    #[wasm_bindgen(js_name = Pay)]
    pub fn pay() -> JsVarianceSwapSide {
        JsVarianceSwapSide {
            inner: PayReceive::Pay,
        }
    }

    /// Receive variance (long variance).
    #[wasm_bindgen(js_name = Receive)]
    pub fn receive() -> JsVarianceSwapSide {
        JsVarianceSwapSide {
            inner: PayReceive::Receive,
        }
    }
}

impl JsVarianceSwapSide {
    pub(crate) fn inner(&self) -> PayReceive {
        self.inner
    }
}

/// FX variance swap instrument.
///
/// A contract that exchanges the realized variance of an FX rate over a
/// specified period against a fixed strike variance.
///
/// @example
/// ```javascript
/// const swap = new FxVarianceSwap(
///   "FXVAR-EURUSD-1Y",
///   Currency.EUR(),
///   Currency.USD(),
///   1_000_000,                  // Notional (vega notional)
///   0.04,                        // Strike variance (20% vol squared)
///   new FsDate(2024, 1, 2),     // Start date
///   new FsDate(2025, 1, 2),     // Maturity
///   Tenor.daily(),
///   VarianceSwapSide.Receive(),
///   "USD-OIS",
///   "EUR-OIS",
///   "EURUSD-VOL"
/// );
/// ```
#[wasm_bindgen(js_name = FxVarianceSwap)]
#[derive(Clone)]
pub struct JsFxVarianceSwap {
    inner: FxVarianceSwap,
}

impl JsFxVarianceSwap {
    pub(crate) fn inner(&self) -> FxVarianceSwap {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_name = FxVarianceSwapBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsFxVarianceSwapBuilder {
    instrument_id: String,
    base_currency: Option<finstack_core::currency::Currency>,
    quote_currency: Option<finstack_core::currency::Currency>,
    notional: Option<f64>,
    strike_variance: Option<f64>,
    start_date: Option<finstack_core::dates::Date>,
    maturity: Option<finstack_core::dates::Date>,
    observation_freq: Option<finstack_core::dates::Tenor>,
    side: Option<PayReceive>,
    domestic_curve_id: Option<String>,
    foreign_curve_id: Option<String>,
    vol_surface_id: Option<String>,
}

#[wasm_bindgen(js_class = FxVarianceSwapBuilder)]
impl JsFxVarianceSwapBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsFxVarianceSwapBuilder {
        JsFxVarianceSwapBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = baseCurrency)]
    pub fn base_currency(mut self, base_currency: &JsCurrency) -> JsFxVarianceSwapBuilder {
        self.base_currency = Some(base_currency.inner());
        self
    }

    #[wasm_bindgen(js_name = quoteCurrency)]
    pub fn quote_currency(mut self, quote_currency: &JsCurrency) -> JsFxVarianceSwapBuilder {
        self.quote_currency = Some(quote_currency.inner());
        self
    }

    #[wasm_bindgen(js_name = notional)]
    pub fn notional(mut self, notional: f64) -> JsFxVarianceSwapBuilder {
        self.notional = Some(notional);
        self
    }

    #[wasm_bindgen(js_name = strikeVariance)]
    pub fn strike_variance(mut self, strike_variance: f64) -> JsFxVarianceSwapBuilder {
        self.strike_variance = Some(strike_variance);
        self
    }

    #[wasm_bindgen(js_name = startDate)]
    pub fn start_date(mut self, start_date: &FsDate) -> JsFxVarianceSwapBuilder {
        self.start_date = Some(start_date.inner());
        self
    }

    #[wasm_bindgen(js_name = maturity)]
    pub fn maturity(mut self, maturity: &FsDate) -> JsFxVarianceSwapBuilder {
        self.maturity = Some(maturity.inner());
        self
    }

    #[wasm_bindgen(js_name = observationFreq)]
    pub fn observation_freq(mut self, observation_freq: &JsTenor) -> JsFxVarianceSwapBuilder {
        self.observation_freq = Some(observation_freq.inner());
        self
    }

    #[wasm_bindgen(js_name = side)]
    pub fn side(mut self, side: &JsVarianceSwapSide) -> JsFxVarianceSwapBuilder {
        self.side = Some(side.inner());
        self
    }

    #[wasm_bindgen(js_name = domesticCurveId)]
    pub fn domestic_curve_id(mut self, domestic_curve_id: &str) -> JsFxVarianceSwapBuilder {
        self.domestic_curve_id = Some(domestic_curve_id.to_string());
        self
    }

    #[wasm_bindgen(js_name = foreignCurveId)]
    pub fn foreign_curve_id(mut self, foreign_curve_id: &str) -> JsFxVarianceSwapBuilder {
        self.foreign_curve_id = Some(foreign_curve_id.to_string());
        self
    }

    #[wasm_bindgen(js_name = volSurfaceId)]
    pub fn vol_surface_id(mut self, vol_surface_id: &str) -> JsFxVarianceSwapBuilder {
        self.vol_surface_id = Some(vol_surface_id.to_string());
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsFxVarianceSwap, JsValue> {
        use finstack_core::math::stats::RealizedVarMethod;

        let base_currency = self
            .base_currency
            .ok_or_else(|| JsValue::from_str("FxVarianceSwapBuilder: baseCurrency is required"))?;
        let quote_currency = self
            .quote_currency
            .ok_or_else(|| JsValue::from_str("FxVarianceSwapBuilder: quoteCurrency is required"))?;
        let notional = self
            .notional
            .ok_or_else(|| JsValue::from_str("FxVarianceSwapBuilder: notional is required"))?;
        let strike_variance = self.strike_variance.ok_or_else(|| {
            JsValue::from_str("FxVarianceSwapBuilder: strikeVariance is required")
        })?;
        let start_date = self
            .start_date
            .ok_or_else(|| JsValue::from_str("FxVarianceSwapBuilder: startDate is required"))?;
        let maturity = self
            .maturity
            .ok_or_else(|| JsValue::from_str("FxVarianceSwapBuilder: maturity is required"))?;
        let observation_freq = self.observation_freq.ok_or_else(|| {
            JsValue::from_str("FxVarianceSwapBuilder: observationFreq is required")
        })?;
        let side = self
            .side
            .ok_or_else(|| JsValue::from_str("FxVarianceSwapBuilder: side is required"))?;
        let domestic_curve_id = self.domestic_curve_id.as_deref().ok_or_else(|| {
            JsValue::from_str("FxVarianceSwapBuilder: domesticCurveId is required")
        })?;
        let foreign_curve_id = self.foreign_curve_id.as_deref().ok_or_else(|| {
            JsValue::from_str("FxVarianceSwapBuilder: foreignCurveId is required")
        })?;
        let vol_surface_id = self
            .vol_surface_id
            .as_deref()
            .ok_or_else(|| JsValue::from_str("FxVarianceSwapBuilder: volSurfaceId is required"))?;

        let swap = FxVarianceSwap::builder()
            .id(InstrumentId::new(&self.instrument_id))
            .base_currency(base_currency)
            .quote_currency(quote_currency)
            .notional(Money::new(notional, quote_currency))
            .strike_variance(strike_variance)
            .start_date(start_date)
            .maturity(maturity)
            .observation_freq(observation_freq)
            .realized_var_method(RealizedVarMethod::CloseToClose)
            .side(side)
            .domestic_discount_curve_id(CurveId::new(domestic_curve_id))
            .foreign_discount_curve_id(CurveId::new(foreign_curve_id))
            .vol_surface_id(CurveId::new(vol_surface_id))
            .day_count(DayCount::Act365F)
            .attributes(Attributes::new())
            .build()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(JsFxVarianceSwap { inner: swap })
    }
}

#[wasm_bindgen(js_class = FxVarianceSwap)]
impl JsFxVarianceSwap {
    /// Create a new FX variance swap.
    ///
    /// @param {string} id - Instrument identifier
    /// @param {Currency} baseCurrency - Base currency (foreign)
    /// @param {Currency} quoteCurrency - Quote currency (domestic)
    /// @param {number} notional - Variance notional in quote currency
    /// @param {number} strikeVariance - Strike variance (annualized)
    /// @param {FsDate} startDate - Start date of observation period
    /// @param {FsDate} maturity - Maturity/settlement date
    /// @param {Tenor} observationFreq - Observation frequency
    /// @param {VarianceSwapSide} side - Pay or receive variance
    /// @param {string} domesticCurveId - Domestic discount curve ID
    /// @param {string} foreignCurveId - Foreign discount curve ID
    /// @param {string} volSurfaceId - FX volatility surface ID
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &str,
        base_currency: &JsCurrency,
        quote_currency: &JsCurrency,
        notional: f64,
        strike_variance: f64,
        start_date: &FsDate,
        maturity: &FsDate,
        observation_freq: &JsTenor,
        side: &JsVarianceSwapSide,
        domestic_curve_id: &str,
        foreign_curve_id: &str,
        vol_surface_id: &str,
    ) -> Result<JsFxVarianceSwap, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "FxVarianceSwap constructor is deprecated; use FxVarianceSwapBuilder instead.",
        ));
        use finstack_core::math::stats::RealizedVarMethod;

        let swap = FxVarianceSwap::builder()
            .id(InstrumentId::new(id))
            .base_currency(base_currency.inner())
            .quote_currency(quote_currency.inner())
            .notional(Money::new(notional, quote_currency.inner()))
            .strike_variance(strike_variance)
            .start_date(start_date.inner())
            .maturity(maturity.inner())
            .observation_freq(observation_freq.inner())
            .realized_var_method(RealizedVarMethod::CloseToClose)
            .side(side.inner())
            .domestic_discount_curve_id(CurveId::new(domestic_curve_id))
            .foreign_discount_curve_id(CurveId::new(foreign_curve_id))
            .vol_surface_id(CurveId::new(vol_surface_id))
            .day_count(DayCount::Act365F)
            .attributes(Attributes::new())
            .build()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(JsFxVarianceSwap { inner: swap })
    }

    /// Get the instrument ID.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    /// Get the strike variance.
    #[wasm_bindgen(getter, js_name = strikeVariance)]
    pub fn strike_variance(&self) -> f64 {
        self.inner.strike_variance
    }

    /// Get the notional amount.
    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> f64 {
        self.inner.notional.amount()
    }

    /// Get the strike volatility (sqrt of variance).
    #[wasm_bindgen(getter, js_name = strikeVol)]
    pub fn strike_vol(&self) -> f64 {
        self.inner.strike_variance.sqrt()
    }

    /// Get the annualization factor.
    #[wasm_bindgen(js_name = annualizationFactor)]
    pub fn annualization_factor(&self) -> f64 {
        self.inner.annualization_factor()
    }

    /// Calculate the payoff for a given realized variance.
    pub fn payoff(&self, realized_variance: f64) -> f64 {
        self.inner.payoff(realized_variance).amount()
    }

    /// Calculate present value.
    #[wasm_bindgen(js_name = value)]
    pub fn value(&self, market: &JsMarketContext, as_of: &FsDate) -> Result<f64, JsValue> {
        self.inner
            .value(market.inner(), as_of.inner())
            .map(|m| m.amount())
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsFxVarianceSwap, JsValue> {
        from_js_value(value).map(|inner| JsFxVarianceSwap { inner })
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}
