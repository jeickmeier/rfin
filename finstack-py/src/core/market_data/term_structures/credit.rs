use crate::core::common::args::extract_float_pairs;
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::dates::PyDayCount;
use crate::errors::{core_to_py, PyContext};
use finstack_core::market_data::term_structures::BaseCorrelationCurve;
use finstack_core::market_data::term_structures::CreditIndexData;
use finstack_core::market_data::term_structures::InflationCurve;
use finstack_core::market_data::term_structures::{HazardCurve, Seniority};
use finstack_core::math::interp::InterpStyle;
use finstack_core::HashMap;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::PyRef;
use std::str::FromStr;
use std::sync::Arc;

use super::parse_day_count;
use crate::core::common::args::parse_interp_style;

fn parse_seniority(value: Option<&str>) -> PyResult<Option<Seniority>> {
    match value {
        None => Ok(None),
        Some(name) => Seniority::from_str(name)
            .map(Some)
            .map_err(|e| PyValueError::new_err(e.to_string())),
    }
}

/// Credit hazard curve with piecewise-constant survival probabilities.
///
/// Parameters
/// ----------
/// id : str
///     Identifier for the hazard curve.
/// base_date : datetime.date
///     Anchor date for the survival curve.
/// knots : list[tuple[float, float]]
///     `(time, hazard_rate)` pairs.
/// recovery_rate : float, optional
///     Recovery assumption to embed in the curve.
/// day_count : DayCount, optional
///     Day-count convention used when converting dates.
/// issuer : str, optional
///     Descriptive issuer label.
/// seniority : str, optional
///     Seniority string such as ``"senior"`` or ``"subordinated"``.
/// currency : Currency, optional
///     Currency binding for par spread points.
/// par_points : list[tuple[float, float]], optional
///     Par spread inputs used during calibration.
///
/// Returns
/// -------
/// HazardCurve
///     Hazard curve wrapper offering survival and default probability methods.
#[pyclass(
    module = "finstack.core.market_data.term_structures",
    name = "HazardCurve",
    from_py_object
)]
#[derive(Clone)]
pub struct PyHazardCurve {
    pub(crate) inner: Arc<HazardCurve>,
}

impl PyHazardCurve {
    pub(crate) fn new_arc(inner: Arc<HazardCurve>) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyHazardCurve {
    /// Construct a hazard (default intensity) curve from `(time, hazard)` knots.
    #[new]
    #[pyo3(signature = (id, base_date, knots, recovery_rate=None, day_count=None, issuer=None, seniority=None, currency=None, par_points=None))]
    #[allow(clippy::too_many_arguments)]
    fn ctor(
        py: Python<'_>,
        id: &str,
        base_date: Bound<'_, PyAny>,
        knots: Bound<'_, PyAny>,
        recovery_rate: Option<f64>,
        day_count: Option<Bound<'_, PyAny>>,
        issuer: Option<&str>,
        seniority: Option<&str>,
        currency: Option<PyRef<PyCurrency>>,
        par_points: Option<Vec<(f64, f64)>>,
    ) -> PyResult<Self> {
        let knots_vec = extract_float_pairs(&knots)?;
        if knots_vec.is_empty() {
            return Err(PyValueError::new_err(
                "knots must contain at least one (time, hazard) pair",
            ));
        }
        let base = py_to_date(&base_date).context("base_date")?;
        let mut builder = HazardCurve::builder(id).base_date(base).knots(knots_vec);
        if let Some(rr) = recovery_rate {
            builder = builder.recovery_rate(rr);
        }
        if let Some(dc) = parse_day_count(day_count)? {
            builder = builder.day_count(dc);
        }
        if let Some(name) = issuer {
            builder = builder.issuer(name.to_string());
        }
        if let Some(sen) = parse_seniority(seniority)? {
            builder = builder.seniority(sen);
        }
        if let Some(ccy) = currency {
            builder = builder.currency(ccy.inner);
        }
        if let Some(points) = par_points {
            builder = builder.par_spreads(points);
        }
        let curve = py.detach(|| builder.build()).map_err(core_to_py)?;
        Ok(Self::new_arc(Arc::new(curve)))
    }

    /// Return the curve identifier.
    ///
    /// Returns
    /// -------
    /// str
    ///     Hazard curve id.
    #[getter]
    fn id(&self) -> String {
        self.inner.id().to_string()
    }

    /// Base date used for survival probability calculations.
    ///
    /// Returns
    /// -------
    /// datetime.date
    ///     Hazard curve base date.
    #[getter]
    fn base_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.base_date())
    }

    /// Assumed recovery rate for defaulted exposures.
    ///
    /// Returns
    /// -------
    /// float
    ///     Recovery rate expressed as a decimal.
    #[getter]
    fn recovery_rate(&self) -> f64 {
        self.inner.recovery_rate()
    }

    /// Day-count convention used to convert dates into year fractions.
    ///
    /// Returns
    /// -------
    /// DayCount
    ///     Day-count convention for this hazard curve.
    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count())
    }

    /// Knot points for hazard rates as ``(time, hazard)`` pairs.
    ///
    /// Returns
    /// -------
    /// list[tuple[float, float]]
    ///     Hazard rate knot points.
    #[getter]
    fn points(&self) -> Vec<(f64, f64)> {
        self.inner.knot_points().collect()
    }

    /// Optional par spread inputs used for calibration.
    ///
    /// Returns
    /// -------
    /// list[tuple[float, float]]
    ///     Par spread knots.
    #[getter]
    fn par_spreads(&self) -> Vec<(f64, f64)> {
        self.inner.par_spread_points().collect()
    }

    /// Survival probability ``S(t)`` at time ``t`` in years.
    ///
    /// Parameters
    /// ----------
    /// t : float
    ///     Time in years.
    ///
    /// Returns
    /// -------
    /// float
    ///     Survival probability to ``t``.
    #[pyo3(text_signature = "(self, t)")]
    fn survival(&self, t: f64) -> f64 {
        self.inner.sp(t)
    }

    /// Default probability over the interval ``(t1, t2)``.
    ///
    /// Parameters
    /// ----------
    /// t1 : float
    ///     Start time in years.
    /// t2 : float
    ///     End time in years.
    ///
    /// Returns
    /// -------
    /// float
    ///     Default probability between ``t1`` and ``t2``.
    #[pyo3(text_signature = "(self, t1, t2)")]
    fn default_prob(&self, t1: f64, t2: f64) -> PyResult<f64> {
        self.inner.default_prob(t1, t2).map_err(core_to_py)
    }
}

/// Inflation curve used for CPI projections.
///
/// Parameters
/// ----------
/// id : str
///     Identifier for the inflation curve.
/// base_date : datetime.date
///     Anchor date corresponding to ``t = 0``.
/// base_cpi : float
///     CPI level anchoring the curve.
/// knots : list[tuple[float, float]]
///     `(time, cpi_level)` points.
/// interp : str, optional
///     Interpolation style label (defaults to ``"log_linear"``).
///
/// Returns
/// -------
/// InflationCurve
///     Inflation curve wrapper exposing CPI and inflation rate calculations.
#[pyclass(
    module = "finstack.core.market_data.term_structures",
    name = "InflationCurve",
    from_py_object
)]
#[derive(Clone)]
pub struct PyInflationCurve {
    pub(crate) inner: Arc<InflationCurve>,
}

impl PyInflationCurve {
    pub(crate) fn new_arc(inner: Arc<InflationCurve>) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyInflationCurve {
    /// Create an inflation curve from `(time, CPI level)` points.
    #[new]
    #[pyo3(signature = (id, base_date, base_cpi, knots, interp=None))]
    fn ctor(
        py: Python<'_>,
        id: &str,
        base_date: Bound<'_, PyAny>,
        base_cpi: f64,
        knots: Bound<'_, PyAny>,
        interp: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let base = py_to_date(&base_date)?;
        let knots_vec = extract_float_pairs(&knots)?;
        if knots_vec.is_empty() {
            return Err(PyValueError::new_err("knots must not be empty"));
        }
        let style = parse_interp_style(interp.as_ref(), InterpStyle::LogLinear)?;
        let builder = InflationCurve::builder(id)
            .base_date(base)
            .base_cpi(base_cpi)
            .knots(knots_vec)
            .interp(style);
        let curve = py.detach(|| builder.build()).map_err(core_to_py)?;
        Ok(Self::new_arc(Arc::new(curve)))
    }

    /// Return the curve identifier.
    ///
    /// Returns
    /// -------
    /// str
    ///     Inflation curve id.
    #[getter]
    fn id(&self) -> String {
        self.inner.id().to_string()
    }

    /// Base CPI level anchoring the curve.
    ///
    /// Returns
    /// -------
    /// float
    ///     Base CPI level.
    #[getter]
    fn base_cpi(&self) -> f64 {
        self.inner.base_cpi()
    }

    /// Knot points for CPI levels as ``(time, level)``.
    ///
    /// Returns
    /// -------
    /// list[tuple[float, float]]
    ///     CPI level knot points.
    #[getter]
    fn points(&self) -> Vec<(f64, f64)> {
        self.inner
            .knots()
            .iter()
            .zip(self.inner.cpi_levels().iter())
            .map(|(&t, &level)| (t, level))
            .collect()
    }

    /// CPI level at time ``t`` in years.
    ///
    /// Parameters
    /// ----------
    /// t : float
    ///     Time in years from the base date.
    ///
    /// Returns
    /// -------
    /// float
    ///     CPI level at ``t``.
    #[pyo3(text_signature = "(self, t)")]
    fn cpi(&self, t: f64) -> f64 {
        self.inner.cpi(t)
    }

    /// Annualized inflation rate between two maturities.
    ///
    /// Parameters
    /// ----------
    /// t1 : float
    ///     Start time in years.
    /// t2 : float
    ///     End time in years.
    ///
    /// Returns
    /// -------
    /// float
    ///     Implied inflation rate between ``t1`` and ``t2``.
    #[pyo3(text_signature = "(self, t1, t2)")]
    fn inflation_rate(&self, t1: f64, t2: f64) -> f64 {
        self.inner.inflation_rate(t1, t2)
    }
}

/// Base correlation curve used for structured credit pricing.
///
/// Parameters
/// ----------
/// id : str
///     Identifier for the base correlation curve.
/// points : list[tuple[float, float]]
///     `(detachment_pct, correlation)` pairs in ascending detachment order.
///
/// Returns
/// -------
/// BaseCorrelationCurve
///     Base correlation wrapper capable of interpolating tranche correlations.
#[pyclass(
    module = "finstack.core.market_data.term_structures",
    name = "BaseCorrelationCurve",
    from_py_object
)]
#[derive(Clone)]
pub struct PyBaseCorrelationCurve {
    pub(crate) inner: Arc<BaseCorrelationCurve>,
}

impl PyBaseCorrelationCurve {
    pub(crate) fn new_arc(inner: Arc<BaseCorrelationCurve>) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyBaseCorrelationCurve {
    /// Instantiate a base correlation curve from `(detachment, correlation)` points.
    #[new]
    fn ctor(py: Python<'_>, id: &str, points: Bound<'_, PyAny>) -> PyResult<Self> {
        let points_vec = extract_float_pairs(&points)?;
        if points_vec.len() < 2 {
            return Err(PyValueError::new_err(
                "points must contain at least two entries",
            ));
        }
        let curve = py
            .detach(|| BaseCorrelationCurve::builder(id).knots(points_vec).build())
            .map_err(core_to_py)?;
        Ok(Self::new_arc(Arc::new(curve)))
    }

    /// Return the curve identifier.
    ///
    /// Returns
    /// -------
    /// str
    ///     Base-correlation curve id.
    #[getter]
    fn id(&self) -> String {
        self.inner.id.to_string()
    }

    /// Underlying ``(detachment %, correlation)`` knot points.
    ///
    /// Returns
    /// -------
    /// list[tuple[float, float]]
    ///     Detachment percentage and correlation pairs.
    #[getter]
    fn points(&self) -> Vec<(f64, f64)> {
        self.inner
            .detachment_points()
            .iter()
            .zip(self.inner.correlations().iter())
            .map(|(&d, &c)| (d, c))
            .collect()
    }

    /// Base correlation for a tranche detachment percentage.
    ///
    /// Parameters
    /// ----------
    /// detachment_pct : float
    ///     Tranche detachment percentage (0-1).
    ///
    /// Returns
    /// -------
    /// float
    ///     Base correlation at the requested detachment.
    #[pyo3(text_signature = "(self, detachment_pct)")]
    fn correlation(&self, detachment_pct: f64) -> f64 {
        self.inner.correlation(detachment_pct)
    }
}

/// Aggregated market data for a standardized credit index (e.g. CDX, iTraxx).
///
/// Parameters
/// ----------
/// num_constituents : int
///     Number of constituents in the index.
/// recovery_rate : float
///     Recovery assumption for index constituents.
/// index_curve : HazardCurve
///     Aggregated hazard curve for the index.
/// base_correlation_curve : BaseCorrelationCurve
///     Base correlation surface for tranche pricing.
/// issuer_curves : dict[str, HazardCurve], optional
///     Optional mapping of issuer ids to hazard curves.
///
/// Returns
/// -------
/// CreditIndexData
///     Bundle of credit index market data.
#[pyclass(
    module = "finstack.core.market_data.term_structures",
    name = "CreditIndexData",
    from_py_object
)]
#[derive(Clone)]
pub struct PyCreditIndexData {
    pub(crate) inner: Arc<CreditIndexData>,
}

impl PyCreditIndexData {
    pub(crate) fn new_arc(inner: Arc<CreditIndexData>) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCreditIndexData {
    /// Aggregate standardized credit index data with shared curves and constituents.
    #[new]
    #[pyo3(signature = (num_constituents, recovery_rate, index_curve, base_correlation_curve, issuer_curves=None))]
    fn ctor(
        num_constituents: u16,
        recovery_rate: f64,
        index_curve: PyRef<PyHazardCurve>,
        base_correlation_curve: PyRef<PyBaseCorrelationCurve>,
        issuer_curves: Option<HashMap<String, PyHazardCurve>>,
    ) -> PyResult<Self> {
        let mut builder = CreditIndexData::builder()
            .num_constituents(num_constituents)
            .recovery_rate(recovery_rate)
            .index_credit_curve(index_curve.inner.clone())
            .base_correlation_curve(base_correlation_curve.inner.clone());

        if let Some(curves) = issuer_curves {
            let mapped: HashMap<String, Arc<HazardCurve>> = curves
                .into_iter()
                .map(|(k, v)| (k, v.inner.clone()))
                .collect();
            builder = builder.issuer_curves(mapped);
        }

        let data = builder.build().map_err(core_to_py)?;
        Ok(Self::new_arc(Arc::new(data)))
    }

    /// Number of entities included in the index.
    ///
    /// Returns
    /// -------
    /// int
    ///     Number of constituents.
    #[getter]
    fn num_constituents(&self) -> u16 {
        self.inner.num_constituents
    }

    /// Recovery rate assumption used for index calculations.
    ///
    /// Returns
    /// -------
    /// float
    ///     Recovery rate applied to index constituents.
    #[getter]
    fn recovery_rate(&self) -> f64 {
        self.inner.recovery_rate
    }

    /// Hazard curve representing the pooled index.
    ///
    /// Returns
    /// -------
    /// HazardCurve
    ///     Aggregate index hazard curve.
    #[getter]
    fn index_curve(&self) -> PyHazardCurve {
        PyHazardCurve::new_arc(self.inner.index_credit_curve.clone())
    }

    /// Base correlation curve associated with the index.
    ///
    /// Returns
    /// -------
    /// BaseCorrelationCurve
    ///     Base correlation curve used for tranche pricing.
    #[getter]
    fn base_correlation_curve(&self) -> PyBaseCorrelationCurve {
        PyBaseCorrelationCurve::new_arc(self.inner.base_correlation_curve.clone())
    }

    /// Whether issuer-level curves are available.
    ///
    /// Returns
    /// -------
    /// bool
    ///     ``True`` when individual issuer curves are populated.
    fn has_issuer_curves(&self) -> bool {
        self.inner.has_issuer_curves()
    }

    /// List of issuer identifiers when issuer curves are present.
    ///
    /// Returns
    /// -------
    /// list[str]
    ///     Issuer ids available in the map.
    fn issuer_ids(&self) -> Vec<String> {
        self.inner.issuer_ids()
    }

    /// Retrieve the hazard curve for a given issuer id.
    ///
    /// Parameters
    /// ----------
    /// issuer_id : str
    ///     Issuer identifier.
    ///
    /// Returns
    /// -------
    /// HazardCurve or None
    ///     Issuer-specific hazard curve if present.
    #[pyo3(text_signature = "(self, issuer_id)")]
    fn issuer_curve(&self, issuer_id: &str) -> Option<PyHazardCurve> {
        self.inner
            .issuer_credit_curves
            .as_ref()
            .and_then(|map| map.get(issuer_id).cloned())
            .map(PyHazardCurve::new_arc)
    }
}
