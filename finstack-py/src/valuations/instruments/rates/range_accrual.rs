use super::common::{
    meta_attributes, option_pricing_overrides, require_builder_clone, require_builder_field,
};
use crate::core::common::args::DayCountArg;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::rates::range_accrual::{BoundsType, RangeAccrual};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyModule, PyType};
use pyo3::{Bound, PyRefMut};
use std::collections::HashMap;
use std::sync::Arc;

/// How range bounds are interpreted (absolute price levels or relative to initial spot).
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "BoundsType",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyBoundsType {
    pub(crate) inner: BoundsType,
}

impl PyBoundsType {
    pub(crate) const fn new(inner: BoundsType) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyBoundsType {
    #[classattr]
    const ABSOLUTE: Self = Self::new(BoundsType::Absolute);
    #[classattr]
    const RELATIVE_TO_INITIAL_SPOT: Self = Self::new(BoundsType::RelativeToInitialSpot);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        name.parse()
            .map(Self::new)
            .map_err(|e: String| PyValueError::new_err(e))
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("BoundsType('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __hash__(&self) -> isize {
        self.inner as isize
    }
}

impl From<PyBoundsType> for BoundsType {
    fn from(value: PyBoundsType) -> Self {
        value.inner
    }
}

/// Range accrual instrument.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "RangeAccrual",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRangeAccrual {
    pub(crate) inner: Arc<RangeAccrual>,
}

impl PyRangeAccrual {
    pub(crate) fn new(inner: RangeAccrual) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "RangeAccrualBuilder"
)]
pub struct PyRangeAccrualBuilder {
    instrument_id: InstrumentId,
    ticker: Option<String>,
    observation_dates: Vec<time::Date>,
    lower_bound: Option<f64>,
    upper_bound: Option<f64>,
    bounds_type: BoundsType,
    coupon_rate: Option<f64>,
    notional: Option<finstack_core::money::Money>,
    day_count: finstack_core::dates::DayCount,
    discount_curve: Option<CurveId>,
    spot_id: Option<String>,
    vol_surface: Option<CurveId>,
    div_yield_id: Option<CurveId>,
    payment_date: Option<time::Date>,
    past_fixings_in_range: Option<u32>,
    total_past_observations: Option<u32>,
    implied_volatility: Option<f64>,
    tree_steps: Option<usize>,
    pending_attributes: Option<HashMap<String, String>>,
}

impl PyRangeAccrualBuilder {
    fn new_with_id(id: InstrumentId) -> Self {
        Self {
            instrument_id: id,
            ticker: None,
            observation_dates: Vec::new(),
            lower_bound: None,
            upper_bound: None,
            bounds_type: BoundsType::default(),
            coupon_rate: None,
            notional: None,
            day_count: finstack_core::dates::DayCount::Act365F,
            discount_curve: None,
            spot_id: None,
            vol_surface: None,
            div_yield_id: None,
            payment_date: None,
            past_fixings_in_range: None,
            total_past_observations: None,
            implied_volatility: None,
            tree_steps: None,
            pending_attributes: None,
        }
    }
}

#[pymethods]
impl PyRangeAccrualBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn new_py(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    #[pyo3(text_signature = "($self, ticker)")]
    fn ticker(mut slf: PyRefMut<'_, Self>, ticker: String) -> PyRefMut<'_, Self> {
        slf.ticker = Some(ticker);
        slf
    }

    #[pyo3(text_signature = "($self, dates)")]
    fn observation_dates<'py>(
        mut slf: PyRefMut<'py, Self>,
        dates: Bound<'py, PyList>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        let mut obs = Vec::new();
        for item in dates.iter() {
            obs.push(py_to_date(&item).context("observation_dates")?);
        }
        slf.observation_dates = obs;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, lower_bound)")]
    fn lower_bound(mut slf: PyRefMut<'_, Self>, lower_bound: f64) -> PyRefMut<'_, Self> {
        slf.lower_bound = Some(lower_bound);
        slf
    }

    #[pyo3(text_signature = "($self, upper_bound)")]
    fn upper_bound(mut slf: PyRefMut<'_, Self>, upper_bound: f64) -> PyRefMut<'_, Self> {
        slf.upper_bound = Some(upper_bound);
        slf
    }

    #[pyo3(text_signature = "($self, bounds_type)")]
    fn bounds_type<'py>(
        mut slf: PyRefMut<'py, Self>,
        bounds_type: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        if let Ok(py_bt) = bounds_type.extract::<pyo3::PyRef<PyBoundsType>>() {
            slf.bounds_type = py_bt.inner;
        } else if let Ok(name) = bounds_type.extract::<&str>() {
            slf.bounds_type = name.parse().map_err(|e: String| PyValueError::new_err(e))?;
        } else {
            return Err(PyValueError::new_err(
                "bounds_type() expects str or BoundsType",
            ));
        }
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, coupon_rate)")]
    fn coupon_rate(mut slf: PyRefMut<'_, Self>, coupon_rate: f64) -> PyRefMut<'_, Self> {
        slf.coupon_rate = Some(coupon_rate);
        slf
    }

    #[pyo3(text_signature = "($self, notional)")]
    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        notional: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        slf.notional = Some(extract_money(&notional).context("notional")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, day_count)")]
    fn day_count<'py>(
        mut slf: PyRefMut<'py, Self>,
        day_count: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let DayCountArg(dc) = day_count
            .extract()
            .map_err(|_| PyValueError::new_err("day_count() expects DayCount or str"))?;
        slf.day_count = dc;
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, discount_curve)")]
    fn discount_curve(mut slf: PyRefMut<'_, Self>, discount_curve: String) -> PyRefMut<'_, Self> {
        slf.discount_curve = Some(CurveId::new(&discount_curve));
        slf
    }

    #[pyo3(text_signature = "($self, spot_id)")]
    fn spot_id(mut slf: PyRefMut<'_, Self>, spot_id: String) -> PyRefMut<'_, Self> {
        slf.spot_id = Some(spot_id);
        slf
    }

    #[pyo3(text_signature = "($self, vol_surface)")]
    fn vol_surface(mut slf: PyRefMut<'_, Self>, vol_surface: String) -> PyRefMut<'_, Self> {
        slf.vol_surface = Some(CurveId::new(&vol_surface));
        slf
    }

    #[pyo3(text_signature = "($self, div_yield_id=None)", signature = (div_yield_id=None))]
    fn div_yield_id(
        mut slf: PyRefMut<'_, Self>,
        div_yield_id: Option<String>,
    ) -> PyRefMut<'_, Self> {
        slf.div_yield_id = div_yield_id.map(|d| CurveId::new(&d));
        slf
    }

    #[pyo3(text_signature = "($self, payment_date)")]
    fn payment_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        payment_date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        use crate::errors::PyContext;
        slf.payment_date = Some(py_to_date(&payment_date).context("payment_date")?);
        Ok(slf)
    }

    #[pyo3(text_signature = "($self, count=None)", signature = (count=None))]
    fn past_fixings_in_range(
        mut slf: PyRefMut<'_, Self>,
        count: Option<u32>,
    ) -> PyRefMut<'_, Self> {
        slf.past_fixings_in_range = count;
        slf
    }

    #[pyo3(text_signature = "($self, count=None)", signature = (count=None))]
    fn total_past_observations(
        mut slf: PyRefMut<'_, Self>,
        count: Option<u32>,
    ) -> PyRefMut<'_, Self> {
        slf.total_past_observations = count;
        slf
    }

    #[pyo3(
        text_signature = "($self, implied_volatility=None)",
        signature = (implied_volatility=None)
    )]
    fn implied_volatility(
        mut slf: PyRefMut<'_, Self>,
        implied_volatility: Option<f64>,
    ) -> PyRefMut<'_, Self> {
        slf.implied_volatility = implied_volatility;
        slf
    }

    #[pyo3(text_signature = "($self, tree_steps=None)", signature = (tree_steps=None))]
    fn tree_steps(mut slf: PyRefMut<'_, Self>, tree_steps: Option<usize>) -> PyRefMut<'_, Self> {
        slf.tree_steps = tree_steps;
        slf
    }

    #[pyo3(text_signature = "($self, attributes=None)", signature = (attributes=None))]
    fn attributes(
        mut slf: PyRefMut<'_, Self>,
        attributes: Option<HashMap<String, String>>,
    ) -> PyRefMut<'_, Self> {
        slf.pending_attributes = attributes;
        slf
    }

    #[pyo3(text_signature = "($self)")]
    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyRangeAccrual> {
        let ticker = require_builder_clone("RangeAccrualBuilder", "ticker", slf.ticker.as_ref())?;
        let notional = require_builder_field("RangeAccrualBuilder", "notional", slf.notional)?;
        let discount_curve = require_builder_clone(
            "RangeAccrualBuilder",
            "discount_curve",
            slf.discount_curve.as_ref(),
        )?;
        let spot_id =
            require_builder_clone("RangeAccrualBuilder", "spot_id", slf.spot_id.as_ref())?;
        let vol_surface = require_builder_clone(
            "RangeAccrualBuilder",
            "vol_surface",
            slf.vol_surface.as_ref(),
        )?;
        let lower = require_builder_field("RangeAccrualBuilder", "lower_bound", slf.lower_bound)?;
        let upper = require_builder_field("RangeAccrualBuilder", "upper_bound", slf.upper_bound)?;
        let coupon = require_builder_field("RangeAccrualBuilder", "coupon_rate", slf.coupon_rate)?;

        let pricing_overrides = option_pricing_overrides(slf.implied_volatility, slf.tree_steps);
        let attrs = meta_attributes(slf.pending_attributes.as_ref());

        let mut builder = RangeAccrual::builder();
        builder = builder.id(slf.instrument_id.clone());
        builder = builder.underlying_ticker(ticker);
        builder = builder.observation_dates(slf.observation_dates.clone());
        builder = builder.lower_bound(lower);
        builder = builder.upper_bound(upper);
        builder = builder.bounds_type(slf.bounds_type);
        builder = builder.coupon_rate(coupon);
        builder = builder.notional(notional);
        builder = builder.day_count(slf.day_count);
        builder = builder.pricing_overrides(pricing_overrides);
        builder = builder.discount_curve_id(discount_curve);
        builder = builder.spot_id(spot_id.into());
        builder = builder.vol_surface_id(vol_surface);
        builder = builder.div_yield_id_opt(slf.div_yield_id.clone());
        builder = builder.payment_date_opt(slf.payment_date);
        builder = builder.past_fixings_in_range_opt(slf.past_fixings_in_range.map(|v| v as usize));
        builder =
            builder.total_past_observations_opt(slf.total_past_observations.map(|v| v as usize));
        builder = builder.attributes(attrs);

        let range_accrual = builder
            .build()
            .map_err(|e| PyValueError::new_err(format!("Failed to build RangeAccrual: {e}")))?;
        Ok(PyRangeAccrual::new(range_accrual))
    }

    fn __repr__(&self) -> String {
        "RangeAccrualBuilder(...)".to_string()
    }
}

#[pymethods]
impl PyRangeAccrual {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    fn builder(_cls: &Bound<'_, PyType>, instrument_id: &str) -> PyRangeAccrualBuilder {
        PyRangeAccrualBuilder::new_with_id(InstrumentId::new(instrument_id))
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn underlying_ticker(&self) -> &str {
        &self.inner.underlying_ticker
    }

    #[getter]
    fn lower_bound(&self) -> f64 {
        self.inner.lower_bound
    }

    #[getter]
    fn upper_bound(&self) -> f64 {
        self.inner.upper_bound
    }

    #[getter]
    fn coupon_rate(&self) -> f64 {
        self.inner.coupon_rate
    }

    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    #[getter]
    fn observation_dates(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dates = PyList::empty(py);
        for d in &self.inner.observation_dates {
            dates.append(date_to_py(py, *d)?)?;
        }
        Ok(dates.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "RangeAccrual(id='{}', ticker='{}', lower_bound={}, upper_bound={})",
            self.inner.id.as_str(),
            self.inner.underlying_ticker,
            self.inner.lower_bound,
            self.inner.upper_bound
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    parent.add_class::<PyBoundsType>()?;
    parent.add_class::<PyRangeAccrual>()?;
    parent.add_class::<PyRangeAccrualBuilder>()?;
    Ok(vec!["BoundsType", "RangeAccrual", "RangeAccrualBuilder"])
}
