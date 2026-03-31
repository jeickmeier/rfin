//! WASM bindings for CommoditySpreadOption instrument.

use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::common::parameters::JsOptionType;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_core::currency::Currency;
use finstack_valuations::instruments::commodity::commodity_spread_option::CommoditySpreadOption;
use finstack_valuations::instruments::{Attributes, PricingOverrides};
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

/// Builder for commodity spread options.
#[wasm_bindgen(js_name = CommoditySpreadOptionBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsCommoditySpreadOptionBuilder {
    /// Instrument identifier string.
    instrument_id: String,
    /// Settlement currency.
    currency: Option<Currency>,
    /// Option type (call/put).
    option_type: Option<finstack_valuations::instruments::OptionType>,
    /// Option expiry date.
    expiry: Option<finstack_core::dates::Date>,
    /// Spread strike price.
    strike: Option<f64>,
    /// Notional quantity.
    notional: Option<f64>,
    /// Leg 1 forward curve ID.
    leg1_forward_curve_id: Option<String>,
    /// Leg 2 forward curve ID.
    leg2_forward_curve_id: Option<String>,
    /// Leg 1 vol surface ID.
    leg1_vol_surface_id: Option<String>,
    /// Leg 2 vol surface ID.
    leg2_vol_surface_id: Option<String>,
    /// Discount curve ID.
    discount_curve_id: Option<String>,
    /// Correlation between the two commodities.
    correlation: Option<f64>,
}

#[wasm_bindgen(js_class = CommoditySpreadOptionBuilder)]
impl JsCommoditySpreadOptionBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsCommoditySpreadOptionBuilder {
        JsCommoditySpreadOptionBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = currency)]
    pub fn currency(
        mut self,
        currency: String,
    ) -> Result<JsCommoditySpreadOptionBuilder, JsValue> {
        let ccy: Currency = currency
            .parse()
            .map_err(|e: strum::ParseError| js_error(e.to_string()))?;
        self.currency = Some(ccy);
        Ok(self)
    }

    #[wasm_bindgen(js_name = optionType)]
    pub fn option_type(
        mut self,
        option_type: &JsOptionType,
    ) -> JsCommoditySpreadOptionBuilder {
        self.option_type = Some(option_type.inner());
        self
    }

    #[wasm_bindgen(js_name = expiry)]
    pub fn expiry(mut self, expiry: &JsDate) -> JsCommoditySpreadOptionBuilder {
        self.expiry = Some(expiry.inner());
        self
    }

    #[wasm_bindgen(js_name = strike)]
    pub fn strike(mut self, strike: f64) -> JsCommoditySpreadOptionBuilder {
        self.strike = Some(strike);
        self
    }

    #[wasm_bindgen(js_name = notional)]
    pub fn notional(mut self, notional: f64) -> JsCommoditySpreadOptionBuilder {
        self.notional = Some(notional);
        self
    }

    #[wasm_bindgen(js_name = leg1ForwardCurveId)]
    pub fn leg1_forward_curve_id(mut self, id: &str) -> JsCommoditySpreadOptionBuilder {
        self.leg1_forward_curve_id = Some(id.to_string());
        self
    }

    #[wasm_bindgen(js_name = leg2ForwardCurveId)]
    pub fn leg2_forward_curve_id(mut self, id: &str) -> JsCommoditySpreadOptionBuilder {
        self.leg2_forward_curve_id = Some(id.to_string());
        self
    }

    #[wasm_bindgen(js_name = leg1VolSurfaceId)]
    pub fn leg1_vol_surface_id(mut self, id: &str) -> JsCommoditySpreadOptionBuilder {
        self.leg1_vol_surface_id = Some(id.to_string());
        self
    }

    #[wasm_bindgen(js_name = leg2VolSurfaceId)]
    pub fn leg2_vol_surface_id(mut self, id: &str) -> JsCommoditySpreadOptionBuilder {
        self.leg2_vol_surface_id = Some(id.to_string());
        self
    }

    #[wasm_bindgen(js_name = discountCurveId)]
    pub fn discount_curve_id(mut self, id: &str) -> JsCommoditySpreadOptionBuilder {
        self.discount_curve_id = Some(id.to_string());
        self
    }

    #[wasm_bindgen(js_name = correlation)]
    pub fn correlation(mut self, correlation: f64) -> JsCommoditySpreadOptionBuilder {
        self.correlation = Some(correlation);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsCommoditySpreadOption, JsValue> {
        let ccy = self.currency.ok_or_else(|| {
            js_error("CommoditySpreadOptionBuilder: currency is required".to_string())
        })?;
        let option_type = self.option_type.ok_or_else(|| {
            js_error("CommoditySpreadOptionBuilder: optionType is required".to_string())
        })?;
        let expiry = self.expiry.ok_or_else(|| {
            js_error("CommoditySpreadOptionBuilder: expiry is required".to_string())
        })?;
        let strike = self.strike.ok_or_else(|| {
            js_error("CommoditySpreadOptionBuilder: strike is required".to_string())
        })?;
        let notional = self.notional.ok_or_else(|| {
            js_error("CommoditySpreadOptionBuilder: notional is required".to_string())
        })?;
        let leg1_fwd = self.leg1_forward_curve_id.as_deref().ok_or_else(|| {
            js_error("CommoditySpreadOptionBuilder: leg1ForwardCurveId is required".to_string())
        })?;
        let leg2_fwd = self.leg2_forward_curve_id.as_deref().ok_or_else(|| {
            js_error("CommoditySpreadOptionBuilder: leg2ForwardCurveId is required".to_string())
        })?;
        let leg1_vol = self.leg1_vol_surface_id.as_deref().ok_or_else(|| {
            js_error("CommoditySpreadOptionBuilder: leg1VolSurfaceId is required".to_string())
        })?;
        let leg2_vol = self.leg2_vol_surface_id.as_deref().ok_or_else(|| {
            js_error("CommoditySpreadOptionBuilder: leg2VolSurfaceId is required".to_string())
        })?;
        let disc = self.discount_curve_id.as_deref().ok_or_else(|| {
            js_error("CommoditySpreadOptionBuilder: discountCurveId is required".to_string())
        })?;
        let correlation = self.correlation.ok_or_else(|| {
            js_error("CommoditySpreadOptionBuilder: correlation is required".to_string())
        })?;

        CommoditySpreadOption::builder()
            .id(instrument_id_from_str(&self.instrument_id))
            .currency(ccy)
            .option_type(option_type)
            .expiry(expiry)
            .strike(strike)
            .notional(notional)
            .leg1_forward_curve_id(curve_id_from_str(leg1_fwd))
            .leg2_forward_curve_id(curve_id_from_str(leg2_fwd))
            .leg1_vol_surface_id(curve_id_from_str(leg1_vol))
            .leg2_vol_surface_id(curve_id_from_str(leg2_vol))
            .discount_curve_id(curve_id_from_str(disc))
            .correlation(correlation)
            .pricing_overrides(PricingOverrides::default())
            .attributes(Attributes::new())
            .build()
            .map(JsCommoditySpreadOption::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }
}

/// Commodity spread option (option on the price difference between two commodities).
#[wasm_bindgen(js_name = CommoditySpreadOption)]
#[derive(Clone, Debug)]
pub struct JsCommoditySpreadOption {
    pub(crate) inner: CommoditySpreadOption,
}

impl InstrumentWrapper for JsCommoditySpreadOption {
    type Inner = CommoditySpreadOption;
    fn from_inner(inner: CommoditySpreadOption) -> Self {
        JsCommoditySpreadOption { inner }
    }
    fn inner(&self) -> CommoditySpreadOption {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = CommoditySpreadOption)]
impl JsCommoditySpreadOption {
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsCommoditySpreadOption, JsValue> {
        from_js_value(value).map(JsCommoditySpreadOption::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    /// Get the spread strike price.
    #[wasm_bindgen(getter)]
    pub fn strike(&self) -> f64 {
        self.inner.strike
    }

    /// Get the notional quantity.
    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> f64 {
        self.inner.notional
    }

    /// Get the correlation between the two commodities.
    #[wasm_bindgen(getter)]
    pub fn correlation(&self) -> f64 {
        self.inner.correlation
    }

    /// Get a cashflow view (spread options return an empty schedule).
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(&self) -> Array {
        Array::new()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> String {
        InstrumentType::CommoditySpreadOption.to_string()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "CommoditySpreadOption(id='{}', strike={:.2})",
            self.inner.id, self.inner.strike
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsCommoditySpreadOption {
        JsCommoditySpreadOption::from_inner(self.inner.clone())
    }
}
