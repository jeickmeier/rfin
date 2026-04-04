//! WASM bindings for CommodityAsianOption instrument.

use super::asian_option::JsAveragingMethod;
use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::parameters::JsOptionType;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::currency::Currency;
use finstack_core::dates::DayCount;
use finstack_valuations::instruments::commodity::commodity_asian_option::CommodityAsianOption;
use finstack_valuations::instruments::{Attributes, CommodityUnderlyingParams, PricingOverrides};
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

/// Builder for commodity Asian options.
#[wasm_bindgen(js_name = CommodityAsianOptionBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsCommodityAsianOptionBuilder {
    /// Instrument identifier string.
    instrument_id: String,
    /// Commodity type (e.g., "Energy").
    commodity_type: Option<String>,
    /// Ticker symbol (e.g., "CL").
    ticker: Option<String>,
    /// Unit of measure (e.g., "BBL").
    unit: Option<String>,
    /// Settlement currency.
    currency: Option<Currency>,
    /// Strike price per unit.
    strike: Option<f64>,
    /// Option type (call/put).
    option_type: Option<finstack_valuations::instruments::OptionType>,
    /// Averaging method (arithmetic/geometric).
    averaging_method: Option<JsAveragingMethod>,
    /// Fixing dates for price observations.
    fixing_dates: Vec<finstack_core::dates::Date>,
    /// Realized fixings for seasoned options.
    realized_fixings: Vec<(finstack_core::dates::Date, f64)>,
    /// Contract quantity.
    quantity: Option<f64>,
    /// Option expiry date.
    expiry: Option<finstack_core::dates::Date>,
    /// Forward curve ID.
    forward_curve_id: Option<String>,
    /// Discount curve ID.
    discount_curve_id: Option<String>,
    /// Vol surface ID.
    vol_surface_id: Option<String>,
}

#[wasm_bindgen(js_class = CommodityAsianOptionBuilder)]
impl JsCommodityAsianOptionBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsCommodityAsianOptionBuilder {
        JsCommodityAsianOptionBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = commodityType)]
    pub fn commodity_type(mut self, commodity_type: String) -> JsCommodityAsianOptionBuilder {
        self.commodity_type = Some(commodity_type);
        self
    }

    #[wasm_bindgen(js_name = ticker)]
    pub fn ticker(mut self, ticker: String) -> JsCommodityAsianOptionBuilder {
        self.ticker = Some(ticker);
        self
    }

    #[wasm_bindgen(js_name = unit)]
    pub fn unit(mut self, unit: String) -> JsCommodityAsianOptionBuilder {
        self.unit = Some(unit);
        self
    }

    #[wasm_bindgen(js_name = currency)]
    pub fn currency(mut self, currency: String) -> Result<JsCommodityAsianOptionBuilder, JsValue> {
        let ccy: Currency = currency
            .parse()
            .map_err(|e: strum::ParseError| js_error(e.to_string()))?;
        self.currency = Some(ccy);
        Ok(self)
    }

    #[wasm_bindgen(js_name = strike)]
    pub fn strike(mut self, strike: f64) -> JsCommodityAsianOptionBuilder {
        self.strike = Some(strike);
        self
    }

    #[wasm_bindgen(js_name = optionType)]
    pub fn option_type(mut self, option_type: &JsOptionType) -> JsCommodityAsianOptionBuilder {
        self.option_type = Some(option_type.inner());
        self
    }

    #[wasm_bindgen(js_name = averagingMethod)]
    pub fn averaging_method(
        mut self,
        averaging_method: JsAveragingMethod,
    ) -> JsCommodityAsianOptionBuilder {
        self.averaging_method = Some(averaging_method);
        self
    }

    /// Add a single fixing date.
    #[wasm_bindgen(js_name = addFixingDate)]
    pub fn add_fixing_date(mut self, date: &JsDate) -> JsCommodityAsianOptionBuilder {
        self.fixing_dates.push(date.inner());
        self
    }

    /// Add a realized fixing (date, price) for seasoned options.
    #[wasm_bindgen(js_name = addRealizedFixing)]
    pub fn add_realized_fixing(
        mut self,
        date: &JsDate,
        price: f64,
    ) -> JsCommodityAsianOptionBuilder {
        self.realized_fixings.push((date.inner(), price));
        self
    }

    #[wasm_bindgen(js_name = quantity)]
    pub fn quantity(mut self, quantity: f64) -> JsCommodityAsianOptionBuilder {
        self.quantity = Some(quantity);
        self
    }

    #[wasm_bindgen(js_name = expiry)]
    pub fn expiry(mut self, expiry: &JsDate) -> JsCommodityAsianOptionBuilder {
        self.expiry = Some(expiry.inner());
        self
    }

    #[wasm_bindgen(js_name = forwardCurveId)]
    pub fn forward_curve_id(mut self, id: &str) -> JsCommodityAsianOptionBuilder {
        self.forward_curve_id = Some(id.to_string());
        self
    }

    #[wasm_bindgen(js_name = discountCurveId)]
    pub fn discount_curve_id(mut self, id: &str) -> JsCommodityAsianOptionBuilder {
        self.discount_curve_id = Some(id.to_string());
        self
    }

    #[wasm_bindgen(js_name = volSurfaceId)]
    pub fn vol_surface_id(mut self, id: &str) -> JsCommodityAsianOptionBuilder {
        self.vol_surface_id = Some(id.to_string());
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsCommodityAsianOption, JsValue> {
        let commodity_type = self.commodity_type.as_deref().ok_or_else(|| {
            js_error("CommodityAsianOptionBuilder: commodityType is required".to_string())
        })?;
        let ticker = self.ticker.as_deref().ok_or_else(|| {
            js_error("CommodityAsianOptionBuilder: ticker is required".to_string())
        })?;
        let unit = self
            .unit
            .as_deref()
            .ok_or_else(|| js_error("CommodityAsianOptionBuilder: unit is required".to_string()))?;
        let ccy = self.currency.ok_or_else(|| {
            js_error("CommodityAsianOptionBuilder: currency is required".to_string())
        })?;
        let strike = self.strike.ok_or_else(|| {
            js_error("CommodityAsianOptionBuilder: strike is required".to_string())
        })?;
        let option_type = self.option_type.ok_or_else(|| {
            js_error("CommodityAsianOptionBuilder: optionType is required".to_string())
        })?;
        let averaging_method_js = self.averaging_method.ok_or_else(|| {
            js_error("CommodityAsianOptionBuilder: averagingMethod is required".to_string())
        })?;
        let averaging_method: finstack_valuations::instruments::exotics::asian_option::AveragingMethod = averaging_method_js.into();
        let quantity = self.quantity.ok_or_else(|| {
            js_error("CommodityAsianOptionBuilder: quantity is required".to_string())
        })?;
        let expiry = self.expiry.ok_or_else(|| {
            js_error("CommodityAsianOptionBuilder: expiry is required".to_string())
        })?;
        let forward_curve_id = self.forward_curve_id.as_deref().ok_or_else(|| {
            js_error("CommodityAsianOptionBuilder: forwardCurveId is required".to_string())
        })?;
        let discount_curve_id = self.discount_curve_id.as_deref().ok_or_else(|| {
            js_error("CommodityAsianOptionBuilder: discountCurveId is required".to_string())
        })?;
        let vol_surface_id = self.vol_surface_id.as_deref().ok_or_else(|| {
            js_error("CommodityAsianOptionBuilder: volSurfaceId is required".to_string())
        })?;

        if self.fixing_dates.is_empty() {
            return Err(js_error(
                "CommodityAsianOptionBuilder: at least one fixingDate is required".to_string(),
            ));
        }

        CommodityAsianOption::builder()
            .id(instrument_id_from_str(&self.instrument_id))
            .underlying(CommodityUnderlyingParams::new(
                commodity_type,
                ticker,
                unit,
                ccy,
            ))
            .strike(strike)
            .option_type(option_type)
            .averaging_method(averaging_method)
            .fixing_dates(self.fixing_dates)
            .realized_fixings(self.realized_fixings)
            .quantity(quantity)
            .expiry(expiry)
            .forward_curve_id(curve_id_from_str(forward_curve_id))
            .discount_curve_id(curve_id_from_str(discount_curve_id))
            .vol_surface_id(curve_id_from_str(vol_surface_id))
            .day_count(DayCount::Act365F)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .map(JsCommodityAsianOption::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }
}

/// Commodity Asian option instrument (option on the average commodity price).
#[wasm_bindgen(js_name = CommodityAsianOption)]
#[derive(Clone, Debug)]
pub struct JsCommodityAsianOption {
    pub(crate) inner: CommodityAsianOption,
}

impl InstrumentWrapper for JsCommodityAsianOption {
    type Inner = CommodityAsianOption;
    fn from_inner(inner: CommodityAsianOption) -> Self {
        JsCommodityAsianOption { inner }
    }
    fn inner(&self) -> CommodityAsianOption {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = CommodityAsianOption)]
impl JsCommodityAsianOption {
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsCommodityAsianOption, JsValue> {
        from_js_value(value).map(JsCommodityAsianOption::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
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

    /// Get the quantity.
    #[wasm_bindgen(getter)]
    pub fn quantity(&self) -> f64 {
        self.inner.quantity
    }

    /// Get a cashflow view (Asian options return an empty schedule).
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(&self) -> Array {
        Array::new()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> String {
        InstrumentType::CommodityAsianOption.to_string()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "CommodityAsianOption(id='{}', strike={:.2})",
            self.inner.id, self.inner.strike
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsCommodityAsianOption {
        JsCommodityAsianOption::from_inner(self.inner.clone())
    }
}
