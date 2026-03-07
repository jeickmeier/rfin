//! Scalar market data bindings: unitless/price scalars and generic time series.
//!
//! Provides `MarketScalar` for single quotes (unitless or money-denominated) and
//! `ScalarTimeSeries` for dated observations with configurable interpolation.
//! Use `SeriesInterpolation` to control step vs linear behavior between points.
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::PyMoney;
use crate::errors::{core_to_py, PyContext};
use finstack_core::market_data::scalars::{
    InflationIndex, InflationInterpolation, InflationLag, MarketScalar, ScalarTimeSeries,
    SeriesInterpolation,
};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyFloat, PyIterator, PyList, PyModule, PyType};
use pyo3::Bound;
use pyo3::Py;
use std::sync::Arc;
use time::Date;

/// Parse an iterable of Python `datetime.date` into a Vec<Date>.
///
/// Parameters
/// ----------
/// dates : Iterable[datetime.date]
///     Python iterable of date objects.
///
/// Returns
/// -------
/// list[Date]
///     Rust dates converted from the input.
fn parse_dates_sequence(_py: Python<'_>, dates: &Bound<'_, PyAny>) -> PyResult<Vec<Date>> {
    let iter = PyIterator::from_object(dates)?;
    let mut result = Vec::new();
    for item in iter {
        let bound = item?;
        let date = py_to_date(&bound)?;
        result.push(date);
    }
    Ok(result)
}

/// Parse an iterable of `(date, value)` tuples into typed observations.
///
/// Parameters
/// ----------
/// observations : Iterable[tuple[datetime.date, float]]
///     Sequence of `(date, value)` pairs.
///
/// Returns
/// -------
/// list[tuple[Date, float]]
///     Converted observations.
fn parse_observations(
    py: Python<'_>,
    observations: &Bound<'_, PyAny>,
) -> PyResult<Vec<(Date, f64)>> {
    let iter = PyIterator::from_object(observations)?;
    let mut result = Vec::new();
    for item in iter {
        let tuple = item?;
        let (date_obj, value): (Py<PyAny>, f64) = tuple.extract().map_err(|_| {
            PyTypeError::new_err("observations must be iterable of (date, value) pairs")
        })?;
        let bound = date_obj.bind(py);
        let date = py_to_date(bound)?;
        result.push((date, value));
    }
    Ok(result)
}

/// Enumeration of interpolation styles for scalar time series.
///
/// Parameters
/// ----------
/// None
///     Use class attributes (e.g. :attr:`SeriesInterpolation.STEP`) or :py:meth:`SeriesInterpolation.from_name`.
///
/// Returns
/// -------
/// SeriesInterpolation
///     Enum value passed to :class:`ScalarTimeSeries`.
#[pyclass(
    module = "finstack.core.market_data.scalars",
    name = "SeriesInterpolation",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PySeriesInterpolation {
    pub(crate) inner: SeriesInterpolation,
}

impl PySeriesInterpolation {
    pub(crate) const fn new(inner: SeriesInterpolation) -> Self {
        Self { inner }
    }

    fn label(self) -> &'static str {
        match self.inner {
            SeriesInterpolation::Step => "step",
            SeriesInterpolation::Linear => "linear",
        }
    }
}

#[pymethods]
impl PySeriesInterpolation {
    #[classattr]
    const STEP: Self = Self {
        inner: SeriesInterpolation::Step,
    };

    #[classattr]
    const LINEAR: Self = Self {
        inner: SeriesInterpolation::Linear,
    };

    /// Parse an interpolation style from a string literal.
    ///
    /// Parameters
    /// ----------
    /// name : str
    ///     Interpolation label (``"step"`` or ``"linear"``).
    ///
    /// Returns
    /// -------
    /// SeriesInterpolation
    ///     Interpolation style matching ``name``.
    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        match name.to_ascii_lowercase().as_str() {
            "step" => Ok(Self::new(SeriesInterpolation::Step)),
            "linear" => Ok(Self::new(SeriesInterpolation::Linear)),
            other => Err(PyValueError::new_err(format!(
                "Unknown interpolation style: {other}"
            ))),
        }
    }

    /// Canonical label for the interpolation style.
    ///
    /// Returns
    /// -------
    /// str
    ///     Snake-case style label (``"step"`` or ``"linear"``).
    #[getter]
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("SeriesInterpolation('{}')", self.label())
    }

    fn __str__(&self) -> &'static str {
        self.label()
    }
}

/// Wrapper around either a unitless scalar or a money-denominated price.
///
/// Parameters
/// ----------
/// value : float, optional
///     Unitless scalar provided via :py:meth:`MarketScalar.unitless`.
/// money : Money, optional
///     Money amount provided via :py:meth:`MarketScalar.price`.
///
/// Returns
/// -------
/// MarketScalar
///     Scalar wrapper that records unit information.
#[pyclass(
    module = "finstack.core.market_data.scalars",
    name = "MarketScalar",
    unsendable,
    from_py_object
)]
#[derive(Clone)]
pub struct PyMarketScalar {
    pub(crate) inner: MarketScalar,
}

impl PyMarketScalar {
    pub(crate) fn new(inner: MarketScalar) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMarketScalar {
    /// Create a scalar without currency, typically used for spreads or ratios.
    ///
    /// Parameters
    /// ----------
    /// value : float
    ///     Numeric value without currency.
    ///
    /// Returns
    /// -------
    /// MarketScalar
    ///     Unitless scalar wrapper.
    #[classmethod]
    #[pyo3(text_signature = "(cls, value)")]
    fn unitless(_cls: &Bound<'_, PyType>, value: f64) -> Self {
        Self::new(MarketScalar::Unitless(value))
    }

    /// Create a price scalar from a money amount.
    ///
    /// Parameters
    /// ----------
    /// money : Money
    ///     Monetary value to wrap.
    ///
    /// Returns
    /// -------
    /// MarketScalar
    ///     Price scalar carrying currency information.
    #[classmethod]
    #[pyo3(text_signature = "(cls, money)")]
    fn price(_cls: &Bound<'_, PyType>, money: &PyMoney) -> Self {
        Self::new(MarketScalar::Price(money.inner))
    }

    /// Whether the scalar carries no currency information.
    ///
    /// Returns
    /// -------
    /// bool
    ///     ``True`` if the scalar is unitless.
    #[getter]
    fn is_unitless(&self) -> bool {
        matches!(self.inner, MarketScalar::Unitless(_))
    }

    /// Whether the scalar is backed by a :class:`Money` value.
    ///
    /// Returns
    /// -------
    /// bool
    ///     ``True`` if the scalar contains a price.
    #[getter]
    fn is_price(&self) -> bool {
        matches!(self.inner, MarketScalar::Price(_))
    }

    /// Return the underlying numeric or :class:`Money` value.
    ///
    /// Returns
    /// -------
    /// float or Money
    ///     Unitless value or money instance depending on the scalar type.
    #[getter]
    fn value(&self) -> PyResult<Py<PyAny>> {
        Python::attach(|py| match &self.inner {
            MarketScalar::Unitless(v) => Ok(PyFloat::new(py, *v).into()),
            MarketScalar::Price(m) => {
                let obj = Py::new(py, PyMoney::new(*m))?;
                Ok(obj.into())
            }
        })
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            MarketScalar::Unitless(v) => format!("MarketScalar.unitless({v})"),
            MarketScalar::Price(m) => format!("MarketScalar.get_price({m})"),
        }
    }
}

/// Daily or ad-hoc observations of scalar market data (rates, spreads, prices).
///
/// Parameters
/// ----------
/// id : str
///     Identifier for the series.
/// observations : Iterable[tuple[datetime.date, float]]
///     Date/value observations used to build the series.
/// currency : Currency, optional
///     Currency attached to price-based series.
/// interpolation : SeriesInterpolation, optional
///     Interpolation mode between observations.
///
/// Returns
/// -------
/// ScalarTimeSeries
///     Time series wrapper providing value lookup helpers.
#[pyclass(
    module = "finstack.core.market_data.scalars",
    name = "ScalarTimeSeries",
    unsendable,
    from_py_object
)]
#[derive(Clone)]
pub struct PyScalarTimeSeries {
    pub(crate) inner: Arc<ScalarTimeSeries>,
}

/// Inflation index observations with interpolation and lag conventions.
#[pyclass(
    module = "finstack.core.market_data.scalars",
    name = "InflationIndex",
    from_py_object
)]
#[derive(Clone)]
pub struct PyInflationIndex {
    pub(crate) inner: Arc<InflationIndex>,
}

impl PyInflationIndex {
    pub(crate) fn new_arc(inner: Arc<InflationIndex>) -> Self {
        Self { inner }
    }
}

impl PyScalarTimeSeries {
    pub(crate) fn new_arc(inner: ScalarTimeSeries) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pymethods]
impl PyScalarTimeSeries {
    #[pyo3(signature = (id, observations, currency=None, interpolation=None))]
    #[new]
    #[pyo3(text_signature = "(id, observations, /, *, currency=None, interpolation=None)")]
    /// Create a scalar time series from ``(date, value)`` observations.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Series identifier.
    /// observations : Iterable[tuple[datetime.date, float]]
    ///     Input observations in chronological order.
    /// currency : Currency, optional
    ///     Currency associated with the values (for price series).
    /// interpolation : SeriesInterpolation, optional
    ///     Interpolation style between observations.
    ///
    /// Returns
    /// -------
    /// ScalarTimeSeries
    ///     Time series supporting value lookups.
    ///
    /// Examples
    /// --------
    /// >>> series = ScalarTimeSeries(
    /// ...     "CPI",
    /// ...     [(date(2024, 1, 31), 300.0), (date(2024, 2, 29), 301.2)],
    /// ...     interpolation=SeriesInterpolation.LINEAR,
    /// ... )
    fn ctor(
        py: Python<'_>,
        id: &str,
        observations: Bound<'_, PyAny>,
        currency: Option<&PyCurrency>,
        interpolation: Option<PySeriesInterpolation>,
    ) -> PyResult<Self> {
        let parsed = parse_observations(py, &observations)?;
        if parsed.is_empty() {
            return Err(PyValueError::new_err(
                "observations must contain at least one entry",
            ));
        }
        let mut series =
            ScalarTimeSeries::new(id, parsed, currency.map(|c| c.inner)).map_err(core_to_py)?;
        if let Some(mode) = interpolation {
            series = series.with_interpolation(mode.inner);
        }
        Ok(Self::new_arc(series))
    }

    /// Override the interpolation mode (step or linear) for the series.
    ///
    /// Parameters
    /// ----------
    /// interpolation : SeriesInterpolation
    ///     New interpolation style.
    ///
    /// Returns
    /// -------
    /// None
    #[pyo3(text_signature = "(self, interpolation)")]
    fn set_interpolation(&mut self, interpolation: PySeriesInterpolation) {
        let updated = self
            .inner
            .as_ref()
            .clone()
            .with_interpolation(interpolation.inner);
        self.inner = Arc::new(updated);
    }

    /// Series identifier.
    ///
    /// Returns
    /// -------
    /// str
    ///     Series id.
    #[getter]
    fn id(&self) -> String {
        self.inner.id().to_string()
    }

    /// Optional reporting currency for price-based series.
    ///
    /// Returns
    /// -------
    /// Currency or None
    ///     Currency if present.
    #[getter]
    fn currency(&self) -> Option<PyCurrency> {
        self.inner.currency().map(PyCurrency::new)
    }

    /// Interpolation style applied between observations.
    ///
    /// Returns
    /// -------
    /// SeriesInterpolation
    ///     Current interpolation mode.
    #[getter]
    fn interpolation(&self) -> PySeriesInterpolation {
        PySeriesInterpolation::new(self.inner.interpolation())
    }

    /// Value on a single calendar date (interpolated if necessary).
    ///
    /// Parameters
    /// ----------
    /// date : datetime.date
    ///     Date at which to evaluate the series.
    ///
    /// Returns
    /// -------
    /// float
    ///     Interpolated value on ``date``.
    #[pyo3(text_signature = "(self, date)")]
    fn value_on(&self, date: Bound<'_, PyAny>) -> PyResult<f64> {
        let d = py_to_date(&date).context("date")?;
        self.inner.value_on(d).map_err(core_to_py)
    }

    /// Values on many dates; returns a list aligned with the input iterable.
    ///
    /// Parameters
    /// ----------
    /// dates : Iterable[datetime.date]
    ///     Dates to evaluate.
    ///
    /// Returns
    /// -------
    /// list[float]
    ///     Values aligned with the input order.
    #[pyo3(text_signature = "(self, dates)")]
    fn values_on(&self, py: Python<'_>, dates: Bound<'_, PyAny>) -> PyResult<Vec<f64>> {
        let parsed = parse_dates_sequence(py, &dates)?;
        self.inner.values_on(&parsed).map_err(core_to_py)
    }
}

#[pymethods]
impl PyInflationIndex {
    #[new]
    #[pyo3(signature = (id, observations, currency))]
    #[pyo3(text_signature = "(id, observations, currency)")]
    fn ctor(
        py: Python<'_>,
        id: &str,
        observations: Bound<'_, PyAny>,
        currency: &PyCurrency,
    ) -> PyResult<Self> {
        let parsed = parse_observations(py, &observations)?;
        let index = InflationIndex::new(id, parsed, currency.inner).map_err(core_to_py)?;
        Ok(Self::new_arc(Arc::new(index)))
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.currency)
    }

    #[getter]
    fn interpolation(&self) -> String {
        match self.inner.interpolation {
            InflationInterpolation::Step => "step".to_string(),
            InflationInterpolation::Linear => "linear".to_string(),
            _ => "step".to_string(),
        }
    }

    #[getter]
    fn lag_months(&self) -> Option<u8> {
        match self.inner.lag() {
            InflationLag::Months(m) => Some(m),
            _ => None,
        }
    }

    #[getter]
    fn observations(&self, py: Python<'_>) -> PyResult<Vec<(Py<PyAny>, f64)>> {
        self.inner
            .observations()
            .into_iter()
            .map(|(date, value)| Ok((date_to_py(py, date)?, value)))
            .collect()
    }

    #[pyo3(text_signature = "(self, date)")]
    fn value_on(&self, date: Bound<'_, PyAny>) -> PyResult<f64> {
        let d = py_to_date(&date).context("date")?;
        self.inner.value_on(d).map_err(core_to_py)
    }

    #[pyo3(text_signature = "(self, base_date, settle_date)")]
    fn ratio(&self, base_date: Bound<'_, PyAny>, settle_date: Bound<'_, PyAny>) -> PyResult<f64> {
        let base = py_to_date(&base_date).context("base_date")?;
        let settle = py_to_date(&settle_date).context("settle_date")?;
        self.inner.ratio(base, settle).map_err(core_to_py)
    }

    #[pyo3(text_signature = "(self, interpolation)")]
    fn with_interpolation(&self, interpolation: &str) -> PyResult<Self> {
        let mode = match interpolation.to_ascii_lowercase().as_str() {
            "step" => InflationInterpolation::Step,
            "linear" => InflationInterpolation::Linear,
            other => {
                return Err(PyValueError::new_err(format!(
                    "Unknown inflation interpolation: {other}"
                )))
            }
        };
        let updated = self.inner.as_ref().clone().with_interpolation(mode);
        Ok(Self::new_arc(Arc::new(updated)))
    }

    #[pyo3(text_signature = "(self, months)")]
    fn with_lag_months(&self, months: u8) -> Self {
        let updated = self
            .inner
            .as_ref()
            .clone()
            .with_lag(InflationLag::Months(months));
        Self::new_arc(Arc::new(updated))
    }

    fn __repr__(&self) -> String {
        format!("InflationIndex(id='{}')", self.inner.id)
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "scalars")?;
    module.setattr(
        "__doc__",
        "Scalar market data primitives: single-value quotes and generic time series.",
    )?;
    module.add_class::<PySeriesInterpolation>()?;
    module.add_class::<PyMarketScalar>()?;
    module.add_class::<PyScalarTimeSeries>()?;
    module.add_class::<PyInflationIndex>()?;
    let exports = [
        "SeriesInterpolation",
        "MarketScalar",
        "ScalarTimeSeries",
        "InflationIndex",
    ];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
