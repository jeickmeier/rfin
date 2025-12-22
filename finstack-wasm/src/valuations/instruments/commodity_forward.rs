//! WASM bindings for CommodityForward instrument.

use crate::core::currency::JsCurrency;
use crate::core::dates::date::JsDate;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::commodity_forward::{CommodityForward, SettlementType};
use finstack_valuations::instruments::common::traits::Attributes;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

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
            .commodity_type(commodity_type.to_string())
            .ticker(ticker.to_string())
            .quantity(quantity)
            .unit(unit.to_string())
            .multiplier(multiplier.unwrap_or(1.0))
            .settlement_date(settlement_date.inner())
            .currency(currency.inner())
            .forward_curve_id(CurveId::new(forward_curve_id))
            .discount_curve_id(CurveId::new(discount_curve_id))
            .attributes(Attributes::new());

        if let Some(st) = settlement_type_enum {
            builder = builder.settlement_type_opt(Some(st));
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
        self.inner.commodity_type.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn ticker(&self) -> String {
        self.inner.ticker.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn quantity(&self) -> f64 {
        self.inner.quantity
    }

    #[wasm_bindgen(getter)]
    pub fn unit(&self) -> String {
        self.inner.unit.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn multiplier(&self) -> f64 {
        self.inner.multiplier
    }

    #[wasm_bindgen(getter, js_name = settlementDate)]
    pub fn settlement_date(&self) -> JsDate {
        JsDate::from_core(self.inner.settlement_date)
    }

    #[wasm_bindgen(getter)]
    pub fn currency(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.currency)
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

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::CommodityForward as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "CommodityForward(id='{}', ticker='{}', quantity={}, settlement='{}')",
            self.inner.id.as_str(),
            self.inner.ticker,
            self.inner.quantity,
            self.inner.settlement_date
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsCommodityForward {
        JsCommodityForward::from_inner(self.inner.clone())
    }
}
