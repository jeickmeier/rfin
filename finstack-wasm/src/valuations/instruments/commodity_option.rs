//! Commodity Option WASM bindings.

use crate::core::dates::FsDate;
use crate::core::market_data::context::JsMarketContext;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::parameters::{JsExerciseStyle, JsOptionType, JsSettlementType};
use finstack_core::currency::Currency;
use finstack_core::dates::DayCount;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::commodity::commodity_option::CommodityOption;
use finstack_valuations::instruments::Attributes;
use finstack_valuations::instruments::CommodityUnderlyingParams;
use finstack_valuations::instruments::PricingOverrides;
use finstack_valuations::prelude::Instrument;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = CommodityOptionBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsCommodityOptionBuilder {
    instrument_id: String,
    commodity_type: Option<String>,
    ticker: Option<String>,
    strike: Option<f64>,
    option_type: Option<finstack_valuations::instruments::OptionType>,
    exercise_style: Option<finstack_valuations::instruments::ExerciseStyle>,
    expiry: Option<finstack_core::dates::Date>,
    quantity: Option<f64>,
    unit: Option<String>,
    currency: Option<Currency>,
    forward_curve_id: Option<String>,
    discount_curve_id: Option<String>,
    vol_surface_id: Option<String>,
}

#[wasm_bindgen(js_class = CommodityOptionBuilder)]
impl JsCommodityOptionBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsCommodityOptionBuilder {
        JsCommodityOptionBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = commodityType)]
    pub fn commodity_type(mut self, commodity_type: String) -> JsCommodityOptionBuilder {
        self.commodity_type = Some(commodity_type);
        self
    }

    #[wasm_bindgen(js_name = ticker)]
    pub fn ticker(mut self, ticker: String) -> JsCommodityOptionBuilder {
        self.ticker = Some(ticker);
        self
    }

    #[wasm_bindgen(js_name = strike)]
    pub fn strike(mut self, strike: f64) -> JsCommodityOptionBuilder {
        self.strike = Some(strike);
        self
    }

    #[wasm_bindgen(js_name = optionType)]
    pub fn option_type(mut self, option_type: &JsOptionType) -> JsCommodityOptionBuilder {
        self.option_type = Some(option_type.inner());
        self
    }

    #[wasm_bindgen(js_name = exerciseStyle)]
    pub fn exercise_style(mut self, exercise_style: &JsExerciseStyle) -> JsCommodityOptionBuilder {
        self.exercise_style = Some(exercise_style.inner());
        self
    }

    #[wasm_bindgen(js_name = expiry)]
    pub fn expiry(mut self, expiry: &FsDate) -> JsCommodityOptionBuilder {
        self.expiry = Some(expiry.inner());
        self
    }

    #[wasm_bindgen(js_name = quantity)]
    pub fn quantity(mut self, quantity: f64) -> JsCommodityOptionBuilder {
        self.quantity = Some(quantity);
        self
    }

    #[wasm_bindgen(js_name = unit)]
    pub fn unit(mut self, unit: String) -> JsCommodityOptionBuilder {
        self.unit = Some(unit);
        self
    }

    #[wasm_bindgen(js_name = currency)]
    pub fn currency(mut self, currency: String) -> Result<JsCommodityOptionBuilder, JsValue> {
        let ccy: Currency = currency
            .parse()
            .map_err(|e: strum::ParseError| JsValue::from_str(&e.to_string()))?;
        self.currency = Some(ccy);
        Ok(self)
    }

    #[wasm_bindgen(js_name = forwardCurveId)]
    pub fn forward_curve_id(mut self, forward_curve_id: &str) -> JsCommodityOptionBuilder {
        self.forward_curve_id = Some(forward_curve_id.to_string());
        self
    }

    #[wasm_bindgen(js_name = discountCurveId)]
    pub fn discount_curve_id(mut self, discount_curve_id: &str) -> JsCommodityOptionBuilder {
        self.discount_curve_id = Some(discount_curve_id.to_string());
        self
    }

    #[wasm_bindgen(js_name = volSurfaceId)]
    pub fn vol_surface_id(mut self, vol_surface_id: &str) -> JsCommodityOptionBuilder {
        self.vol_surface_id = Some(vol_surface_id.to_string());
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsCommodityOption, JsValue> {
        use finstack_valuations::instruments::SettlementType;

        let commodity_type = self.commodity_type.as_deref().ok_or_else(|| {
            JsValue::from_str("CommodityOptionBuilder: commodityType is required")
        })?;
        let ticker = self
            .ticker
            .as_deref()
            .ok_or_else(|| JsValue::from_str("CommodityOptionBuilder: ticker is required"))?;
        let strike = self
            .strike
            .ok_or_else(|| JsValue::from_str("CommodityOptionBuilder: strike is required"))?;
        let option_type = self
            .option_type
            .ok_or_else(|| JsValue::from_str("CommodityOptionBuilder: optionType is required"))?;
        let exercise_style = self.exercise_style.ok_or_else(|| {
            JsValue::from_str("CommodityOptionBuilder: exerciseStyle is required")
        })?;
        let expiry = self
            .expiry
            .ok_or_else(|| JsValue::from_str("CommodityOptionBuilder: expiry is required"))?;
        let quantity = self
            .quantity
            .ok_or_else(|| JsValue::from_str("CommodityOptionBuilder: quantity is required"))?;
        let unit = self
            .unit
            .as_deref()
            .ok_or_else(|| JsValue::from_str("CommodityOptionBuilder: unit is required"))?;
        let ccy = self
            .currency
            .ok_or_else(|| JsValue::from_str("CommodityOptionBuilder: currency is required"))?;
        let forward_curve_id = self.forward_curve_id.as_deref().ok_or_else(|| {
            JsValue::from_str("CommodityOptionBuilder: forwardCurveId is required")
        })?;
        let discount_curve_id = self.discount_curve_id.as_deref().ok_or_else(|| {
            JsValue::from_str("CommodityOptionBuilder: discountCurveId is required")
        })?;
        let vol_surface_id = self
            .vol_surface_id
            .as_deref()
            .ok_or_else(|| JsValue::from_str("CommodityOptionBuilder: volSurfaceId is required"))?;

        let option = CommodityOption::builder()
            .id(InstrumentId::new(&self.instrument_id))
            .underlying(CommodityUnderlyingParams::new(
                commodity_type,
                ticker,
                unit,
                ccy,
            ))
            .strike(strike)
            .option_type(option_type)
            .exercise_style(exercise_style)
            .expiry(expiry)
            .quantity(quantity)
            .multiplier(1.0)
            .settlement(SettlementType::Cash)
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
}

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
        self.inner.underlying.ticker.clone()
    }

    /// Get the commodity type.
    #[wasm_bindgen(getter, js_name = commodityType)]
    pub fn commodity_type(&self) -> String {
        self.inner.underlying.commodity_type.clone()
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
    #[wasm_bindgen(js_name = value)]
    pub fn value(&self, market: &JsMarketContext, as_of: &FsDate) -> Result<f64, JsValue> {
        self.inner
            .value(market.inner(), as_of.inner())
            .map(|m| m.amount())
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsCommodityOption, JsValue> {
        from_js_value(value).map(|inner| JsCommodityOption { inner })
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }
}
