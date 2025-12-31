//! NDF (Non-Deliverable Forward) WASM bindings.

use crate::core::currency::JsCurrency;
use crate::core::dates::FsDate;
use crate::core::market_data::context::JsMarketContext;
use crate::utils::json::{from_js_value, to_js_value};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::ndf::Ndf;
use wasm_bindgen::prelude::*;

/// Non-Deliverable Forward (NDF) instrument.
///
/// A cash-settled forward contract on a restricted currency pair.
/// Settlement is in the convertible (settlement) currency, typically USD.
///
/// @example
/// ```javascript
/// const ndf = new Ndf(
///   "USDCNY-NDF-3M",
///   Currency.CNY(),  // Base (restricted)
///   Currency.USD(),  // Settlement
///   new FsDate(2025, 3, 13),  // Fixing date
///   new FsDate(2025, 3, 15),  // Maturity date
///   10_000_000,       // Notional in CNY
///   7.25,             // Contract rate (CNY per USD)
///   "USD-OIS"         // Settlement curve
/// );
///
/// // After fixing
/// ndf.setFixingRate(7.30);
/// ```
#[wasm_bindgen(js_name = Ndf)]
#[derive(Clone)]
pub struct JsNdf {
    inner: Ndf,
}

impl JsNdf {
    pub(crate) fn inner(&self) -> Ndf {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = Ndf)]
impl JsNdf {
    /// Create a new NDF.
    ///
    /// @param {string} id - Instrument identifier
    /// @param {Currency} baseCurrency - Base currency (restricted, numerator)
    /// @param {Currency} settlementCurrency - Settlement currency (convertible, typically USD)
    /// @param {FsDate} fixingDate - Rate observation date (typically T-2 before maturity)
    /// @param {FsDate} maturityDate - Maturity/settlement date
    /// @param {number} notional - Notional amount in base currency
    /// @param {number} contractRate - Contract forward rate (base per settlement)
    /// @param {string} settlementCurveId - Settlement currency discount curve ID
    #[allow(clippy::too_many_arguments)]
    #[wasm_bindgen(constructor)]
    pub fn new(
        id: &str,
        base_currency: &JsCurrency,
        settlement_currency: &JsCurrency,
        fixing_date: &FsDate,
        maturity_date: &FsDate,
        notional: f64,
        contract_rate: f64,
        settlement_curve_id: &str,
    ) -> Result<JsNdf, JsValue> {
        use finstack_valuations::instruments::common::traits::Attributes;

        let ndf = Ndf::builder()
            .id(InstrumentId::new(id))
            .base_currency(base_currency.inner())
            .settlement_currency(settlement_currency.inner())
            .fixing_date(fixing_date.inner())
            .maturity_date(maturity_date.inner())
            .notional(Money::new(notional, base_currency.inner()))
            .contract_rate(contract_rate)
            .settlement_curve_id(CurveId::new(settlement_curve_id))
            .attributes(Attributes::new())
            .build()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(JsNdf { inner: ndf })
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

    /// Get the settlement currency.
    #[wasm_bindgen(getter, js_name = settlementCurrency)]
    pub fn settlement_currency(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.settlement_currency)
    }

    /// Get the contract rate.
    #[wasm_bindgen(getter, js_name = contractRate)]
    pub fn contract_rate(&self) -> f64 {
        self.inner.contract_rate
    }

    /// Get the fixing rate (if set).
    #[wasm_bindgen(getter, js_name = fixingRate)]
    pub fn fixing_rate(&self) -> Option<f64> {
        self.inner.fixing_rate
    }

    /// Check if NDF is in post-fixing mode.
    #[wasm_bindgen(js_name = isFixed)]
    pub fn is_fixed(&self) -> bool {
        self.inner.is_fixed()
    }

    /// Set the observed fixing rate (transitions to post-fixing mode).
    #[wasm_bindgen(js_name = setFixingRate)]
    pub fn set_fixing_rate(&mut self, rate: f64) {
        self.inner.fixing_rate = Some(rate);
    }

    /// Set the spot rate override.
    #[wasm_bindgen(js_name = setSpotRate)]
    pub fn set_spot_rate(&mut self, rate: f64) {
        self.inner.spot_rate_override = Some(rate);
    }

    /// Set the foreign discount curve ID (for CIRP-based forward estimation).
    #[wasm_bindgen(js_name = setForeignCurveId)]
    pub fn set_foreign_curve_id(&mut self, curve_id: &str) {
        self.inner.foreign_curve_id = Some(CurveId::new(curve_id));
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
    pub fn from_json(value: JsValue) -> Result<JsNdf, JsValue> {
        from_js_value(value).map(|inner| JsNdf { inner })
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}
