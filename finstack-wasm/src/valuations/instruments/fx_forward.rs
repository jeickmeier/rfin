//! FX Forward WASM bindings.

use crate::core::currency::JsCurrency;
use crate::core::dates::FsDate;
use crate::core::market_data::context::JsMarketContext;
use crate::utils::json::{from_js_value, to_js_value};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::fx::fx_forward::FxForward;
use wasm_bindgen::prelude::*;

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
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsFxForward, JsValue> {
        from_js_value(value).map(|inner| JsFxForward { inner })
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}
