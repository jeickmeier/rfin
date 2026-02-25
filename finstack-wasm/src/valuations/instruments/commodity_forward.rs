//! WASM bindings for CommodityForward instrument.

use crate::core::currency::JsCurrency;
use crate::core::dates::date::JsDate;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::commodity::commodity_forward::{
    CommodityForward, SettlementType,
};
use finstack_valuations::instruments::Attributes;
use finstack_valuations::instruments::CommodityUnderlyingParams;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = CommodityForwardBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsCommodityForwardBuilder {
    instrument_id: String,
    commodity_type: Option<String>,
    ticker: Option<String>,
    quantity: Option<f64>,
    unit: Option<String>,
    maturity: Option<finstack_core::dates::Date>,
    currency: Option<finstack_core::currency::Currency>,
    forward_curve_id: Option<String>,
    discount_curve_id: Option<String>,
    multiplier: Option<f64>,
    quoted_price: Option<f64>,
    settlement: Option<String>,
}

#[wasm_bindgen(js_class = CommodityForwardBuilder)]
impl JsCommodityForwardBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsCommodityForwardBuilder {
        JsCommodityForwardBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = commodityType)]
    pub fn commodity_type(mut self, commodity_type: String) -> JsCommodityForwardBuilder {
        self.commodity_type = Some(commodity_type);
        self
    }

    #[wasm_bindgen(js_name = ticker)]
    pub fn ticker(mut self, ticker: String) -> JsCommodityForwardBuilder {
        self.ticker = Some(ticker);
        self
    }

    #[wasm_bindgen(js_name = quantity)]
    pub fn quantity(mut self, quantity: f64) -> JsCommodityForwardBuilder {
        self.quantity = Some(quantity);
        self
    }

    #[wasm_bindgen(js_name = unit)]
    pub fn unit(mut self, unit: String) -> JsCommodityForwardBuilder {
        self.unit = Some(unit);
        self
    }

    #[wasm_bindgen(js_name = settlementDate)]
    pub fn maturity(mut self, maturity: &JsDate) -> JsCommodityForwardBuilder {
        self.maturity = Some(maturity.inner());
        self
    }

    #[wasm_bindgen(js_name = currency)]
    pub fn currency(mut self, currency: &JsCurrency) -> JsCommodityForwardBuilder {
        self.currency = Some(currency.inner());
        self
    }

    #[wasm_bindgen(js_name = forwardCurveId)]
    pub fn forward_curve_id(mut self, forward_curve_id: &str) -> JsCommodityForwardBuilder {
        self.forward_curve_id = Some(forward_curve_id.to_string());
        self
    }

    #[wasm_bindgen(js_name = discountCurveId)]
    pub fn discount_curve_id(mut self, discount_curve_id: &str) -> JsCommodityForwardBuilder {
        self.discount_curve_id = Some(discount_curve_id.to_string());
        self
    }

    #[wasm_bindgen(js_name = multiplier)]
    pub fn multiplier(mut self, multiplier: f64) -> JsCommodityForwardBuilder {
        self.multiplier = Some(multiplier);
        self
    }

    #[wasm_bindgen(js_name = quotedPrice)]
    pub fn quoted_price(mut self, quoted_price: f64) -> JsCommodityForwardBuilder {
        self.quoted_price = Some(quoted_price);
        self
    }

    #[wasm_bindgen(js_name = settlementType)]
    pub fn settlement_type(mut self, settlement_type: String) -> JsCommodityForwardBuilder {
        self.settlement = Some(settlement_type);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsCommodityForward, JsValue> {
        let commodity_type = self.commodity_type.as_deref().ok_or_else(|| {
            JsValue::from_str("CommodityForwardBuilder: commodityType is required")
        })?;
        let ticker = self
            .ticker
            .as_deref()
            .ok_or_else(|| JsValue::from_str("CommodityForwardBuilder: ticker is required"))?;
        let quantity = self
            .quantity
            .ok_or_else(|| JsValue::from_str("CommodityForwardBuilder: quantity is required"))?;
        let unit = self
            .unit
            .as_deref()
            .ok_or_else(|| JsValue::from_str("CommodityForwardBuilder: unit is required"))?;
        let maturity = self
            .maturity
            .ok_or_else(|| JsValue::from_str("CommodityForwardBuilder: maturity is required"))?;
        let currency = self
            .currency
            .ok_or_else(|| JsValue::from_str("CommodityForwardBuilder: currency is required"))?;
        let forward_curve_id = self.forward_curve_id.as_deref().ok_or_else(|| {
            JsValue::from_str("CommodityForwardBuilder: forwardCurveId is required")
        })?;
        let discount_curve_id = self.discount_curve_id.as_deref().ok_or_else(|| {
            JsValue::from_str("CommodityForwardBuilder: discountCurveId is required")
        })?;

        let settlement_type_enum = match self.settlement.as_deref() {
            Some("physical") | Some("Physical") => Some(SettlementType::Physical),
            Some("cash") | Some("Cash") => Some(SettlementType::Cash),
            None => None,
            Some(other) => {
                return Err(JsValue::from_str(&format!(
                    "Invalid settlement_type: '{}'. Must be 'physical' or 'cash'",
                    other
                )));
            }
        };

        let mut builder = CommodityForward::builder()
            .id(InstrumentId::new(&self.instrument_id))
            .underlying(CommodityUnderlyingParams::new(
                commodity_type,
                ticker,
                unit,
                currency,
            ))
            .quantity(quantity)
            .multiplier(self.multiplier.unwrap_or(1.0))
            .maturity(maturity)
            .forward_curve_id(CurveId::new(forward_curve_id))
            .discount_curve_id(CurveId::new(discount_curve_id))
            .attributes(Attributes::new());

        if let Some(st) = settlement_type_enum {
            builder = builder.settlement_opt(Some(st));
        }
        if let Some(qp) = self.quoted_price {
            builder = builder.quoted_price_opt(Some(qp));
        }

        let forward = builder
            .build()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(JsCommodityForward::from_inner(forward))
    }
}

/// JavaScript representation of a commodity forward contract.
#[wasm_bindgen(js_name = CommodityForward)]
#[derive(Clone, Debug)]
pub struct JsCommodityForward {
    pub(crate) inner: CommodityForward,
}

impl InstrumentWrapper for JsCommodityForward {
    type Inner = CommodityForward;
    fn from_inner(inner: CommodityForward) -> Self {
        JsCommodityForward { inner }
    }
    fn inner(&self) -> CommodityForward {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = CommodityForward)]
impl JsCommodityForward {
    /// Create a new commodity forward.
    ///
    /// @param instrumentId - Unique identifier
    /// @param commodityType - Commodity category (e.g., "Energy", "Metal")
    /// @param ticker - Commodity symbol (e.g., "CL", "GC")
    /// @param quantity - Contract quantity
    /// @param unit - Unit of measure (e.g., "BBL", "OZ")
    /// @param settlementDate - Settlement date
    /// @param currency - Contract currency
    /// @param forwardCurveId - Forward/futures curve ID
    /// @param discountCurveId - Discount curve ID
    /// @param multiplier - Contract multiplier (default: 1.0)
    /// @param quotedPrice - Optional quoted forward price
    /// @param settlementType - "physical" or "cash"
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        commodity_type: &str,
        ticker: &str,
        quantity: f64,
        unit: &str,
        settlement_date: &JsDate,
        currency: &JsCurrency,
        forward_curve_id: &str,
        discount_curve_id: &str,
        multiplier: Option<f64>,
        quoted_price: Option<f64>,
        settlement_type: Option<String>,
    ) -> Result<JsCommodityForward, JsValue> {
        web_sys::console::warn_1(&JsValue::from_str(
            "CommodityForward constructor is deprecated; use CommodityForwardBuilder instead.",
        ));
        let settlement_type_enum = match settlement_type.as_deref() {
            Some("physical") | Some("Physical") => Some(SettlementType::Physical),
            Some("cash") | Some("Cash") => Some(SettlementType::Cash),
            None => None,
            Some(other) => {
                return Err(JsValue::from_str(&format!(
                    "Invalid settlement_type: '{}'. Must be 'physical' or 'cash'",
                    other
                )));
            }
        };

        let mut builder = CommodityForward::builder()
            .id(InstrumentId::new(instrument_id))
            .underlying(CommodityUnderlyingParams::new(
                commodity_type,
                ticker,
                unit,
                currency.inner(),
            ))
            .quantity(quantity)
            .multiplier(multiplier.unwrap_or(1.0))
            .maturity(settlement_date.inner())
            .forward_curve_id(CurveId::new(forward_curve_id))
            .discount_curve_id(CurveId::new(discount_curve_id))
            .attributes(Attributes::new());

        if let Some(st) = settlement_type_enum {
            builder = builder.settlement_opt(Some(st));
        }
        if let Some(qp) = quoted_price {
            builder = builder.quoted_price_opt(Some(qp));
        }

        let forward = builder
            .build()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(JsCommodityForward::from_inner(forward))
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = commodityType)]
    pub fn commodity_type(&self) -> String {
        self.inner.underlying.commodity_type.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn ticker(&self) -> String {
        self.inner.underlying.ticker.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn quantity(&self) -> f64 {
        self.inner.quantity
    }

    #[wasm_bindgen(getter)]
    pub fn unit(&self) -> String {
        self.inner.underlying.unit.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn multiplier(&self) -> f64 {
        self.inner.multiplier
    }

    #[wasm_bindgen(getter, js_name = settlementDate)]
    pub fn settlement_date(&self) -> JsDate {
        JsDate::from_core(self.inner.maturity)
    }

    #[wasm_bindgen(getter)]
    pub fn currency(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.underlying.currency)
    }

    #[wasm_bindgen(getter, js_name = quotedPrice)]
    pub fn quoted_price(&self) -> Option<f64> {
        self.inner.quoted_price
    }

    #[wasm_bindgen(getter, js_name = forwardCurveId)]
    pub fn forward_curve_id(&self) -> String {
        self.inner.forward_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = discountCurveId)]
    pub fn discount_curve_id(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn exchange(&self) -> Option<String> {
        self.inner.exchange.clone()
    }

    #[wasm_bindgen(getter, js_name = contractMonth)]
    pub fn contract_month(&self) -> Option<String> {
        self.inner.contract_month.clone()
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsCommodityForward, JsValue> {
        from_js_value(value).map(JsCommodityForward::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::CommodityForward as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "CommodityForward(id='{}', ticker='{}', quantity={}, settlement='{}')",
            self.inner.id.as_str(),
            self.inner.underlying.ticker,
            self.inner.quantity,
            self.inner.maturity
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsCommodityForward {
        JsCommodityForward::from_inner(self.inner.clone())
    }
}
