//! NDF (Non-Deliverable Forward) WASM bindings.

use crate::core::currency::JsCurrency;
use crate::core::dates::FsDate;
use crate::core::market_data::context::JsMarketContext;
use crate::utils::json::{from_js_value, to_js_value};
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::fx::ndf::Ndf;
use finstack_valuations::prelude::Instrument;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = NdfBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsNdfBuilder {
    instrument_id: String,
    base_currency: Option<finstack_core::currency::Currency>,
    settlement_currency: Option<finstack_core::currency::Currency>,
    fixing_date: Option<finstack_core::dates::Date>,
    maturity_date: Option<finstack_core::dates::Date>,
    notional: Option<f64>,
    contract_rate: Option<f64>,
    domestic_discount_curve_id: Option<String>,
}

#[wasm_bindgen(js_class = NdfBuilder)]
impl JsNdfBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsNdfBuilder {
        JsNdfBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = baseCurrency)]
    pub fn base_currency(mut self, base_currency: &JsCurrency) -> JsNdfBuilder {
        self.base_currency = Some(base_currency.inner());
        self
    }

    #[wasm_bindgen(js_name = settlementCurrency)]
    pub fn settlement_currency(mut self, settlement_currency: &JsCurrency) -> JsNdfBuilder {
        self.settlement_currency = Some(settlement_currency.inner());
        self
    }

    #[wasm_bindgen(js_name = fixingDate)]
    pub fn fixing_date(mut self, fixing_date: &FsDate) -> JsNdfBuilder {
        self.fixing_date = Some(fixing_date.inner());
        self
    }

    #[wasm_bindgen(js_name = maturityDate)]
    pub fn maturity_date(mut self, maturity_date: &FsDate) -> JsNdfBuilder {
        self.maturity_date = Some(maturity_date.inner());
        self
    }

    #[wasm_bindgen(js_name = notional)]
    pub fn notional(mut self, notional: f64) -> JsNdfBuilder {
        self.notional = Some(notional);
        self
    }

    #[wasm_bindgen(js_name = contractRate)]
    pub fn contract_rate(mut self, contract_rate: f64) -> JsNdfBuilder {
        self.contract_rate = Some(contract_rate);
        self
    }

    #[wasm_bindgen(js_name = settlementCurveId)]
    pub fn settlement_curve_id(mut self, settlement_curve_id: &str) -> JsNdfBuilder {
        self.domestic_discount_curve_id = Some(settlement_curve_id.to_string());
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsNdf, JsValue> {
        use finstack_valuations::instruments::Attributes;

        let base_currency = self
            .base_currency
            .ok_or_else(|| JsValue::from_str("NdfBuilder: baseCurrency is required"))?;
        let settlement_currency = self
            .settlement_currency
            .ok_or_else(|| JsValue::from_str("NdfBuilder: settlementCurrency is required"))?;
        let fixing_date = self
            .fixing_date
            .ok_or_else(|| JsValue::from_str("NdfBuilder: fixingDate is required"))?;
        let maturity_date = self
            .maturity_date
            .ok_or_else(|| JsValue::from_str("NdfBuilder: maturityDate is required"))?;
        let notional = self
            .notional
            .ok_or_else(|| JsValue::from_str("NdfBuilder: notional is required"))?;
        let contract_rate = self
            .contract_rate
            .ok_or_else(|| JsValue::from_str("NdfBuilder: contractRate is required"))?;
        let settlement_curve_id = self
            .domestic_discount_curve_id
            .as_deref()
            .ok_or_else(|| JsValue::from_str("NdfBuilder: settlementCurveId is required"))?;

        let ndf = Ndf::builder()
            .id(InstrumentId::new(&self.instrument_id))
            .base_currency(base_currency)
            .settlement_currency(settlement_currency)
            .fixing_date(fixing_date)
            .maturity(maturity_date)
            .notional(Money::new(notional, base_currency))
            .contract_rate(contract_rate)
            .domestic_discount_curve_id(CurveId::new(settlement_curve_id))
            .attributes(Attributes::new())
            .build()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(JsNdf { inner: ndf })
    }
}

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
        self.inner.foreign_discount_curve_id = Some(CurveId::new(curve_id));
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
    pub fn from_json(value: JsValue) -> Result<JsNdf, JsValue> {
        from_js_value(value).map(|inner| JsNdf { inner })
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}
