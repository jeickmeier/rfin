//! FX Forward WASM bindings.

use crate::core::currency::JsCurrency;
use crate::core::dates::FsDate;
use crate::core::market_data::context::JsMarketContext;
use crate::utils::json::{from_js_value, to_js_value};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::fx::fx_forward::FxForward;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = FxForwardBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsFxForwardBuilder {
    instrument_id: String,
    base_currency: Option<finstack_core::currency::Currency>,
    quote_currency: Option<finstack_core::currency::Currency>,
    maturity_date: Option<finstack_core::dates::Date>,
    notional: Option<f64>,
    domestic_curve_id: Option<String>,
    foreign_curve_id: Option<String>,
}

#[wasm_bindgen(js_class = FxForwardBuilder)]
impl JsFxForwardBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsFxForwardBuilder {
        JsFxForwardBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = baseCurrency)]
    pub fn base_currency(mut self, base_currency: &JsCurrency) -> JsFxForwardBuilder {
        self.base_currency = Some(base_currency.inner());
        self
    }

    #[wasm_bindgen(js_name = quoteCurrency)]
    pub fn quote_currency(mut self, quote_currency: &JsCurrency) -> JsFxForwardBuilder {
        self.quote_currency = Some(quote_currency.inner());
        self
    }

    #[wasm_bindgen(js_name = maturityDate)]
    pub fn maturity_date(mut self, maturity_date: &FsDate) -> JsFxForwardBuilder {
        self.maturity_date = Some(maturity_date.inner());
        self
    }

    #[wasm_bindgen(js_name = notional)]
    pub fn notional(mut self, notional: f64) -> JsFxForwardBuilder {
        self.notional = Some(notional);
        self
    }

    #[wasm_bindgen(js_name = domesticCurveId)]
    pub fn domestic_curve_id(mut self, domestic_curve_id: &str) -> JsFxForwardBuilder {
        self.domestic_curve_id = Some(domestic_curve_id.to_string());
        self
    }

    #[wasm_bindgen(js_name = foreignCurveId)]
    pub fn foreign_curve_id(mut self, foreign_curve_id: &str) -> JsFxForwardBuilder {
        self.foreign_curve_id = Some(foreign_curve_id.to_string());
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsFxForward, JsValue> {
        use finstack_valuations::instruments::Attributes;

        let base = self
            .base_currency
            .ok_or_else(|| JsValue::from_str("FxForwardBuilder: baseCurrency is required"))?;
        let quote = self
            .quote_currency
            .ok_or_else(|| JsValue::from_str("FxForwardBuilder: quoteCurrency is required"))?;
        let maturity = self
            .maturity_date
            .ok_or_else(|| JsValue::from_str("FxForwardBuilder: maturityDate is required"))?;
        let notional = self
            .notional
            .ok_or_else(|| JsValue::from_str("FxForwardBuilder: notional is required"))?;
        let domestic = self
            .domestic_curve_id
            .as_deref()
            .ok_or_else(|| JsValue::from_str("FxForwardBuilder: domesticCurveId is required"))?;
        let foreign = self
            .foreign_curve_id
            .as_deref()
            .ok_or_else(|| JsValue::from_str("FxForwardBuilder: foreignCurveId is required"))?;

        let forward = FxForward::builder()
            .id(InstrumentId::new(&self.instrument_id))
            .base_currency(base)
            .quote_currency(quote)
            .maturity_date(maturity)
            .notional(Money::new(notional, base))
            .domestic_discount_curve_id(CurveId::new(domestic))
            .foreign_discount_curve_id(CurveId::new(foreign))
            .attributes(Attributes::new())
            .build()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(JsFxForward { inner: forward })
    }
}

/// FX Forward (outright forward) instrument.
///
/// Represents a commitment to exchange one currency for another at a specified
/// future date at a predetermined rate.
///
/// @example
/// ```javascript
/// const forward = new FxForward(
///   "EURUSD-FWD-6M",
///   Currency.EUR(),
///   Currency.USD(),
///   new FsDate(2025, 6, 15),
///   1_000_000,
///   "USD-OIS",
///   "EUR-OIS"
/// );
///
/// // Optionally set contract rate
/// forward.setContractRate(1.12);
/// ```
#[wasm_bindgen(js_name = FxForward)]
#[derive(Clone)]
pub struct JsFxForward {
    inner: FxForward,
}

impl JsFxForward {
    pub(crate) fn inner(&self) -> FxForward {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = FxForward)]
impl JsFxForward {
    /// Create a new FX forward.
    ///
    /// @param {string} id - Instrument identifier
    /// @param {Currency} baseCurrency - Base currency (foreign, numerator)
    /// @param {Currency} quoteCurrency - Quote currency (domestic, denominator, PV currency)
    /// @param {FsDate} maturityDate - Maturity/settlement date
    /// @param {number} notional - Notional amount in base currency
    /// @param {string} domesticCurveId - Domestic (quote) currency discount curve ID
    /// @param {string} foreignCurveId - Foreign (base) currency discount curve ID
    #[wasm_bindgen(constructor)]
    pub fn new(
        id: &str,
        base_currency: &JsCurrency,
        quote_currency: &JsCurrency,
        maturity_date: &FsDate,
        notional: f64,
        domestic_curve_id: &str,
        foreign_curve_id: &str,
    ) -> Result<JsFxForward, JsValue> {
        use finstack_valuations::instruments::Attributes;
        web_sys::console::warn_1(&JsValue::from_str(
            "FxForward constructor is deprecated; use FxForwardBuilder instead.",
        ));

        let forward = FxForward::builder()
            .id(InstrumentId::new(id))
            .base_currency(base_currency.inner())
            .quote_currency(quote_currency.inner())
            .maturity_date(maturity_date.inner())
            .notional(Money::new(notional, base_currency.inner()))
            .domestic_discount_curve_id(CurveId::new(domestic_curve_id))
            .foreign_discount_curve_id(CurveId::new(foreign_curve_id))
            .attributes(Attributes::new())
            .build()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(JsFxForward { inner: forward })
    }

    /// Get the instrument ID.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    /// Get the base currency.
    #[wasm_bindgen(getter, js_name = baseCurrency)]
    pub fn base_currency(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.base_currency)
    }

    /// Get the quote currency.
    #[wasm_bindgen(getter, js_name = quoteCurrency)]
    pub fn quote_currency(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.quote_currency)
    }

    /// Get the notional amount.
    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> f64 {
        self.inner.notional.amount()
    }

    /// Get the contract rate (if set).
    #[wasm_bindgen(getter, js_name = contractRate)]
    pub fn contract_rate(&self) -> Option<f64> {
        self.inner.contract_rate
    }

    /// Set the contract rate.
    #[wasm_bindgen(js_name = setContractRate)]
    pub fn set_contract_rate(&mut self, rate: f64) {
        self.inner.contract_rate = Some(rate);
    }

    /// Set the spot rate override.
    #[wasm_bindgen(js_name = setSpotRate)]
    pub fn set_spot_rate(&mut self, rate: f64) {
        self.inner.spot_rate_override = Some(rate);
    }

    /// Set contract rate from forward points.
    ///
    /// @param {number} spotRate - Current spot rate
    /// @param {number} forwardPoints - Forward points (difference from spot)
    #[wasm_bindgen(js_name = setForwardPoints)]
    pub fn set_forward_points(&mut self, spot_rate: f64, forward_points: f64) {
        self.inner.contract_rate = Some(spot_rate + forward_points);
        self.inner.spot_rate_override = Some(spot_rate);
    }

    /// Calculate the market forward rate via covered interest rate parity.
    #[wasm_bindgen(js_name = marketForwardRate)]
    pub fn market_forward_rate(
        &self,
        market: &JsMarketContext,
        as_of: &FsDate,
    ) -> Result<f64, JsValue> {
        self.inner
            .market_forward_rate(market.inner(), as_of.inner())
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Calculate present value.
    pub fn npv(&self, market: &JsMarketContext, as_of: &FsDate) -> Result<f64, JsValue> {
        self.inner
            .npv(market.inner(), as_of.inner())
            .map(|m| m.amount())
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsFxForward, JsValue> {
        from_js_value(value).map(|inner| JsFxForward { inner })
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}
