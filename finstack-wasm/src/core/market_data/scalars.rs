use crate::core::currency::JsCurrency;
use crate::core::dates::date::JsDate;
use crate::core::error::js_error;
use crate::core::money::JsMoney;
use crate::core::utils::js_array_from_iter;
use finstack_core::market_data::scalars::{
    InflationIndex, InflationInterpolation, InflationLag, MarketScalar, ScalarTimeSeries,
    SeriesInterpolation,
};
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = SeriesInterpolation)]
#[derive(Clone, Copy, Debug)]
pub struct JsSeriesInterpolation {
    inner: SeriesInterpolation,
}

impl JsSeriesInterpolation {
    fn new(inner: SeriesInterpolation) -> Self {
        Self { inner }
    }
}

impl From<JsSeriesInterpolation> for SeriesInterpolation {
    fn from(value: JsSeriesInterpolation) -> Self {
        value.inner
    }
}

impl From<SeriesInterpolation> for JsSeriesInterpolation {
    fn from(value: SeriesInterpolation) -> Self {
        Self::new(value)
    }
}

#[wasm_bindgen(js_class = SeriesInterpolation)]
impl JsSeriesInterpolation {
    #[wasm_bindgen(js_name = Step)]
    pub fn step() -> JsSeriesInterpolation {
        Self::new(SeriesInterpolation::Step)
    }

    #[wasm_bindgen(js_name = Linear)]
    pub fn linear() -> JsSeriesInterpolation {
        Self::new(SeriesInterpolation::Linear)
    }

    #[wasm_bindgen(js_name = fromName)]
    pub fn from_name(name: &str) -> Result<JsSeriesInterpolation, JsValue> {
        match name.to_ascii_lowercase().as_str() {
            "step" => Ok(Self::step()),
            "linear" => Ok(Self::linear()),
            other => Err(js_error(format!("Unknown interpolation style: {other}"))),
        }
    }

    #[wasm_bindgen(js_name = name)]
    pub fn name(&self) -> String {
        match self.inner {
            SeriesInterpolation::Step => "step".to_string(),
            SeriesInterpolation::Linear => "linear".to_string(),
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
        currency: &JsCurrency,
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

        let mut series = ScalarTimeSeries::new(id, observations, Some(currency.inner()))
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

// ======================================================================
// Inflation types
// ======================================================================

/// Interpolation method for inflation index values between monthly observations.
#[wasm_bindgen(js_name = InflationInterpolation)]
#[derive(Clone, Copy, Debug)]
pub struct JsInflationInterpolation {
    inner: InflationInterpolation,
}

impl JsInflationInterpolation {
    pub(crate) fn inner(&self) -> InflationInterpolation {
        self.inner
    }
}

#[wasm_bindgen(js_class = InflationInterpolation)]
impl JsInflationInterpolation {
    /// Step interpolation (last observation carried forward).
    #[wasm_bindgen(js_name = Step)]
    pub fn step() -> JsInflationInterpolation {
        Self {
            inner: InflationInterpolation::Step,
        }
    }

    /// Linear interpolation between monthly observations (TIPS standard).
    #[wasm_bindgen(js_name = Linear)]
    pub fn linear() -> JsInflationInterpolation {
        Self {
            inner: InflationInterpolation::Linear,
        }
    }

    /// Parse from string name.
    #[wasm_bindgen(js_name = fromName)]
    pub fn from_name(name: &str) -> Result<JsInflationInterpolation, JsValue> {
        match name.to_ascii_lowercase().as_str() {
            "step" => Ok(Self::step()),
            "linear" => Ok(Self::linear()),
            other => Err(js_error(format!(
                "Unknown inflation interpolation: {other}"
            ))),
        }
    }

    /// String name of the interpolation method.
    #[wasm_bindgen(js_name = name)]
    pub fn name(&self) -> String {
        self.inner.to_string()
    }
}

/// Publication lag for inflation index reference dates.
///
/// @example
/// ```javascript
/// const tipsLag = InflationLag.Months(3);  // US TIPS standard
/// const noLag = InflationLag.None();       // Inflation swaps
/// ```
#[wasm_bindgen(js_name = InflationLag)]
#[derive(Clone, Copy, Debug)]
pub struct JsInflationLag {
    inner: InflationLag,
}

impl JsInflationLag {
    pub(crate) fn inner(&self) -> InflationLag {
        self.inner
    }
}

#[wasm_bindgen(js_class = InflationLag)]
impl JsInflationLag {
    /// No lag applied (used for inflation swaps).
    #[wasm_bindgen(js_name = None)]
    pub fn none() -> JsInflationLag {
        Self {
            inner: InflationLag::None,
        }
    }

    /// Lag by specified number of months (standard: 3 for TIPS).
    #[wasm_bindgen(js_name = Months)]
    pub fn months(n: u8) -> JsInflationLag {
        Self {
            inner: InflationLag::Months(n),
        }
    }

    /// Lag by specified number of calendar days.
    #[wasm_bindgen(js_name = Days)]
    pub fn days(n: u16) -> JsInflationLag {
        Self {
            inner: InflationLag::Days(n),
        }
    }
}

/// Inflation index time series with lag and interpolation support.
///
/// Wraps historical CPI/RPI observations with market conventions.
///
/// @example
/// ```javascript
/// const index = new InflationIndex("US-CPI", dates, values, Currency.USD(),
///   InflationInterpolation.Linear(), InflationLag.Months(3));
/// const ratio = index.ratio(baseDate, settleDate);
/// ```
#[wasm_bindgen(js_name = InflationIndex)]
#[derive(Clone)]
pub struct JsInflationIndex {
    inner: Arc<InflationIndex>,
}

#[wasm_bindgen(js_class = InflationIndex)]
impl JsInflationIndex {
    /// Create an inflation index from observation data.
    ///
    /// @param {string} id - Index identifier (e.g. "US-CPI")
    /// @param {FsDate[]} dates - Observation dates
    /// @param {Float64Array} values - Index levels at each date
    /// @param {Currency} currency - Currency denomination
    /// @param {InflationInterpolation} [interpolation] - Interpolation method
    /// @param {InflationLag} [lag] - Publication lag
    #[wasm_bindgen(constructor)]
    pub fn new(
        id: &str,
        dates: Vec<JsDate>,
        values: Vec<f64>,
        currency: &JsCurrency,
        interpolation: Option<JsInflationInterpolation>,
        lag: Option<JsInflationLag>,
    ) -> Result<JsInflationIndex, JsValue> {
        if dates.len() != values.len() {
            return Err(js_error(
                "dates and values must have the same number of entries",
            ));
        }

        let observations: Vec<_> = dates
            .into_iter()
            .zip(values)
            .map(|(d, v)| (d.inner(), v))
            .collect();

        let mut index = InflationIndex::new(id, observations, currency.inner())
            .map_err(|e| js_error(e.to_string()))?;

        if let Some(interp) = interpolation {
            index = index.with_interpolation(interp.inner());
        }
        if let Some(l) = lag {
            index = index.with_lag(l.inner());
        }

        Ok(JsInflationIndex {
            inner: Arc::new(index),
        })
    }

    /// Index identifier.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.clone()
    }

    /// Reference index level at a given date (with lag and interpolation applied).
    #[wasm_bindgen(js_name = referenceIndex)]
    pub fn reference_index(&self, date: &JsDate) -> Result<f64, JsValue> {
        self.inner
            .value_on(date.inner())
            .map_err(|e| js_error(e.to_string()))
    }

    /// Index ratio between two dates.
    ///
    /// @param {FsDate} base - Base date
    /// @param {FsDate} settle - Settlement date
    /// @returns {number} Index ratio (settle / base)
    #[wasm_bindgen(js_name = ratio)]
    pub fn ratio(&self, base: &JsDate, settle: &JsDate) -> Result<f64, JsValue> {
        self.inner
            .ratio(base.inner(), settle.inner())
            .map_err(|e| js_error(e.to_string()))
    }
}
