//! WASM bindings for VolatilityIndexFuture.

use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::utils::json::{from_js_value, to_js_value};
use crate::valuations::common::parse::parse_optional_with_default;
use crate::valuations::common::{curve_id_from_str, instrument_id_from_str};
use crate::valuations::instruments::InstrumentWrapper;
use finstack_valuations::instruments::equity::vol_index_future::{
    VolIndexContractSpecs, VolatilityIndexFuture,
};
use finstack_valuations::instruments::rates::ir_future::Position;
use finstack_valuations::pricer::InstrumentType;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = VolatilityIndexFutureBuilder)]
#[derive(Clone, Debug, Default)]
pub struct JsVolatilityIndexFutureBuilder {
    instrument_id: String,
    notional: Option<finstack_core::money::Money>,
    quoted_price: Option<f64>,
    expiry: Option<finstack_core::dates::Date>,
    discount_curve: Option<String>,
    vol_index_curve: Option<String>,
    position: Option<String>,
    multiplier: Option<f64>,
    tick_size: Option<f64>,
    tick_value: Option<f64>,
    index_id: Option<String>,
}

#[wasm_bindgen(js_class = VolatilityIndexFutureBuilder)]
impl JsVolatilityIndexFutureBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(instrument_id: &str) -> JsVolatilityIndexFutureBuilder {
        JsVolatilityIndexFutureBuilder {
            instrument_id: instrument_id.to_string(),
            ..Default::default()
        }
    }

    #[wasm_bindgen(js_name = money)]
    pub fn money(mut self, notional: &JsMoney) -> JsVolatilityIndexFutureBuilder {
        self.notional = Some(notional.inner());
        self
    }

    #[wasm_bindgen(js_name = quotedPrice)]
    pub fn quoted_price(mut self, quoted_price: f64) -> JsVolatilityIndexFutureBuilder {
        self.quoted_price = Some(quoted_price);
        self
    }

    #[wasm_bindgen(js_name = expiry)]
    pub fn expiry(mut self, expiry: &JsDate) -> JsVolatilityIndexFutureBuilder {
        self.expiry = Some(expiry.inner());
        self
    }

    #[wasm_bindgen(js_name = discountCurve)]
    pub fn discount_curve(mut self, discount_curve: &str) -> JsVolatilityIndexFutureBuilder {
        self.discount_curve = Some(discount_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = volIndexCurve)]
    pub fn vol_index_curve(mut self, vol_index_curve: &str) -> JsVolatilityIndexFutureBuilder {
        self.vol_index_curve = Some(vol_index_curve.to_string());
        self
    }

    #[wasm_bindgen(js_name = position)]
    pub fn position(mut self, position: String) -> JsVolatilityIndexFutureBuilder {
        self.position = Some(position);
        self
    }

    #[wasm_bindgen(js_name = multiplier)]
    pub fn multiplier(mut self, multiplier: f64) -> JsVolatilityIndexFutureBuilder {
        self.multiplier = Some(multiplier);
        self
    }

    #[wasm_bindgen(js_name = tickSize)]
    pub fn tick_size(mut self, tick_size: f64) -> JsVolatilityIndexFutureBuilder {
        self.tick_size = Some(tick_size);
        self
    }

    #[wasm_bindgen(js_name = tickValue)]
    pub fn tick_value(mut self, tick_value: f64) -> JsVolatilityIndexFutureBuilder {
        self.tick_value = Some(tick_value);
        self
    }

    #[wasm_bindgen(js_name = indexId)]
    pub fn index_id(mut self, index_id: String) -> JsVolatilityIndexFutureBuilder {
        self.index_id = Some(index_id);
        self
    }

    #[wasm_bindgen(js_name = build)]
    pub fn build(self) -> Result<JsVolatilityIndexFuture, JsValue> {
        let notional = self.notional.ok_or_else(|| {
            js_error("VolatilityIndexFutureBuilder: notional (money) is required".to_string())
        })?;
        let quoted_price = self.quoted_price.ok_or_else(|| {
            js_error("VolatilityIndexFutureBuilder: quotedPrice is required".to_string())
        })?;
        let expiry = self.expiry.ok_or_else(|| {
            js_error("VolatilityIndexFutureBuilder: expiry is required".to_string())
        })?;
        let discount_curve = self.discount_curve.as_deref().ok_or_else(|| {
            js_error("VolatilityIndexFutureBuilder: discountCurve is required".to_string())
        })?;
        let vol_index_curve = self.vol_index_curve.as_deref().ok_or_else(|| {
            js_error("VolatilityIndexFutureBuilder: volIndexCurve is required".to_string())
        })?;

        let position_value = parse_optional_with_default(self.position, Position::Long)?;
        let specs = VolIndexContractSpecs {
            multiplier: self.multiplier.unwrap_or(1000.0),
            tick_size: self.tick_size.unwrap_or(0.05),
            tick_value: self.tick_value.unwrap_or(50.0),
            index_id: self.index_id.unwrap_or_else(|| "VIX".to_string()),
        };

        VolatilityIndexFuture::builder()
            .id(instrument_id_from_str(&self.instrument_id))
            .notional(notional)
            .quoted_price(quoted_price)
            .expiry(expiry)
            .discount_curve_id(curve_id_from_str(discount_curve))
            .vol_index_curve_id(curve_id_from_str(vol_index_curve))
            .position(position_value)
            .contract_specs(specs)
            .attributes(Default::default())
            .build()
            .map(JsVolatilityIndexFuture::from_inner)
            .map_err(|e| js_error(e.to_string()))
    }
}

#[wasm_bindgen(js_name = VolatilityIndexFuture)]
#[derive(Clone, Debug)]
pub struct JsVolatilityIndexFuture {
    pub(crate) inner: VolatilityIndexFuture,
}

impl InstrumentWrapper for JsVolatilityIndexFuture {
    type Inner = VolatilityIndexFuture;
    fn from_inner(inner: VolatilityIndexFuture) -> Self {
        JsVolatilityIndexFuture { inner }
    }
    fn inner(&self) -> VolatilityIndexFuture {
        self.inner.clone()
    }
}

#[wasm_bindgen(js_class = VolatilityIndexFuture)]
impl JsVolatilityIndexFuture {
    #[wasm_bindgen(getter, js_name = instrumentId)]
    pub fn instrument_id(&self) -> String {
        self.inner.id.as_str().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn notional(&self) -> JsMoney {
        JsMoney::from_inner(self.inner.notional)
    }

    #[wasm_bindgen(getter, js_name = quotedPrice)]
    pub fn quoted_price(&self) -> f64 {
        self.inner.quoted_price
    }

    #[wasm_bindgen(js_name = fromJson)]
    pub fn from_json(value: JsValue) -> Result<JsVolatilityIndexFuture, JsValue> {
        from_js_value(value).map(JsVolatilityIndexFuture::from_inner)
    }

    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        to_js_value(&self.inner)
    }

    #[wasm_bindgen(js_name = instrumentType)]
    pub fn instrument_type(&self) -> String {
        InstrumentType::VolatilityIndexFuture.to_string()
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "VolatilityIndexFuture(id='{}', price={:.2})",
            self.inner.id, self.inner.quoted_price
        )
    }

    #[wasm_bindgen(js_name = clone)]
    pub fn clone_js(&self) -> JsVolatilityIndexFuture {
        JsVolatilityIndexFuture::from_inner(self.inner.clone())
    }
}
