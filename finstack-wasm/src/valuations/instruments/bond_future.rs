//! Bond Future WASM bindings.

use crate::core::dates::FsDate;
use crate::core::market_data::context::JsMarketContext;
use crate::utils::json::{from_js_value, to_js_value};
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::fixed_income::bond_future::{
    BondFuture, BondFutureSpecs, DeliverableBond, Position,
};
use wasm_bindgen::prelude::*;

/// Position side for futures contracts.
#[wasm_bindgen(js_name = FuturePosition)]
#[derive(Clone, Copy)]
pub struct JsFuturePosition {
    inner: Position,
}

#[wasm_bindgen(js_class = FuturePosition)]
impl JsFuturePosition {
    /// Long position (buying the future).
    #[wasm_bindgen(js_name = Long)]
    pub fn long() -> JsFuturePosition {
        JsFuturePosition {
            inner: Position::Long,
        }
    }

    /// Short position (selling the future).
    #[wasm_bindgen(js_name = Short)]
    pub fn short() -> JsFuturePosition {
        JsFuturePosition {
            inner: Position::Short,
        }
    }

    /// Check if this is a long position.
    #[wasm_bindgen(js_name = isLong)]
    pub fn is_long(&self) -> bool {
        matches!(self.inner, Position::Long)
    }
}

impl JsFuturePosition {
    pub(crate) fn inner(&self) -> Position {
        self.inner
    }
}

/// Bond future contract specifications.
#[wasm_bindgen(js_name = BondFutureSpecs)]
#[derive(Clone)]
pub struct JsBondFutureSpecs {
    inner: BondFutureSpecs,
}

#[wasm_bindgen(js_class = BondFutureSpecs)]
impl JsBondFutureSpecs {
    /// UST 10-year futures specifications.
    #[wasm_bindgen(js_name = ust10y)]
    pub fn ust_10y() -> JsBondFutureSpecs {
        JsBondFutureSpecs {
            inner: BondFutureSpecs::ust_10y(),
        }
    }

    /// UST 5-year futures specifications.
    #[wasm_bindgen(js_name = ust5y)]
    pub fn ust_5y() -> JsBondFutureSpecs {
        JsBondFutureSpecs {
            inner: BondFutureSpecs::ust_5y(),
        }
    }

    /// UST 2-year futures specifications.
    #[wasm_bindgen(js_name = ust2y)]
    pub fn ust_2y() -> JsBondFutureSpecs {
        JsBondFutureSpecs {
            inner: BondFutureSpecs::ust_2y(),
        }
    }

    /// German Bund futures specifications.
    pub fn bund() -> JsBondFutureSpecs {
        JsBondFutureSpecs {
            inner: BondFutureSpecs::bund(),
        }
    }

    /// UK Gilt futures specifications.
    pub fn gilt() -> JsBondFutureSpecs {
        JsBondFutureSpecs {
            inner: BondFutureSpecs::gilt(),
        }
    }

    /// Get the contract size.
    #[wasm_bindgen(getter, js_name = contractSize)]
    pub fn contract_size(&self) -> f64 {
        self.inner.contract_size
    }

    /// Get the tick size.
    #[wasm_bindgen(getter, js_name = tickSize)]
    pub fn tick_size(&self) -> f64 {
        self.inner.tick_size
    }

    /// Get the tick value.
    #[wasm_bindgen(getter, js_name = tickValue)]
    pub fn tick_value(&self) -> f64 {
        self.inner.tick_value
    }
}

impl JsBondFutureSpecs {
    pub(crate) fn inner(&self) -> BondFutureSpecs {
        self.inner.clone()
    }
}

/// Bond future instrument.
///
/// A futures contract on government bonds (UST, Bund, Gilt) with a
/// deliverable basket and cheapest-to-deliver (CTD) pricing.
///
/// @example
/// ```javascript
/// const specs = BondFutureSpecs.ust10y();
/// const future = new BondFuture(
///   "TYH5",
///   1_000_000,                  // Notional USD
///   "USD",
///   new FsDate(2025, 3, 20),    // Expiry
///   new FsDate(2025, 3, 21),    // Delivery start
///   new FsDate(2025, 3, 31),    // Delivery end
///   125.50,                      // Quoted price
///   FuturePosition.Long(),
///   specs,
///   [{ bondId: "US912828XG33", conversionFactor: 0.8234 }],
///   "US912828XG33",             // CTD bond ID
///   "USD-TREASURY"
/// );
/// ```
#[wasm_bindgen(js_name = BondFuture)]
#[derive(Clone)]
pub struct JsBondFuture {
    inner: BondFuture,
}

impl JsBondFuture {
    pub(crate) fn inner(&self) -> BondFuture {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_name = BondFutureBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsBondFutureBuilder {
    instrument_id: String,
    notional: Option<f64>,
    currency: Option<String>,
    expiry_date: Option<finstack_core::dates::Date>,
    delivery_start: Option<finstack_core::dates::Date>,
    delivery_end: Option<finstack_core::dates::Date>,
    quoted_price: Option<f64>,
    position: Option<Position>,
    specs: Option<BondFutureSpecs>,
    deliverable_basket: Option<JsValue>,
    ctd_bond_id: Option<String>,
    discount_curve_id: Option<String>,
}

#[wasm_bindgen(js_class = BondFutureBuilder)]
impl JsBondFutureBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsBondFutureBuilder {
        JsBondFutureBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = notional)]
    pub fn notional(mut self, notional: f64) -> JsBondFutureBuilder {
        self.notional = Some(notional);
        self
    }

    #[wasm_bindgen(js_name = currency)]
    pub fn currency(mut self, currency: String) -> JsBondFutureBuilder {
        self.currency = Some(currency);
        self
    }

    #[wasm_bindgen(js_name = expiryDate)]
    pub fn expiry_date(mut self, expiry_date: &FsDate) -> JsBondFutureBuilder {
        self.expiry_date = Some(expiry_date.inner());
        self
    }

    #[wasm_bindgen(js_name = deliveryStart)]
    pub fn delivery_start(mut self, delivery_start: &FsDate) -> JsBondFutureBuilder {
        self.delivery_start = Some(delivery_start.inner());
        self
    }

    #[wasm_bindgen(js_name = deliveryEnd)]
    pub fn delivery_end(mut self, delivery_end: &FsDate) -> JsBondFutureBuilder {
        self.delivery_end = Some(delivery_end.inner());
        self
    }

    #[wasm_bindgen(js_name = quotedPrice)]
    pub fn quoted_price(mut self, quoted_price: f64) -> JsBondFutureBuilder {
        self.quoted_price = Some(quoted_price);
        self
    }

    #[wasm_bindgen(js_name = position)]
    pub fn position(mut self, position: &JsFuturePosition) -> JsBondFutureBuilder {
        self.position = Some(position.inner());
        self
    }

    #[wasm_bindgen(js_name = specs)]
    pub fn specs(mut self, specs: &JsBondFutureSpecs) -> JsBondFutureBuilder {
        self.specs = Some(specs.inner());
        self
    }

    #[wasm_bindgen(js_name = deliverableBasket)]
    pub fn deliverable_basket(mut self, deliverable_basket: JsValue) -> JsBondFutureBuilder {
        self.deliverable_basket = Some(deliverable_basket);
        self
    }

    #[wasm_bindgen(js_name = ctdBondId)]
    pub fn ctd_bond_id(mut self, ctd_bond_id: String) -> JsBondFutureBuilder {
        self.ctd_bond_id = Some(ctd_bond_id);
        self
    }

    #[wasm_bindgen(js_name = discountCurveId)]
    pub fn discount_curve_id(mut self, discount_curve_id: String) -> JsBondFutureBuilder {
        self.discount_curve_id = Some(discount_curve_id);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsBondFuture, JsValue> {
        let notional = self
            .notional
            .ok_or_else(|| JsValue::from_str("BondFutureBuilder: notional is required"))?;
        let currency = self
            .currency
            .as_deref()
            .ok_or_else(|| JsValue::from_str("BondFutureBuilder: currency is required"))?;
        let expiry_date = self
            .expiry_date
            .ok_or_else(|| JsValue::from_str("BondFutureBuilder: expiryDate is required"))?;
        let delivery_start = self
            .delivery_start
            .ok_or_else(|| JsValue::from_str("BondFutureBuilder: deliveryStart is required"))?;
        let delivery_end = self
            .delivery_end
            .ok_or_else(|| JsValue::from_str("BondFutureBuilder: deliveryEnd is required"))?;
        let quoted_price = self
            .quoted_price
            .ok_or_else(|| JsValue::from_str("BondFutureBuilder: quotedPrice is required"))?;
        let position = self
            .position
            .ok_or_else(|| JsValue::from_str("BondFutureBuilder: position is required"))?;
        let specs = self
            .specs
            .ok_or_else(|| JsValue::from_str("BondFutureBuilder: specs is required"))?;
        let deliverable_basket = self
            .deliverable_basket
            .ok_or_else(|| JsValue::from_str("BondFutureBuilder: deliverableBasket is required"))?;
        let ctd_bond_id = self
            .ctd_bond_id
            .as_deref()
            .ok_or_else(|| JsValue::from_str("BondFutureBuilder: ctdBondId is required"))?;
        let discount_curve_id = self
            .discount_curve_id
            .as_deref()
            .ok_or_else(|| JsValue::from_str("BondFutureBuilder: discountCurveId is required"))?;

        let ccy: Currency = currency
            .parse()
            .map_err(|e: strum::ParseError| JsValue::from_str(&e.to_string()))?;

        let basket_raw: Vec<DeliverableBondJs> = serde_wasm_bindgen::from_value(deliverable_basket)
            .map_err(|e| JsValue::from_str(&format!("Invalid deliverable basket: {}", e)))?;
        let basket: Vec<DeliverableBond> = basket_raw
            .into_iter()
            .map(|b| DeliverableBond {
                bond_id: InstrumentId::new(&b.bond_id),
                conversion_factor: b.conversion_factor,
            })
            .collect();

        let future = BondFuture::ust_10y(
            InstrumentId::new(&self.instrument_id),
            Money::new(notional, ccy),
            expiry_date,
            delivery_start,
            delivery_end,
            quoted_price,
            position,
            basket,
            InstrumentId::new(ctd_bond_id),
            CurveId::new(discount_curve_id),
        )
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let mut result = future;
        result.contract_specs = specs;

        Ok(JsBondFuture { inner: result })
    }
}

#[wasm_bindgen(js_class = BondFuture)]
impl JsBondFuture {
    /// Create a new bond future.
    ///
    /// @param {string} id - Instrument identifier (e.g., "TYH5")
    /// @param {number} notional - Total notional exposure
    /// @param {string} currency - Currency code
    /// @param {FsDate} expiryDate - Last trading day
    /// @param {FsDate} deliveryStart - First delivery date
    /// @param {FsDate} deliveryEnd - Last delivery date
    /// @param {number} quotedPrice - Futures price (e.g., 125.50)
    /// @param {FuturePosition} position - Long or Short
    /// @param {BondFutureSpecs} specs - Contract specifications
    /// @param {Array} deliverableBasket - Array of {bondId, conversionFactor}
    /// @param {string} ctdBondId - Cheapest-to-deliver bond ID
    /// @param {string} discountCurveId - Discount curve ID
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &str,
        notional: f64,
        currency: &str,
        expiry_date: &FsDate,
        delivery_start: &FsDate,
        delivery_end: &FsDate,
        quoted_price: f64,
        position: &JsFuturePosition,
        specs: &JsBondFutureSpecs,
        deliverable_basket: JsValue,
        ctd_bond_id: &str,
        discount_curve_id: &str,
    ) -> Result<JsBondFuture, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "BondFuture constructor is deprecated; use BondFutureBuilder instead.",
        ));
        let ccy: Currency = currency
            .parse()
            .map_err(|e: strum::ParseError| JsValue::from_str(&e.to_string()))?;

        // Parse deliverable basket from JS array
        let basket_raw: Vec<DeliverableBondJs> = serde_wasm_bindgen::from_value(deliverable_basket)
            .map_err(|e| JsValue::from_str(&format!("Invalid deliverable basket: {}", e)))?;

        let basket: Vec<DeliverableBond> = basket_raw
            .into_iter()
            .map(|b| DeliverableBond {
                bond_id: InstrumentId::new(&b.bond_id),
                conversion_factor: b.conversion_factor,
            })
            .collect();

        let future = BondFuture::ust_10y(
            InstrumentId::new(id),
            Money::new(notional, ccy),
            expiry_date.inner(),
            delivery_start.inner(),
            delivery_end.inner(),
            quoted_price,
            position.inner(),
            basket,
            InstrumentId::new(ctd_bond_id),
            CurveId::new(discount_curve_id),
        )
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

        // Replace specs if different from UST 10Y
        let mut result = future;
        result.contract_specs = specs.inner();

        Ok(JsBondFuture { inner: result })
    }

    /// Get the instrument ID.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    /// Get the quoted price.
    #[wasm_bindgen(getter, js_name = quotedPrice)]
    pub fn quoted_price(&self) -> f64 {
        self.inner.quoted_price
    }

    /// Get the notional amount.
    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> f64 {
        self.inner.notional.amount()
    }

    /// Get the CTD bond ID.
    #[wasm_bindgen(getter, js_name = ctdBondId)]
    pub fn ctd_bond_id(&self) -> String {
        self.inner
            .ctd_bond_id
            .as_ref()
            .map(|id| id.as_str().to_string())
            .unwrap_or_default()
    }

    /// Calculate present value.
    #[wasm_bindgen(js_name = value)]
    pub fn value(&self, market: &JsMarketContext, as_of: &FsDate) -> Result<f64, JsValue> {
        use finstack_valuations::instruments::Instrument;

        self.inner
            .value(market.inner(), as_of.inner())
            .map(|m| m.amount())
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsBondFuture, JsValue> {
        from_js_value(value).map(|inner| JsBondFuture { inner })
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}

/// Helper struct for deserializing deliverable bonds from JS.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeliverableBondJs {
    bond_id: String,
    conversion_factor: f64,
}
