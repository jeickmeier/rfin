//! Python bindings for `finstack_core::market_data::term_structures` curve types.

use std::sync::Arc;

use finstack_core::dates::DayCount;
use finstack_core::market_data::term_structures::{
    DiscountCurve, ForwardCurve, HazardCurve, PriceCurve, VolatilityIndexCurve,
};
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};

use crate::bindings::core::dates::utils::{date_to_py, py_to_date};

use crate::errors::core_to_py;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parse a DayCount from a Python string like `"act_365f"`, `"act_360"`, etc.
fn parse_day_count(s: &str) -> PyResult<DayCount> {
    s.parse::<DayCount>()
        .map_err(|e| PyValueError::new_err(format!("Invalid day_count {s:?}: {e}")))
}

/// Parse an [`InterpStyle`] from a Python string.
fn parse_interp_style(s: &str) -> PyResult<InterpStyle> {
    s.parse::<InterpStyle>()
        .map_err(|e| PyValueError::new_err(format!("Invalid interp style {s:?}: {e}")))
}

/// Parse an [`ExtrapolationPolicy`] from a Python string.
fn parse_extrapolation(s: &str) -> PyResult<ExtrapolationPolicy> {
    s.parse::<ExtrapolationPolicy>()
        .map_err(|e| PyValueError::new_err(format!("Invalid extrapolation {s:?}: {e}")))
}

// ---------------------------------------------------------------------------
// PyDiscountCurve
// ---------------------------------------------------------------------------

/// Discount factor curve for present-value calculations.
///
/// Wraps [`DiscountCurve`] from `finstack-core`. Constructed via the builder
/// pattern using `(time, df)` knot pairs.
#[pyclass(
    name = "DiscountCurve",
    module = "finstack.core.market_data.curves",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyDiscountCurve {
    /// Shared Rust curve.
    pub(crate) inner: Arc<DiscountCurve>,
}

impl PyDiscountCurve {
    /// Build from an existing `Arc<DiscountCurve>`.
    pub(crate) fn from_inner(inner: Arc<DiscountCurve>) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyDiscountCurve {
    /// Construct a discount curve from knot points.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique curve identifier (e.g. ``"USD-OIS"``).
    /// base_date : datetime.date
    ///     Valuation date.
    /// knots : list[tuple[float, float]]
    ///     ``(time_years, discount_factor)`` pairs.
    /// interp : str, optional
    ///     Interpolation style (default ``"monotone_convex"``).
    /// extrapolation : str, optional
    ///     Extrapolation policy (default ``"flat_forward"``).
    /// day_count : str, optional
    ///     Day-count convention (default ``"act_365f"``).
    #[new]
    #[pyo3(signature = (id, base_date, knots, interp="monotone_convex", extrapolation="flat_forward", day_count="act_365f"))]
    fn new(
        id: &str,
        base_date: &Bound<'_, PyAny>,
        knots: Vec<(f64, f64)>,
        interp: &str,
        extrapolation: &str,
        day_count: &str,
    ) -> PyResult<Self> {
        let base = py_to_date(base_date)?;
        let style = parse_interp_style(interp)?;
        let extrap = parse_extrapolation(extrapolation)?;
        let dc = parse_day_count(day_count)?;

        let curve = DiscountCurve::builder(id)
            .base_date(base)
            .day_count(dc)
            .knots(knots)
            .interp(style)
            .extrapolation(extrap)
            .build()
            .map_err(core_to_py)?;

        Ok(Self {
            inner: Arc::new(curve),
        })
    }

    /// Discount factor at year fraction `t`.
    #[pyo3(text_signature = "(self, t)")]
    fn df(&self, t: f64) -> f64 {
        self.inner.df(t)
    }

    /// Continuously-compounded zero rate at year fraction `t`.
    #[pyo3(text_signature = "(self, t)")]
    fn zero(&self, t: f64) -> f64 {
        self.inner.zero(t)
    }

    /// Continuously-compounded forward rate between `t1` and `t2`.
    #[pyo3(text_signature = "(self, t1, t2)")]
    fn forward_rate(&self, t1: f64, t2: f64) -> PyResult<f64> {
        self.inner.forward(t1, t2).map_err(core_to_py)
    }

    /// Curve identifier string.
    #[getter]
    fn id(&self) -> &str {
        self.inner.id().as_str()
    }

    /// Valuation base date.
    #[getter]
    fn base_date<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        date_to_py(py, self.inner.base_date())
    }

    fn __repr__(&self) -> String {
        format!("DiscountCurve(id={:?})", self.inner.id().as_str())
    }
}

// ---------------------------------------------------------------------------
// PyForwardCurve
// ---------------------------------------------------------------------------

/// Forward rate curve for a floating-rate index with a fixed tenor.
///
/// Wraps [`ForwardCurve`] from `finstack-core`.
#[pyclass(
    name = "ForwardCurve",
    module = "finstack.core.market_data.curves",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyForwardCurve {
    /// Shared Rust curve.
    pub(crate) inner: Arc<ForwardCurve>,
}

impl PyForwardCurve {
    /// Build from an existing `Arc<ForwardCurve>`.
    pub(crate) fn from_inner(inner: Arc<ForwardCurve>) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyForwardCurve {
    /// Construct a forward rate curve from knot points.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique curve identifier (e.g. ``"USD-SOFR-3M"``).
    /// tenor : float
    ///     Index tenor in years (e.g. ``0.25`` for 3 months).
    /// knots : list[tuple[float, float]]
    ///     ``(time_years, forward_rate)`` pairs.
    /// base_date : datetime.date
    ///     Valuation date.
    /// day_count : str, optional
    ///     Day-count convention (default ``"act_360"``).
    /// interp : str, optional
    ///     Interpolation style (default ``"linear"``).
    /// extrapolation : str, optional
    ///     Extrapolation policy (default ``"flat_forward"``).
    #[new]
    #[pyo3(signature = (id, tenor, knots, base_date, day_count="act_360", interp="linear", extrapolation="flat_forward"))]
    fn new(
        id: &str,
        tenor: f64,
        knots: Vec<(f64, f64)>,
        base_date: &Bound<'_, PyAny>,
        day_count: &str,
        interp: &str,
        extrapolation: &str,
    ) -> PyResult<Self> {
        let base = py_to_date(base_date)?;
        let dc = parse_day_count(day_count)?;
        let style = parse_interp_style(interp)?;
        let extrap = parse_extrapolation(extrapolation)?;

        let curve = ForwardCurve::builder(id, tenor)
            .base_date(base)
            .day_count(dc)
            .knots(knots)
            .interp(style)
            .extrapolation(extrap)
            .build()
            .map_err(core_to_py)?;

        Ok(Self {
            inner: Arc::new(curve),
        })
    }

    /// Forward rate at year fraction `t`.
    #[pyo3(text_signature = "(self, t)")]
    fn rate(&self, t: f64) -> f64 {
        self.inner.rate(t)
    }

    /// Curve identifier string.
    #[getter]
    fn id(&self) -> &str {
        self.inner.id().as_str()
    }

    /// Valuation base date.
    #[getter]
    fn base_date<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        date_to_py(py, self.inner.base_date())
    }

    fn __repr__(&self) -> String {
        format!("ForwardCurve(id={:?})", self.inner.id().as_str())
    }
}

// ---------------------------------------------------------------------------
// PyHazardCurve
// ---------------------------------------------------------------------------

/// Credit hazard-rate curve for default probability modeling.
///
/// Wraps [`HazardCurve`] from `finstack-core`.
#[pyclass(
    name = "HazardCurve",
    module = "finstack.core.market_data.curves",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyHazardCurve {
    /// Shared Rust curve.
    pub(crate) inner: Arc<HazardCurve>,
}

impl PyHazardCurve {
    /// Build from an existing `Arc<HazardCurve>`.
    pub(crate) fn from_inner(inner: Arc<HazardCurve>) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyHazardCurve {
    /// Construct a hazard curve from knot points.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique curve identifier (e.g. ``"ACME-HZD"``).
    /// base_date : datetime.date
    ///     Valuation date.
    /// knots : list[tuple[float, float]]
    ///     ``(time_years, hazard_rate)`` pairs.
    /// recovery_rate : float, optional
    ///     Recovery rate (default ``0.4``).
    /// day_count : str, optional
    ///     Day-count convention (default ``"act_365f"``).
    #[new]
    #[pyo3(signature = (id, base_date, knots, recovery_rate=0.4, day_count="act_365f"))]
    fn new(
        id: &str,
        base_date: &Bound<'_, PyAny>,
        knots: Vec<(f64, f64)>,
        recovery_rate: f64,
        day_count: &str,
    ) -> PyResult<Self> {
        let base = py_to_date(base_date)?;
        let dc = parse_day_count(day_count)?;

        let curve = HazardCurve::builder(id)
            .base_date(base)
            .recovery_rate(recovery_rate)
            .day_count(dc)
            .knots(knots)
            .build()
            .map_err(core_to_py)?;

        Ok(Self {
            inner: Arc::new(curve),
        })
    }

    /// Survival probability at year fraction `t`.
    #[pyo3(text_signature = "(self, t)")]
    fn survival(&self, t: f64) -> f64 {
        self.inner.sp(t)
    }

    /// Instantaneous hazard rate at year fraction `t`.
    #[pyo3(text_signature = "(self, t)")]
    fn hazard_rate(&self, t: f64) -> f64 {
        self.inner.hazard_rate(t)
    }

    /// Curve identifier string.
    #[getter]
    fn id(&self) -> &str {
        self.inner.id().as_str()
    }

    /// Valuation base date.
    #[getter]
    fn base_date<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        date_to_py(py, self.inner.base_date())
    }

    fn __repr__(&self) -> String {
        format!("HazardCurve(id={:?})", self.inner.id().as_str())
    }
}

// ---------------------------------------------------------------------------
// PyPriceCurve
// ---------------------------------------------------------------------------

/// Forward price curve for commodities and other price-based assets.
///
/// Wraps [`PriceCurve`] from `finstack-core`.
#[pyclass(
    name = "PriceCurve",
    module = "finstack.core.market_data.curves",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyPriceCurve {
    /// Shared Rust curve.
    pub(crate) inner: Arc<PriceCurve>,
}

impl PyPriceCurve {
    /// Build from an existing `Arc<PriceCurve>`.
    pub(crate) fn from_inner(inner: Arc<PriceCurve>) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPriceCurve {
    /// Construct a price curve from knot points.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique curve identifier (e.g. ``"WTI-FORWARD"``).
    /// base_date : datetime.date
    ///     Valuation date.
    /// knots : list[tuple[float, float]]
    ///     ``(time_years, forward_price)`` pairs.
    /// extrapolation : str, optional
    ///     Extrapolation policy (default ``"flat_zero"``).
    /// interp : str, optional
    ///     Interpolation style (default ``"linear"``).
    /// day_count : str, optional
    ///     Day-count convention (default ``"act_365f"``).
    #[new]
    #[pyo3(signature = (id, base_date, knots, extrapolation="flat_zero", interp="linear", day_count="act_365f"))]
    fn new(
        id: &str,
        base_date: &Bound<'_, PyAny>,
        knots: Vec<(f64, f64)>,
        extrapolation: &str,
        interp: &str,
        day_count: &str,
    ) -> PyResult<Self> {
        let base = py_to_date(base_date)?;
        let extrap = parse_extrapolation(extrapolation)?;
        let style = parse_interp_style(interp)?;
        let dc = parse_day_count(day_count)?;

        let curve = PriceCurve::builder(id)
            .base_date(base)
            .day_count(dc)
            .knots(knots)
            .interp(style)
            .extrapolation(extrap)
            .build()
            .map_err(core_to_py)?;

        Ok(Self {
            inner: Arc::new(curve),
        })
    }

    /// Forward price at year fraction `t`.
    #[pyo3(text_signature = "(self, t)")]
    fn price(&self, t: f64) -> f64 {
        self.inner.price(t)
    }

    /// Curve identifier string.
    #[getter]
    fn id(&self) -> &str {
        self.inner.id().as_str()
    }

    /// Valuation base date.
    #[getter]
    fn base_date<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        date_to_py(py, self.inner.base_date())
    }

    fn __repr__(&self) -> String {
        format!("PriceCurve(id={:?})", self.inner.id().as_str())
    }
}

// ---------------------------------------------------------------------------
// PyVolatilityIndexCurve
// ---------------------------------------------------------------------------

/// Volatility index forward curve (e.g. VIX term structure).
///
/// Wraps [`VolatilityIndexCurve`] from `finstack-core`.
#[pyclass(
    name = "VolatilityIndexCurve",
    module = "finstack.core.market_data.curves",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyVolatilityIndexCurve {
    /// Shared Rust curve.
    pub(crate) inner: Arc<VolatilityIndexCurve>,
}

impl PyVolatilityIndexCurve {
    /// Build from an existing `Arc<VolatilityIndexCurve>`.
    pub(crate) fn from_inner(inner: Arc<VolatilityIndexCurve>) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyVolatilityIndexCurve {
    /// Construct a volatility index curve from knot points.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique curve identifier (e.g. ``"VIX"``).
    /// base_date : datetime.date
    ///     Valuation date.
    /// knots : list[tuple[float, float]]
    ///     ``(time_years, forward_level)`` pairs.
    /// extrapolation : str, optional
    ///     Extrapolation policy (default ``"flat_zero"``).
    /// interp : str, optional
    ///     Interpolation style (default ``"linear"``).
    /// day_count : str, optional
    ///     Day-count convention (default ``"act_365f"``).
    #[new]
    #[pyo3(signature = (id, base_date, knots, extrapolation="flat_zero", interp="linear", day_count="act_365f"))]
    fn new(
        id: &str,
        base_date: &Bound<'_, PyAny>,
        knots: Vec<(f64, f64)>,
        extrapolation: &str,
        interp: &str,
        day_count: &str,
    ) -> PyResult<Self> {
        let base = py_to_date(base_date)?;
        let extrap = parse_extrapolation(extrapolation)?;
        let style = parse_interp_style(interp)?;
        let dc = parse_day_count(day_count)?;

        let curve = VolatilityIndexCurve::builder(id)
            .base_date(base)
            .day_count(dc)
            .knots(knots)
            .interp(style)
            .extrapolation(extrap)
            .build()
            .map_err(core_to_py)?;

        Ok(Self {
            inner: Arc::new(curve),
        })
    }

    /// Forward volatility index level at year fraction `t`.
    #[pyo3(text_signature = "(self, t)")]
    fn forward_level(&self, t: f64) -> f64 {
        self.inner.forward_level(t)
    }

    /// Curve identifier string.
    #[getter]
    fn id(&self) -> &str {
        self.inner.id().as_str()
    }

    /// Valuation base date.
    #[getter]
    fn base_date<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        date_to_py(py, self.inner.base_date())
    }

    fn __repr__(&self) -> String {
        format!("VolatilityIndexCurve(id={:?})", self.inner.id().as_str())
    }
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

/// Register the `finstack.core.market_data.curves` submodule.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "curves")?;
    m.setattr(
        "__doc__",
        "Term structure curve bindings: discount, forward, hazard, price, vol-index.",
    )?;

    m.add_class::<PyDiscountCurve>()?;
    m.add_class::<PyForwardCurve>()?;
    m.add_class::<PyHazardCurve>()?;
    m.add_class::<PyPriceCurve>()?;
    m.add_class::<PyVolatilityIndexCurve>()?;

    let all = PyList::new(
        py,
        [
            "DiscountCurve",
            "ForwardCurve",
            "HazardCurve",
            "PriceCurve",
            "VolatilityIndexCurve",
        ],
    )?;
    m.setattr("__all__", all)?;

    parent.add_submodule(&m)?;

    let pkg: String = match parent.getattr("__package__") {
        Ok(attr) => match attr.extract::<String>() {
            Ok(s) => s,
            Err(_) => "finstack.core.market_data".to_string(),
        },
        Err(_) => "finstack.core.market_data".to_string(),
    };
    let qual = format!("{pkg}.curves");
    m.setattr("__package__", &qual)?;
    let sys = PyModule::import(py, "sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item(&qual, &m)?;

    Ok(())
}
