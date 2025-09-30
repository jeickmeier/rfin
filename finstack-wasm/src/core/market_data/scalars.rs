use crate::core::currency::JsCurrency;
use crate::core::dates::date::JsDate;
use crate::core::money::JsMoney;
use crate::core::utils::{js_array_from_iter, js_error};
use finstack_core::market_data::scalars::{MarketScalar, ScalarTimeSeries, SeriesInterpolation};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsSeriesInterpolation {
    Step,
    Linear,
}

impl From<JsSeriesInterpolation> for SeriesInterpolation {
    fn from(value: JsSeriesInterpolation) -> Self {
        match value {
            JsSeriesInterpolation::Step => SeriesInterpolation::Step,
            JsSeriesInterpolation::Linear => SeriesInterpolation::Linear,
        }
    }
}

impl From<SeriesInterpolation> for JsSeriesInterpolation {
    fn from(value: SeriesInterpolation) -> Self {
        match value {
            SeriesInterpolation::Step => JsSeriesInterpolation::Step,
            SeriesInterpolation::Linear => JsSeriesInterpolation::Linear,
        }
    }
}

impl JsSeriesInterpolation {
    pub fn from_name(name: &str) -> Result<JsSeriesInterpolation, JsValue> {
        match name.to_ascii_lowercase().as_str() {
            "step" => Ok(JsSeriesInterpolation::Step),
            "linear" => Ok(JsSeriesInterpolation::Linear),
            other => Err(js_error(format!("Unknown interpolation style: {other}"))),
        }
    }

    pub fn name(&self) -> String {
        match self {
            JsSeriesInterpolation::Step => "step".to_string(),
            JsSeriesInterpolation::Linear => "linear".to_string(),
        }
    }
}

#[wasm_bindgen(js_name = MarketScalar)]
#[derive(Clone, Debug)]
pub struct JsMarketScalar {
    inner: MarketScalar,
}

impl JsMarketScalar {
    pub(crate) fn inner(&self) -> MarketScalar {
        self.inner.clone()
    }

    pub(crate) fn from_inner(inner: MarketScalar) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = MarketScalar)]
impl JsMarketScalar {
    #[wasm_bindgen(js_name = unitless)]
    pub fn unitless(value: f64) -> JsMarketScalar {
        JsMarketScalar {
            inner: MarketScalar::Unitless(value),
        }
    }

    #[wasm_bindgen(js_name = price)]
    pub fn price(money: &JsMoney) -> JsMarketScalar {
        JsMarketScalar {
            inner: MarketScalar::Price(money.inner()),
        }
    }

    #[wasm_bindgen(getter, js_name = isUnitless)]
    pub fn is_unitless(&self) -> bool {
        matches!(self.inner, MarketScalar::Unitless(_))
    }

    #[wasm_bindgen(getter, js_name = isPrice)]
    pub fn is_price(&self) -> bool {
        matches!(self.inner, MarketScalar::Price(_))
    }

    #[wasm_bindgen(getter)]
    pub fn value(&self) -> JsValue {
        match &self.inner {
            MarketScalar::Unitless(v) => JsValue::from_f64(*v),
            MarketScalar::Price(m) => JsValue::from(JsMoney::from_inner(*m)),
        }
    }
}

#[wasm_bindgen(js_name = ScalarTimeSeries)]
#[derive(Clone)]
pub struct JsScalarTimeSeries {
    inner: Arc<ScalarTimeSeries>,
}

impl JsScalarTimeSeries {
    pub(crate) fn inner(&self) -> ScalarTimeSeries {
        self.inner.as_ref().clone()
    }

    pub(crate) fn from_arc(inner: Arc<ScalarTimeSeries>) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = ScalarTimeSeries)]
impl JsScalarTimeSeries {
    #[wasm_bindgen(constructor)]
    pub fn new(
        id: &str,
        dates: Vec<JsDate>,
        values: Vec<f64>,
        currency: Option<JsCurrency>,
        interpolation: Option<JsSeriesInterpolation>,
    ) -> Result<JsScalarTimeSeries, JsValue> {
        if dates.len() != values.len() {
            return Err(js_error(
                "dates and values must contain the same number of entries",
            ));
        }
        if dates.is_empty() {
            return Err(js_error("at least one observation is required"));
        }

        let observations: Vec<_> = dates
            .into_iter()
            .zip(values)
            .map(|(d, v)| (d.inner(), v))
            .collect();

        let mut series = ScalarTimeSeries::new(id, observations, currency.map(|c| c.inner()))
            .map_err(|e| js_error(e.to_string()))?;

        if let Some(mode) = interpolation {
            series = series.with_interpolation(mode.into());
        }

        Ok(JsScalarTimeSeries {
            inner: Arc::new(series),
        })
    }

    #[wasm_bindgen(js_name = setInterpolation)]
    pub fn set_interpolation(&mut self, interpolation: JsSeriesInterpolation) {
        let updated = self
            .inner
            .as_ref()
            .clone()
            .with_interpolation(interpolation.into());
        self.inner = Arc::new(updated);
    }

    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id().to_string()
    }

    #[wasm_bindgen(getter)]
    pub fn currency(&self) -> Option<JsCurrency> {
        self.inner.currency().map(JsCurrency::from_inner)
    }

    #[wasm_bindgen(getter)]
    pub fn interpolation(&self) -> JsSeriesInterpolation {
        JsSeriesInterpolation::from(self.inner.interpolation())
    }

    #[wasm_bindgen(js_name = valueOn)]
    pub fn value_on(&self, date: &JsDate) -> Result<f64, JsValue> {
        self.inner
            .value_on(date.inner())
            .map_err(|e| js_error(e.to_string()))
    }

    #[wasm_bindgen(js_name = valuesOn)]
    pub fn values_on(&self, dates: Vec<JsDate>) -> Result<js_sys::Array, JsValue> {
        let cores: Vec<_> = dates.into_iter().map(|d| d.inner()).collect();
        let values = self
            .inner
            .values_on(&cores)
            .map_err(|e| js_error(e.to_string()))?;
        Ok(js_array_from_iter(
            values.into_iter().map(JsValue::from_f64),
        ))
    }
}
