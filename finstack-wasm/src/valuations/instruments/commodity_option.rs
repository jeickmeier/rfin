//! Commodity Option WASM bindings.

use crate::core::dates::FsDate;
use crate::core::market_data::context::JsMarketContext;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::parameters::{JsExerciseStyle, JsOptionType, JsSettlementType};
use finstack_core::currency::Currency;
use finstack_core::dates::DayCount;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::commodity_option::CommodityOption;
use finstack_valuations::instruments::common::traits::Attributes;
use finstack_valuations::instruments::PricingOverrides;
use wasm_bindgen::prelude::*;

/// Commodity option instrument.
///
/// European or American option on a commodity (WTI, Gold, Natural Gas, etc.).
/// Priced using Black-76 model for European options or binomial tree for American.
///
/// @example
/// ```javascript
/// const option = new CommodityOption(
///   "WTI-CALL-DEC25",
///   "Energy",                     // Commodity type
///   "CL",                         // Ticker (WTI crude)
///   80.0,                         // Strike price
///   OptionType.Call(),
///   ExerciseStyle.European(),
///   new FsDate(2025, 12, 15),    // Expiry
///   1000,                         // Quantity (barrels)
///   "BBL",                        // Unit
///   "USD",
///   "WTI-FORWARD",               // Forward curve ID
///   "USD-OIS",                   // Discount curve ID
///   "WTI-VOL"                    // Vol surface ID
/// );
/// ```
#[wasm_bindgen(js_name = CommodityOption)]
#[derive(Clone)]
pub struct JsCommodityOption {
    inner: CommodityOption,
}

impl JsCommodityOption {
    pub(crate) fn inner(&self) -> CommodityOption {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = CommodityOption)]
impl JsCommodityOption {
    /// Create a new commodity option.
    ///
    /// @param {string} id - Instrument identifier
    /// @param {string} commodityType - Commodity type (e.g., "Energy", "Metal")
    /// @param {string} ticker - Ticker symbol (e.g., "CL" for WTI, "GC" for Gold)
    /// @param {number} strike - Strike price per unit
    /// @param {OptionType} optionType - Call or Put
    /// @param {ExerciseStyle} exerciseStyle - European or American
    /// @param {FsDate} expiry - Option expiry date
    /// @param {number} quantity - Contract quantity in units
    /// @param {string} unit - Unit of measurement (e.g., "BBL", "MT", "OZ")
    /// @param {string} currency - Currency code (e.g., "USD")
    /// @param {string} forwardCurveId - Forward curve ID for price interpolation
    /// @param {string} discountCurveId - Discount curve ID
    /// @param {string} volSurfaceId - Volatility surface ID
    #[wasm_bindgen(constructor)]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &str,
        commodity_type: &str,
        ticker: &str,
        strike: f64,
        option_type: &JsOptionType,
        exercise_style: &JsExerciseStyle,
        expiry: &FsDate,
        quantity: f64,
        unit: &str,
        currency: &str,
        forward_curve_id: &str,
        discount_curve_id: &str,
        vol_surface_id: &str,
    ) -> Result<JsCommodityOption, JsValue> {
        use finstack_valuations::instruments::SettlementType;

        let ccy: Currency = currency
            .parse()
            .map_err(|e: strum::ParseError| JsValue::from_str(&e.to_string()))?;

        let option = CommodityOption::builder()
            .id(InstrumentId::new(id))
            .commodity_type(commodity_type.to_string())
            .ticker(ticker.to_string())
            .strike(strike)
            .option_type(option_type.inner())
            .exercise_style(exercise_style.inner())
            .expiry(expiry.inner())
            .quantity(quantity)
            .unit(unit.to_string())
            .multiplier(1.0)
            .settlement(SettlementType::Cash)
            .currency(ccy)
            .forward_curve_id(CurveId::new(forward_curve_id))
            .discount_curve_id(CurveId::new(discount_curve_id))
            .vol_surface_id(CurveId::new(vol_surface_id))
            .day_count(DayCount::Act365F)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(JsCommodityOption { inner: option })
    }

    /// Get the instrument ID.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    /// Get the strike price.
    #[wasm_bindgen(getter)]
    pub fn strike(&self) -> f64 {
        self.inner.strike
    }

    /// Get the ticker.
    #[wasm_bindgen(getter)]
    pub fn ticker(&self) -> String {
        self.inner.ticker.clone()
    }

    /// Get the commodity type.
    #[wasm_bindgen(getter, js_name = commodityType)]
    pub fn commodity_type(&self) -> String {
        self.inner.commodity_type.clone()
    }

    /// Get the quantity.
    #[wasm_bindgen(getter)]
    pub fn quantity(&self) -> f64 {
        self.inner.quantity
    }

    /// Set the quoted forward price (overrides curve lookup).
    #[wasm_bindgen(js_name = setQuotedForward)]
    pub fn set_quoted_forward(&mut self, price: f64) {
        self.inner.quoted_forward = Some(price);
    }

    /// Set the settlement type.
    #[wasm_bindgen(js_name = setSettlement)]
    pub fn set_settlement(&mut self, settlement: &JsSettlementType) {
        self.inner.settlement = settlement.inner();
    }

    /// Calculate present value.
    pub fn npv(&self, market: &JsMarketContext, as_of: &FsDate) -> Result<f64, JsValue> {
        self.inner
            .npv(&market.inner(), as_of.inner())
            .map(|m| m.amount())
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsCommodityOption, JsValue> {
        from_js_value(value).map(|inner| JsCommodityOption { inner })
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}
