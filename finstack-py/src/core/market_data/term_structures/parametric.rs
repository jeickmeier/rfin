use crate::core::common::args::{parse_extrap_style, parse_interp_style, DayCountArg};
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::dates::PyDayCount;
use crate::errors::{core_to_py, PyContext};
use finstack_core::market_data::term_structures::BasisSpreadCurve;
use finstack_core::market_data::term_structures::FlatCurve;
use finstack_core::market_data::term_structures::{NelsonSiegelModel, NsVariant};
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::PyRef;
use std::sync::Arc;

use super::parse_day_count;

/// Flat forward/discount curve with a constant continuously compounded rate.
///
/// Convenience wrapper for quick valuations and testing.
///
/// Parameters
/// ----------
/// rate : float
///     Continuously compounded annual rate (decimal, e.g. 0.05 for 5%).
/// base_date : datetime.date
///     Reference date for the curve.
/// day_count : DayCount or str
///     Day-count convention for year fractions.
/// id : str
///     Curve identifier.
///
/// Returns
/// -------
/// FlatCurve
///     Flat curve exposing discount-factor evaluation.
#[pyclass(
    module = "finstack.core.market_data.term_structures",
    name = "FlatCurve",
    from_py_object
)]
#[derive(Clone)]
pub struct PyFlatCurve {
    pub(crate) inner: FlatCurve,
}

impl PyFlatCurve {
    pub(crate) fn new(inner: FlatCurve) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFlatCurve {
    /// Create a flat curve with a constant continuously compounded rate.
    ///
    /// Parameters
    /// ----------
    /// rate : float
    ///     Continuously compounded annual rate (e.g. 0.05 for 5%).
    /// base_date : datetime.date
    ///     Reference date for the curve.
    /// day_count : DayCount or str
    ///     Day-count convention for year fractions.
    /// id : str
    ///     Curve identifier.
    ///
    /// Returns
    /// -------
    /// FlatCurve
    ///     Flat curve wrapper.
    #[new]
    #[pyo3(signature = (rate, base_date, day_count, id))]
    fn ctor(
        rate: f64,
        base_date: Bound<'_, PyAny>,
        day_count: Bound<'_, PyAny>,
        id: &str,
    ) -> PyResult<Self> {
        let bd = py_to_date(&base_date).context("base_date")?;
        let dc = if let Ok(dc) = day_count.extract::<PyRef<PyDayCount>>() {
            dc.inner
        } else if let Ok(DayCountArg(inner)) = day_count.extract::<DayCountArg>() {
            inner
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "day_count must be DayCount or string",
            ));
        };
        Ok(Self::new(FlatCurve::new(rate, bd, dc, id)))
    }

    /// Return the curve identifier.
    #[getter]
    fn id(&self) -> String {
        use finstack_core::market_data::traits::TermStructure;
        self.inner.id().to_string()
    }

    /// The constant continuously compounded rate.
    #[getter]
    fn rate(&self) -> f64 {
        self.inner.rate()
    }

    /// Curve base date.
    #[getter]
    fn base_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        use finstack_core::market_data::traits::Discounting;
        date_to_py(py, self.inner.base_date())
    }

    /// Day-count convention.
    #[getter]
    fn day_count(&self) -> PyDayCount {
        use finstack_core::market_data::traits::Discounting;
        PyDayCount::new(self.inner.day_count())
    }

    /// Discount factor at time ``t`` years.
    ///
    /// Parameters
    /// ----------
    /// t : float
    ///     Time in years from the base date.
    ///
    /// Returns
    /// -------
    /// float
    ///     Discount factor ``exp(-rate * t)``.
    #[pyo3(text_signature = "(self, t)")]
    fn df(&self, t: f64) -> f64 {
        use finstack_core::market_data::traits::Discounting;
        self.inner.df(t)
    }
}

// ---------------------------------------------------------------------------
// BasisSpreadCurve
// ---------------------------------------------------------------------------

/// Basis spread curve for cross-currency and multi-curve frameworks.
///
/// Stores continuously compounded spread values between two discount curves
/// at discrete pillar points.
///
/// Parameters
/// ----------
/// id : str
///     Curve identifier.
/// base_date : datetime.date
///     Anchor date corresponding to ``t = 0``.
/// knots : list[float]
///     Knot times in year fractions (strictly increasing).
/// spreads : list[float]
///     Continuously compounded spread values at each knot.
/// day_count : DayCount or str, optional
///     Day-count convention (defaults to Act/365F).
/// interp : str, optional
///     Interpolation style (defaults to ``"linear"``).
/// extrapolation : str, optional
///     Extrapolation policy (defaults to ``"flat_zero"``).
///
/// Returns
/// -------
/// BasisSpreadCurve
///     Basis spread curve with interpolation.
#[pyclass(
    module = "finstack.core.market_data.term_structures",
    name = "BasisSpreadCurve",
    from_py_object
)]
#[derive(Clone)]
pub struct PyBasisSpreadCurve {
    pub(crate) inner: Arc<BasisSpreadCurve>,
}

impl PyBasisSpreadCurve {
    pub(crate) fn new_arc(inner: Arc<BasisSpreadCurve>) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyBasisSpreadCurve {
    /// Construct a basis spread curve from knot times and spread values.
    #[new]
    #[pyo3(signature = (id, base_date, knots, spreads, day_count=None, interp=None, extrapolation=None))]
    #[allow(clippy::too_many_arguments)]
    fn ctor(
        py: Python<'_>,
        id: &str,
        base_date: Bound<'_, PyAny>,
        knots: Vec<f64>,
        spreads: Vec<f64>,
        day_count: Option<Bound<'_, PyAny>>,
        interp: Option<Bound<'_, PyAny>>,
        extrapolation: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        if knots.len() != spreads.len() {
            return Err(PyValueError::new_err(
                "knots and spreads must have the same length",
            ));
        }
        if knots.is_empty() {
            return Err(PyValueError::new_err(
                "knots must contain at least one point",
            ));
        }
        let base = py_to_date(&base_date).context("base_date")?;
        let style = parse_interp_style(interp.as_ref(), InterpStyle::Linear)?;
        let extra = parse_extrap_style(extrapolation.as_ref(), ExtrapolationPolicy::FlatZero)?;
        let points: Vec<(f64, f64)> = knots.into_iter().zip(spreads).collect();
        let mut builder = BasisSpreadCurve::builder(id)
            .base_date(base)
            .knots(points)
            .interp(style)
            .extrapolation(extra);
        if let Some(dc) = parse_day_count(day_count)? {
            builder = builder.day_count(dc);
        }
        let curve = py.detach(|| builder.build()).map_err(core_to_py)?;
        Ok(Self::new_arc(Arc::new(curve)))
    }

    /// Return the curve identifier.
    #[getter]
    fn id(&self) -> String {
        self.inner.id().to_string()
    }

    /// Base date of the curve.
    #[getter]
    fn base_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.base_date())
    }

    /// Day-count convention used for time calculations.
    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count())
    }

    /// Interpolation style used by this curve.
    #[getter]
    fn interp_style(&self) -> String {
        format!("{:?}", self.inner.interp_style())
    }

    /// Extrapolation policy used by this curve.
    #[getter]
    fn extrapolation(&self) -> String {
        format!("{:?}", self.inner.extrapolation())
    }

    /// Knot times in year fractions.
    #[getter]
    fn knots(&self) -> Vec<f64> {
        self.inner.knots().to_vec()
    }

    /// Spread values at each knot.
    #[getter]
    fn spreads(&self) -> Vec<f64> {
        self.inner.spreads().to_vec()
    }

    /// Continuously compounded spread at time ``t`` (years from base date).
    ///
    /// Parameters
    /// ----------
    /// t : float
    ///     Time in years.
    ///
    /// Returns
    /// -------
    /// float
    ///     Spread value at ``t``.
    #[pyo3(text_signature = "(self, t)")]
    fn spread(&self, t: f64) -> f64 {
        self.inner.spread(t)
    }

    /// Roll the curve forward by ``days`` calendar days.
    ///
    /// Parameters
    /// ----------
    /// days : int
    ///     Number of calendar days to roll forward.
    ///
    /// Returns
    /// -------
    /// BasisSpreadCurve
    ///     A new curve with updated base date and shifted knots.
    #[pyo3(text_signature = "(self, days)")]
    fn roll_forward(&self, days: i64) -> PyResult<Self> {
        let rolled = self.inner.roll_forward(days).map_err(core_to_py)?;
        Ok(Self::new_arc(Arc::new(rolled)))
    }

    fn __repr__(&self) -> String {
        format!(
            "BasisSpreadCurve(id='{}', knots={})",
            self.inner.id(),
            self.inner.knots().len()
        )
    }
}

// ---------------------------------------------------------------------------
// Nelson-Siegel / Parametric Curve
// ---------------------------------------------------------------------------

/// Nelson-Siegel model variant selector.
///
/// Attributes
/// ----------
/// NS : NsVariant
///     Four-parameter Nelson-Siegel model.
/// NSS : NsVariant
///     Six-parameter Nelson-Siegel-Svensson model.
#[pyclass(
    module = "finstack.core.market_data.term_structures",
    name = "NsVariant",
    from_py_object
)]
#[derive(Clone)]
pub struct PyNsVariant {
    pub(crate) inner: NsVariant,
}

#[pymethods]
impl PyNsVariant {
    /// Four-parameter Nelson-Siegel variant.
    #[classattr]
    #[allow(non_snake_case)]
    fn NS() -> Self {
        Self {
            inner: NsVariant::Ns,
        }
    }

    /// Six-parameter Nelson-Siegel-Svensson variant.
    #[classattr]
    #[allow(non_snake_case)]
    fn NSS() -> Self {
        Self {
            inner: NsVariant::Nss,
        }
    }

    fn __repr__(&self) -> String {
        match self.inner {
            NsVariant::Ns => "NsVariant.NS".to_string(),
            NsVariant::Nss => "NsVariant.NSS".to_string(),
        }
    }
}

/// Nelson-Siegel parametric model for yield curve fitting.
///
/// Provides either the 4-parameter NS or 6-parameter NSS specification.
/// Use classmethods ``ns()`` or ``nss()`` to construct.
#[pyclass(
    module = "finstack.core.market_data.term_structures",
    name = "NelsonSiegelModel",
    from_py_object
)]
#[derive(Clone)]
pub struct PyNelsonSiegelModel {
    pub(crate) inner: NelsonSiegelModel,
}

#[pymethods]
impl PyNelsonSiegelModel {
    /// Construct a four-parameter Nelson-Siegel model.
    ///
    /// Parameters
    /// ----------
    /// beta0 : float
    ///     Long-term rate level.
    /// beta1 : float
    ///     Short-term component.
    /// beta2 : float
    ///     Medium-term hump.
    /// tau : float
    ///     Decay factor (must be > 0).
    ///
    /// Returns
    /// -------
    /// NelsonSiegelModel
    ///     NS model instance.
    #[staticmethod]
    fn ns(beta0: f64, beta1: f64, beta2: f64, tau: f64) -> Self {
        Self {
            inner: NelsonSiegelModel::Ns {
                beta0,
                beta1,
                beta2,
                tau,
            },
        }
    }

    /// Construct a six-parameter Nelson-Siegel-Svensson model.
    ///
    /// Parameters
    /// ----------
    /// beta0 : float
    ///     Long-term rate level.
    /// beta1 : float
    ///     Short-term component.
    /// beta2 : float
    ///     Medium-term hump.
    /// beta3 : float
    ///     Second hump.
    /// tau1 : float
    ///     First decay factor (must be > 0).
    /// tau2 : float
    ///     Second decay factor (must be > 0, distinct from tau1).
    ///
    /// Returns
    /// -------
    /// NelsonSiegelModel
    ///     NSS model instance.
    #[staticmethod]
    fn nss(beta0: f64, beta1: f64, beta2: f64, beta3: f64, tau1: f64, tau2: f64) -> Self {
        Self {
            inner: NelsonSiegelModel::Nss {
                beta0,
                beta1,
                beta2,
                beta3,
                tau1,
                tau2,
            },
        }
    }

    /// Compute the zero rate at time ``t`` using the parametric formula.
    ///
    /// Parameters
    /// ----------
    /// t : float
    ///     Time in years.
    ///
    /// Returns
    /// -------
    /// float
    ///     Continuously compounded zero rate.
    #[pyo3(text_signature = "(self, t)")]
    fn zero_rate(&self, t: f64) -> f64 {
        self.inner.zero_rate(t)
    }

    /// Compute the instantaneous forward rate at time ``t``.
    ///
    /// Parameters
    /// ----------
    /// t : float
    ///     Time in years.
    ///
    /// Returns
    /// -------
    /// float
    ///     Instantaneous forward rate.
    #[pyo3(text_signature = "(self, t)")]
    fn forward_rate(&self, t: f64) -> f64 {
        self.inner.forward_rate(t)
    }

    /// Number of parameters in this model (4 for NS, 6 for NSS).
    ///
    /// Returns
    /// -------
    /// int
    ///     Parameter count.
    #[pyo3(text_signature = "(self)")]
    fn num_params(&self) -> usize {
        self.inner.num_params()
    }

    /// Convert parameters to a flat list for optimizer consumption.
    ///
    /// Returns
    /// -------
    /// list[float]
    ///     Parameter values in canonical order.
    #[pyo3(text_signature = "(self)")]
    fn to_params_vec(&self) -> Vec<f64> {
        self.inner.to_params_vec()
    }

    /// Validate parameter constraints (tau > 0, tau1 != tau2, etc.).
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If parameter constraints are violated.
    #[pyo3(text_signature = "(self)")]
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(core_to_py)
    }

    /// Construct from a flat parameter vector and variant selector.
    ///
    /// Parameters
    /// ----------
    /// variant : NsVariant
    ///     Model variant (NS or NSS).
    /// params : list[float]
    ///     Flat parameter vector (length 4 for NS, 6 for NSS).
    ///
    /// Returns
    /// -------
    /// NelsonSiegelModel
    ///     Reconstructed model.
    #[staticmethod]
    fn from_params_vec(variant: PyRef<PyNsVariant>, params: Vec<f64>) -> PyResult<Self> {
        let model =
            NelsonSiegelModel::from_params_vec(variant.inner, &params).map_err(core_to_py)?;
        Ok(Self { inner: model })
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            NelsonSiegelModel::Ns {
                beta0,
                beta1,
                beta2,
                tau,
            } => format!(
                "NelsonSiegelModel.ns(beta0={beta0}, beta1={beta1}, beta2={beta2}, tau={tau})"
            ),
            NelsonSiegelModel::Nss {
                beta0,
                beta1,
                beta2,
                beta3,
                tau1,
                tau2,
            } => format!(
                "NelsonSiegelModel.nss(beta0={beta0}, beta1={beta1}, beta2={beta2}, beta3={beta3}, tau1={tau1}, tau2={tau2})"
            ),
        }
    }
}
