//! WASM bindings for CommoditySwap instrument.

use crate::core::currency::JsCurrency;
use crate::core::dates::date::JsDate;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::dates::{BusinessDayConvention, Tenor, TenorUnit};
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::commodity_swap::CommoditySwap;
use finstack_valuations::instruments::common::traits::Attributes;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

/// JavaScript representation of a commodity swap.
#[wasm_bindgen(js_name = CommoditySwap)]
#[derive(Clone, Debug)]
pub struct JsCommoditySwap {
    pub(crate) inner: CommoditySwap,
}

impl InstrumentWrapper for JsCommoditySwap {
    type Inner = CommoditySwap;
    fn from_inner(inner: CommoditySwap) -> Self {
        JsCommoditySwap { inner }
    }
    fn inner(&self) -> CommoditySwap {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = CommoditySwap)]
impl JsCommoditySwap {
    /// Create a new commodity swap.
    ///
    /// @param instrumentId - Unique identifier
    /// @param commodityType - Commodity category (e.g., "Energy", "Metal")
    /// @param ticker - Commodity symbol (e.g., "CL", "NG")
    /// @param unit - Unit of measure (e.g., "BBL", "MMBTU")
    /// @param currency - Contract currency
    /// @param notionalQuantity - Notional quantity per period
    /// @param fixedPrice - Fixed price per unit
    /// @param floatingIndexId - Floating index curve ID
    /// @param payFixed - True if paying fixed
    /// @param startDate - Start date
    /// @param endDate - End date
    /// @param paymentFrequency - Payment frequency (e.g., "1M", "3M")
    /// @param discountCurveId - Discount curve ID
    /// @param bdc - Business day convention
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instrument_id: &str,
        commodity_type: &str,
        ticker: &str,
        unit: &str,
        currency: &JsCurrency,
        notional_quantity: f64,
        fixed_price: f64,
        floating_index_id: &str,
        pay_fixed: bool,
        start_date: &JsDate,
        end_date: &JsDate,
        payment_frequency: &str,
        discount_curve_id: &str,
        bdc: Option<String>,
    ) -> Result<JsCommoditySwap, JsValue> {
        let freq = parse_tenor(payment_frequency)
            .map_err(|e| JsValue::from_str(&format!("Invalid payment_frequency: {}", e)))?;

        let bdc_enum = match bdc.as_deref() {
            Some("following") | Some("Following") => Some(BusinessDayConvention::Following),
            Some("modified_following") | Some("ModifiedFollowing") | Some("modifiedFollowing") => {
                Some(BusinessDayConvention::ModifiedFollowing)
            }
            Some("preceding") | Some("Preceding") => Some(BusinessDayConvention::Preceding),
            Some("modified_preceding") | Some("ModifiedPreceding") | Some("modifiedPreceding") => {
                Some(BusinessDayConvention::ModifiedPreceding)
            }
            Some("unadjusted") | Some("Unadjusted") | Some("none") | Some("None") => {
                Some(BusinessDayConvention::Unadjusted)
            }
            None => None,
            Some(other) => {
                return Err(JsValue::from_str(&format!(
                    "Invalid bdc: '{}'. Must be 'following', 'modifiedFollowing', etc.",
                    other
                )));
            }
        };

        let mut builder = CommoditySwap::builder()
            .id(InstrumentId::new(instrument_id))
            .commodity_type(commodity_type.to_string())
            .ticker(ticker.to_string())
            .unit(unit.to_string())
            .currency(currency.inner())
            .notional_quantity(notional_quantity)
            .fixed_price(fixed_price)
            .floating_index_id(CurveId::new(floating_index_id))
            .pay_fixed(pay_fixed)
            .start_date(start_date.inner())
            .end_date(end_date.inner())
            .payment_frequency(freq)
            .discount_curve_id(CurveId::new(discount_curve_id))
            .attributes(Attributes::new());

        if let Some(b) = bdc_enum {
            builder = builder.bdc_opt(Some(b));
        }

        let swap = builder
            .build()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(JsCommoditySwap::from_inner(swap))
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
    pub fn unit(&self) -> String {
        self.inner.unit.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn currency(&self) -> JsCurrency {
        JsCurrency::from_inner(self.inner.currency)
    }

    #[wasm_bindgen(getter, js_name = notionalQuantity)]
    pub fn notional_quantity(&self) -> f64 {
        self.inner.notional_quantity
    }

    #[wasm_bindgen(getter, js_name = fixedPrice)]
    pub fn fixed_price(&self) -> f64 {
        self.inner.fixed_price
    }

    #[wasm_bindgen(getter, js_name = payFixed)]
    pub fn pay_fixed(&self) -> bool {
        self.inner.pay_fixed
    }

    #[wasm_bindgen(getter, js_name = startDate)]
    pub fn start_date(&self) -> JsDate {
        JsDate::from_core(self.inner.start_date)
    }

    #[wasm_bindgen(getter, js_name = endDate)]
    pub fn end_date(&self) -> JsDate {
        JsDate::from_core(self.inner.end_date)
    }

    #[wasm_bindgen(getter, js_name = floatingIndexId)]
    pub fn floating_index_id(&self) -> String {
        self.inner.floating_index_id.as_str().to_string()
    }

    #[wasm_bindgen(getter, js_name = discountCurveId)]
    pub fn discount_curve_id(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> u16 {
        InstrumentType::CommoditySwap as u16
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "CommoditySwap(id='{}', ticker='{}', fixed_price={}, pay_fixed={})",
            self.inner.id.as_str(),
            self.inner.ticker,
            self.inner.fixed_price,
            self.inner.pay_fixed
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsCommoditySwap {
        JsCommoditySwap::from_inner(self.inner.clone())
    }
}

/// Parse a tenor string like "1M", "3M", "1Y" into a Tenor.
fn parse_tenor(s: &str) -> Result<Tenor, String> {
    let s = s.trim().to_uppercase();
    if s.is_empty() {
        return Err("Empty tenor string".to_string());
    }

    let unit_start = s.find(|c: char| c.is_alphabetic()).ok_or("No unit found")?;
    let count_str = &s[..unit_start];
    let unit_str = &s[unit_start..];

    let count: u32 = count_str
        .parse()
        .map_err(|_| format!("Invalid count: {}", count_str))?;

    let unit = match unit_str {
        "D" => TenorUnit::Days,
        "W" => TenorUnit::Weeks,
        "M" => TenorUnit::Months,
        "Y" => TenorUnit::Years,
        _ => return Err(format!("Unknown unit: {}", unit_str)),
    };

    Ok(Tenor::new(count, unit))
}

