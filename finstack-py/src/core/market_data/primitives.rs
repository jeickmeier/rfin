//! Python bindings for MarketScalar and ScalarTimeSeries.

use pyo3::prelude::*;
use pyo3::types::PyList;

use finstack_core::market_data::primitives::{MarketScalar as CoreMarketScalar, ScalarTimeSeries as CoreSeries, SeriesInterpolation as CoreInterp};
use crate::core::currency::PyCurrency;
use crate::core::dates::PyDate;
use crate::core::money::PyMoney;

#[pyclass(name = "SeriesInterpolation")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PySeriesInterpolation {
    Step,
    Linear,
}

impl From<PySeriesInterpolation> for CoreInterp {
    fn from(v: PySeriesInterpolation) -> Self {
        match v {
            PySeriesInterpolation::Step => CoreInterp::Step,
            PySeriesInterpolation::Linear => CoreInterp::Linear,
        }
    }
}

#[pymethods]
impl PySeriesInterpolation {
    #[classattr]
    const STEP: Self = Self::Step;
    #[classattr]
    const LINEAR: Self = Self::Linear;
}

#[pyclass(name = "MarketScalar")]
#[derive(Clone)]
pub struct PyMarketScalar {
    inner: CoreMarketScalar,
}

#[pymethods]
impl PyMarketScalar {
    /// Construct a unitless scalar.
    #[staticmethod]
    pub fn unitless(value: f64) -> Self {
        Self { inner: CoreMarketScalar::Unitless(value) }
    }

    /// Construct a price scalar from a Money value.
    #[staticmethod]
    pub fn price(money: &PyMoney) -> Self {
        Self { inner: CoreMarketScalar::Price(money.inner()) }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            CoreMarketScalar::Unitless(v) => format!("MarketScalar.unitless({})", v),
            CoreMarketScalar::Price(m) => format!("MarketScalar.price(Money({}, {}))", m.amount(), m.currency()),
        }
    }
}

impl PyMarketScalar {
    pub fn inner(&self) -> CoreMarketScalar { self.inner.clone() }
}

#[pyclass(name = "ScalarTimeSeries")]
#[derive(Clone)]
pub struct PyScalarTimeSeries {
    inner: CoreSeries,
}

#[pymethods]
impl PyScalarTimeSeries {
    /// Create a scalar time series from observations.
    /// observations: List[Tuple[Date, float]], currency: Optional[Currency] = None
    #[new]
    #[pyo3(signature = (id, observations, currency=None, interpolation=None))]
    pub fn new(
        id: String,
        observations: &Bound<'_, PyList>,
        currency: Option<PyCurrency>,
        interpolation: Option<PySeriesInterpolation>,
    ) -> PyResult<Self> {
        let mut obs = Vec::new();
        for item in observations.iter() {
            let (d, v) = item.extract::<(PyDate, f64)>()?;
            obs.push((d.inner(), v));
        }
        let series = CoreSeries::new(
            Box::leak(id.into_boxed_str()),
            obs,
            currency.map(|c| c.inner()),
        ).map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{}", e)))?;
        let inner = if let Some(interp) = interpolation { series.with_interpolation(interp.into()) } else { series };
        Ok(Self { inner })
    }

    /// Identifier
    #[getter]
    pub fn id(&self) -> String { self.inner.id().as_str().to_string() }

    /// Optional currency
    #[getter]
    pub fn currency(&self) -> Option<PyCurrency> { self.inner.currency().map(PyCurrency::from_inner) }

    /// Value on a given date
    pub fn value_on(&self, date: PyDate) -> PyResult<f64> {
        self.inner.value_on(date.inner()).map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{}", e)))
    }

    fn __repr__(&self) -> String {
        format!("ScalarTimeSeries(id='{}', currency={:?})", self.id(), self.currency())
    }
}

impl PyScalarTimeSeries { pub fn inner(&self) -> CoreSeries { self.inner.clone() } }


