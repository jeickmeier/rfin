//! WASM bindings for IrFutureOption instrument.

use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::common::parameters::JsOptionType;
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::rates::ir_future_option::IrFutureOption;
use finstack_valuations::pricer::InstrumentType;
use js_sys::Array;
use wasm_bindgen::prelude::*;

/// Builder for IR future options.
#[wasm_bindgen(js_name = IrFutureOptionBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsIrFutureOptionBuilder {
    /// Instrument identifier string.
    instrument_id: String,
    /// Underlying futures price.
    futures_price: Option<f64>,
    /// Option strike price.
    strike: Option<f64>,
    /// Option expiry date.
    expiry: Option<finstack_core::dates::Date>,
    /// Call or Put.
    option_type: Option<finstack_valuations::instruments::OptionType>,
    /// Notional amount per contract.
    notional: Option<finstack_core::money::Money>,
    /// Tick size.
    tick_size: Option<f64>,
    /// Tick value in currency units.
    tick_value: Option<f64>,
    /// Lognormal (Black) volatility, annualized.
    volatility: Option<f64>,
    /// Discount curve ID.
    discount_curve_id: Option<String>,
}

#[wasm_bindgen(js_class = IrFutureOptionBuilder)]
impl JsIrFutureOptionBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsIrFutureOptionBuilder {
        JsIrFutureOptionBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = futuresPrice)]
    pub fn futures_price(mut self, futures_price: f64) -> JsIrFutureOptionBuilder {
        self.futures_price = Some(futures_price);
        self
    }

    #[wasm_bindgen(js_name = strike)]
    pub fn strike(mut self, strike: f64) -> JsIrFutureOptionBuilder {
        self.strike = Some(strike);
        self
    }

    #[wasm_bindgen(js_name = expiry)]
    pub fn expiry(mut self, expiry: &JsDate) -> JsIrFutureOptionBuilder {
        self.expiry = Some(expiry.inner());
        self
    }

    #[wasm_bindgen(js_name = optionType)]
    pub fn option_type(mut self, option_type: &JsOptionType) -> JsIrFutureOptionBuilder {
        self.option_type = Some(option_type.inner());
        self
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsIrFutureOptionBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = tickSize)]
    pub fn tick_size(mut self, tick_size: f64) -> JsIrFutureOptionBuilder {
        self.tick_size = Some(tick_size);
        self
    }

    #[wasm_bindgen(js_name = tickValue)]
    pub fn tick_value(mut self, tick_value: f64) -> JsIrFutureOptionBuilder {
        self.tick_value = Some(tick_value);
        self
    }

    #[wasm_bindgen(js_name = volatility)]
    pub fn volatility(mut self, volatility: f64) -> JsIrFutureOptionBuilder {
        self.volatility = Some(volatility);
        self
    }

    #[wasm_bindgen(js_name = discountCurve)]
    pub fn discount_curve(mut self, discount_curve: &str) -> JsIrFutureOptionBuilder {
        self.discount_curve_id = Some(discount_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsIrFutureOption, JsValue> {
        let futures_price = self.futures_price.ok_or_else(|| {
            js_error("IrFutureOptionBuilder: futuresPrice is required".to_string())
        })?;
        let strike = self.strike.ok_or_else(|| {
            js_error("IrFutureOptionBuilder: strike is required".to_string())
        })?;
        let expiry = self.expiry.ok_or_else(|| {
            js_error("IrFutureOptionBuilder: expiry is required".to_string())
        })?;
        let option_type = self.option_type.ok_or_else(|| {
            js_error("IrFutureOptionBuilder: optionType is required".to_string())
        })?;
        let notional = self.notional.ok_or_else(|| {
            js_error("IrFutureOptionBuilder: money (notional) is required".to_string())
        })?;
        let tick_size = self.tick_size.ok_or_else(|| {
            js_error("IrFutureOptionBuilder: tickSize is required".to_string())
        })?;
        let tick_value = self.tick_value.ok_or_else(|| {
            js_error("IrFutureOptionBuilder: tickValue is required".to_string())
        })?;
        let volatility = self.volatility.ok_or_else(|| {
            js_error("IrFutureOptionBuilder: volatility is required".to_string())
        })?;
        let discount_curve = self.discount_curve_id.as_deref().ok_or_else(|| {
            js_error("IrFutureOptionBuilder: discountCurve is required".to_string())
        })?;

        IrFutureOption::builder()
            .id(instrument_id_from_str(&self.instrument_id))
            .futures_price(futures_price)
            .strike(strike)
            .expiry(expiry)
            .option_type(option_type)
            .notional(notional)
            .tick_size(tick_size)
            .tick_value(tick_value)
            .volatility(volatility)
            .discount_curve_id(curve_id_from_str(discount_curve))
            .build()
            .map(JsIrFutureOption::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }
}

/// Exchange-traded option on an interest rate future (e.g., SOFR futures options).
#[wasm_bindgen(js_name = IrFutureOption)]
#[derive(Clone, Debug)]
pub struct JsIrFutureOption {
    pub(crate) inner: IrFutureOption,
}

impl InstrumentWrapper for JsIrFutureOption {
    type Inner = IrFutureOption;
    fn from_inner(inner: IrFutureOption) -> Self {
        JsIrFutureOption { inner }
    }
    fn inner(&self) -> IrFutureOption {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = IrFutureOption)]
impl JsIrFutureOption {
    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsIrFutureOption, JsValue> {
        from_js_value(value).map(JsIrFutureOption::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    /// Get the underlying futures price.
    #[wasm_bindgen(getter, js_name = futuresPrice)]
    pub fn futures_price(&self) -> f64 {
        self.inner.futures_price
    }

    /// Get the strike price.
    #[wasm_bindgen(getter)]
    pub fn strike(&self) -> f64 {
        self.inner.strike
    }

    /// Get the notional amount.
    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    /// Get the Black volatility.
    #[wasm_bindgen(getter)]
    pub fn volatility(&self) -> f64 {
        self.inner.volatility
    }

    /// IR future options return an empty cashflow schedule.
    #[wasm_bindgen(js_name = getCashflows)]
    pub fn get_cashflows(&self) -> Array {
        Array::new()
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> String {
        InstrumentType::IrFutureOption.to_string()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "IrFutureOption(id='{}', futures={:.2}, strike={:.2})",
            self.inner.id, self.inner.futures_price, self.inner.strike
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsIrFutureOption {
        JsIrFutureOption::from_inner(self.inner.clone())
    }
}
