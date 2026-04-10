use crate::core::common::args::{extract_float_pairs, parse_extrap_style, parse_interp_style};
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::dates::PyDayCount;
use crate::errors::{core_to_py, PyContext};
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::market_data::term_structures::ForwardCurve;
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::sync::Arc;

use super::parse_day_count;

/// Discount curve wrapper supporting multiple interpolation and extrapolation styles.
///
/// Parameters
/// ----------
/// id : str
///     Identifier used to retrieve the curve later.
/// base_date : datetime.date
///     Anchor date corresponding to ``t = 0``.
/// knots : list[tuple[float, float]]
///     `(time, discount_factor)` pairs used to build the curve.
/// day_count : DayCount, optional
///     Day-count convention for converting dates to year fractions.
/// interp : str, optional
///     Interpolation style (defaults to ``"log_linear"`` which guarantees positive
///     forward rates). Other options: ``"linear"``, ``"monotone_convex"``, etc.
/// extrapolation : str, optional
///     Extrapolation policy name (defaults to ``"flat_forward"`` for smooth
///     instantaneous forwards beyond the last knot).
/// require_monotonic : bool, default True
///     Enforce monotonic discount factors across knots (set False to allow non-monotonic DFs).
///
/// Returns
/// -------
/// DiscountCurve
///     Curve object exposing discount factor, zero rate, and forward helpers.
#[pyclass(
    module = "finstack.core.market_data.term_structures",
    name = "DiscountCurve",
    from_py_object
)]
#[derive(Clone)]
pub struct PyDiscountCurve {
    pub(crate) inner: Arc<DiscountCurve>,
}

impl PyDiscountCurve {
    pub(crate) fn new_arc(inner: Arc<DiscountCurve>) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyDiscountCurve {
    /// Create a discount curve identified by ``id`` with ``(time, df)`` knot points.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Curve identifier.
    /// base_date : datetime.date
    ///     Date corresponding to ``t = 0``.
    /// knots : list[tuple[float, float]]
    ///     ``(time, discount_factor)`` pairs in ascending time order.
    /// day_count : DayCount, optional
    ///     Override the default Act/365F convention.
    /// interp : str, optional
    ///     Interpolation style (defaults to ``"log_linear"`` for positive forwards).
    ///     Other options: ``"linear"``, ``"monotone_convex"``, ``"cubic_hermite"``.
    /// extrapolation : str, optional
    ///     Extrapolation policy (defaults to ``"flat_forward"``).
    ///     Other option: ``"flat_zero"``.
    /// require_monotonic : bool, default True
    ///     Enforce monotonic discount factors across knots (set False to allow non-monotonic DFs).
    ///
    /// Returns
    /// -------
    /// DiscountCurve
    ///     Discount curve with pre-computed interpolation.
    #[new]
    #[pyo3(signature = (id, base_date, knots, day_count=None, interp=None, extrapolation=None, require_monotonic=true))]
    #[allow(clippy::too_many_arguments)]
    fn ctor(
        py: Python<'_>,
        id: &str,
        base_date: Bound<'_, PyAny>,
        knots: Bound<'_, PyAny>,
        day_count: Option<Bound<'_, PyAny>>,
        interp: Option<Bound<'_, PyAny>>,
        extrapolation: Option<Bound<'_, PyAny>>,
        require_monotonic: bool,
    ) -> PyResult<Self> {
        let knots_vec = extract_float_pairs(&knots)?;
        if knots_vec.len() < 2 {
            return Err(PyValueError::new_err(
                "knots must contain at least two (time, df) pairs",
            ));
        }
        let base = py_to_date(&base_date).context("base_date")?;
        let style = parse_interp_style(interp.as_ref(), InterpStyle::LogLinear)?;
        let extra = parse_extrap_style(extrapolation.as_ref(), ExtrapolationPolicy::FlatForward)?;
        let mut builder = DiscountCurve::builder(id)
            .base_date(base)
            .knots(knots_vec)
            .interp(style)
            .extrapolation(extra);
        if let Some(dc) = parse_day_count(day_count)? {
            builder = builder.day_count(dc);
        }
        if !require_monotonic {
            builder = builder.allow_non_monotonic();
        }
        let curve = py.detach(|| builder.build()).map_err(core_to_py)?;
        Ok(Self::new_arc(Arc::new(curve)))
    }

    /// Return the curve identifier.
    ///
    /// Returns
    /// -------
    /// str
    ///     Curve id used for registration.
    #[getter]
    fn id(&self) -> String {
        self.inner.id().to_string()
    }

    /// Base date used for discount factors expressed as ``(date -> df)``.
    ///
    /// Returns
    /// -------
    /// datetime.date
    ///     Curve base date.
    #[getter]
    fn base_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.base_date())
    }

    /// Day-count convention associated with the curve.
    ///
    /// Returns
    /// -------
    /// DayCount
    ///     Day-count convention used internally.
    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count())
    }

    /// Knot points backing the piecewise representation as ``(time, df)`` pairs.
    ///
    /// Returns
    /// -------
    /// list[tuple[float, float]]
    ///     Copy of times and discount factors.
    #[getter]
    fn points(&self) -> Vec<(f64, f64)> {
        self.inner
            .knots()
            .iter()
            .zip(self.inner.dfs().iter())
            .map(|(&t, &df)| (t, df))
            .collect()
    }

    /// Discount factor ``df(t)`` for the given time in years.
    ///
    /// Parameters
    /// ----------
    /// t : float
    ///     Time in years from the base date.
    ///
    /// Returns
    /// -------
    /// float
    ///     Discount factor at ``t``.
    #[pyo3(text_signature = "(self, t)")]
    fn df(&self, t: f64) -> f64 {
        self.inner.df(t)
    }

    /// Continuously compounded zero rate for maturity ``t`` (years).
    ///
    /// Parameters
    /// ----------
    /// t : float
    ///     Time in years from the base date.
    ///
    /// Returns
    /// -------
    /// float
    ///     Zero rate expressed in decimal form.
    #[pyo3(text_signature = "(self, t)")]
    fn zero(&self, t: f64) -> f64 {
        self.inner.zero(t)
    }

    /// Forward rate implied between ``t1`` and ``t2`` (years).
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
    ///     Forward rate for the interval ``(t1, t2)``.
    #[pyo3(text_signature = "(self, t1, t2)")]
    fn forward(&self, t1: f64, t2: f64) -> PyResult<f64> {
        self.inner.forward(t1, t2).map_err(core_to_py)
    }

    /// Annually compounded zero rate for maturity ``t`` (years).
    ///
    /// This is the bond equivalent yield convention commonly used by
    /// Bloomberg for displaying zero rates.
    ///
    /// Formula: ``r_annual = DF^(-1/t) - 1``
    ///
    /// Parameters
    /// ----------
    /// t : float
    ///     Time in years from the base date.
    ///
    /// Returns
    /// -------
    /// float
    ///     Zero rate with annual compounding, expressed in decimal form.
    ///
    /// Examples
    /// --------
    /// >>> curve = DiscountCurve("USD", base_date, [(0.0, 1.0), (1.0, 0.95)])
    /// >>> curve.zero_annual(1.0)  # ~5.26% for DF=0.95
    /// 0.05263157894736842
    #[pyo3(text_signature = "(self, t)")]
    fn zero_annual(&self, t: f64) -> f64 {
        self.inner.zero_annual(t)
    }

    /// Periodically compounded zero rate with ``n`` compounding periods per year.
    ///
    /// Common values for ``n``:
    ///
    /// - 1: Annual (same as ``zero_annual``)
    /// - 2: Semi-annual (US Treasury convention)
    /// - 4: Quarterly
    /// - 12: Monthly
    ///
    /// Formula: ``r_periodic = n * (DF^(-1/(n*t)) - 1)``
    ///
    /// Parameters
    /// ----------
    /// t : float
    ///     Time in years from the base date.
    /// n : int
    ///     Number of compounding periods per year.
    ///
    /// Returns
    /// -------
    /// float
    ///     Zero rate with periodic compounding, expressed in decimal form.
    ///
    /// Examples
    /// --------
    /// >>> curve = DiscountCurve("USD", base_date, [(0.0, 1.0), (1.0, 0.95)])
    /// >>> curve.zero_periodic(1.0, 2)  # Semi-annual compounding
    /// 0.05195190528383289
    #[pyo3(text_signature = "(self, t, n)")]
    fn zero_periodic(&self, t: f64, n: u32) -> f64 {
        self.inner.zero_periodic(t, n)
    }

    /// Discount factor on a calendar date using the curve's base date.
    ///
    /// Parameters
    /// ----------
    /// date : datetime.date
    ///     Calendar date to evaluate.
    ///
    /// Returns
    /// -------
    /// float
    ///     Discount factor at the supplied date.
    #[pyo3(text_signature = "(self, date)")]
    fn df_on_date(&self, _py: Python<'_>, date: Bound<'_, PyAny>) -> PyResult<f64> {
        let d = py_to_date(&date).context("date")?;
        self.inner.df_on_date_curve(d).map_err(core_to_py)
    }

    /// Continuously compounded zero rate on a calendar date.
    ///
    /// Uses the curve's day-count convention to compute the year fraction
    /// from base date to the target date, ensuring consistency.
    ///
    /// Parameters
    /// ----------
    /// date : datetime.date
    ///     Calendar date to evaluate.
    ///
    /// Returns
    /// -------
    /// float
    ///     Continuously compounded zero rate at the supplied date.
    #[pyo3(text_signature = "(self, date)")]
    fn zero_on_date(&self, date: Bound<'_, PyAny>) -> PyResult<f64> {
        let d = py_to_date(&date).context("date")?;
        self.inner
            .zero_rate_on_date(d, finstack_core::math::Compounding::Continuous)
            .map_err(core_to_py)
    }

    /// Annually compounded zero rate on a calendar date.
    ///
    /// Uses the curve's day-count convention to compute the year fraction
    /// from base date to the target date. This is the convention commonly
    /// used by Bloomberg for displaying zero rates.
    ///
    /// Parameters
    /// ----------
    /// date : datetime.date
    ///     Calendar date to evaluate.
    ///
    /// Returns
    /// -------
    /// float
    ///     Annually compounded zero rate at the supplied date.
    ///
    /// Examples
    /// --------
    /// >>> from datetime import date
    /// >>> curve.zero_annual_on_date(date(2026, 12, 14))
    /// 0.0342  # ~3.42% annual rate
    #[pyo3(text_signature = "(self, date)")]
    fn zero_annual_on_date(&self, date: Bound<'_, PyAny>) -> PyResult<f64> {
        let d = py_to_date(&date).context("date")?;
        self.inner
            .zero_rate_on_date(d, finstack_core::math::Compounding::Annual)
            .map_err(core_to_py)
    }

    /// Periodically compounded zero rate on a calendar date.
    ///
    /// Uses the curve's day-count convention to compute the year fraction.
    ///
    /// Parameters
    /// ----------
    /// date : datetime.date
    ///     Calendar date to evaluate.
    /// n : int
    ///     Number of compounding periods per year (1=annual, 2=semi-annual, etc.).
    ///
    /// Returns
    /// -------
    /// float
    ///     Periodically compounded zero rate at the supplied date.
    #[pyo3(text_signature = "(self, date, n)")]
    fn zero_periodic_on_date(&self, date: Bound<'_, PyAny>, n: u32) -> PyResult<f64> {
        use std::num::NonZeroU32;
        let d = py_to_date(&date).context("date")?;
        let n = NonZeroU32::new(n).ok_or_else(|| {
            PyValueError::new_err(
                "compounding periods per year (n) must be positive; use zero_on_date() for continuous compounding",
            )
        })?;
        self.inner
            .zero_rate_on_date(d, finstack_core::math::Compounding::Periodic(n))
            .map_err(core_to_py)
    }

    /// Forward rate between two calendar dates.
    ///
    /// Uses the curve's day-count convention to compute year fractions,
    /// ensuring consistency with curve construction.
    ///
    /// Parameters
    /// ----------
    /// d1 : datetime.date
    ///     Start date.
    /// d2 : datetime.date
    ///     End date (must be after d1).
    ///
    /// Returns
    /// -------
    /// float
    ///     Forward rate for the interval ``(d1, d2)``.
    ///
    /// Examples
    /// --------
    /// >>> from datetime import date
    /// >>> curve.forward_on_dates(date(2025, 12, 12), date(2026, 12, 14))
    /// 0.0365  # ~3.65% forward rate
    #[pyo3(text_signature = "(self, d1, d2)")]
    fn forward_on_dates(&self, d1: Bound<'_, PyAny>, d2: Bound<'_, PyAny>) -> PyResult<f64> {
        let date1 = py_to_date(&d1).context("d1")?;
        let date2 = py_to_date(&d2).context("d2")?;
        self.inner
            .forward_on_dates(date1, date2)
            .map_err(core_to_py)
    }

    /// Discount factors at each knot point.
    ///
    /// Returns
    /// -------
    /// list[float]
    ///     Discount factor values at each knot time.
    #[getter]
    fn discount_factors(&self) -> Vec<f64> {
        self.inner.dfs().to_vec()
    }

    /// Create a new curve with a parallel rate bump applied.
    ///
    /// Uses the formula: ``df_bumped(t) = df_original(t) * exp(-bump * t)``
    /// where ``bump = bp / 10_000``.
    ///
    /// Parameters
    /// ----------
    /// bp : float
    ///     Bump size in basis points (e.g., 1.0 = 1bp = 0.01%).
    ///
    /// Returns
    /// -------
    /// DiscountCurve
    ///     A new discount curve with bumped discount factors.
    ///
    /// Examples
    /// --------
    /// >>> bumped = curve.bumped_parallel(10.0)  # 10bp parallel bump
    /// >>> bumped.df(5.0) < curve.df(5.0)
    /// True
    #[pyo3(text_signature = "(self, bp)")]
    fn bumped_parallel(&self, bp: f64) -> PyResult<Self> {
        let bumped = self.inner.with_parallel_bump(bp).map_err(core_to_py)?;
        Ok(Self::new_arc(Arc::new(bumped)))
    }
}

/// Forward rate term structure parameterised by tenor length.
///
/// Parameters
/// ----------
/// id : str
///     Identifier used to fetch the curve.
/// tenor_years : float
///     Length of the forward period in years.
/// knots : list[tuple[float, float]]
///     `(time, forward_rate)` knot points.
/// base_date : datetime.date, optional
///     Base date for reset calculations.
/// reset_lag : int, optional
///     Business-day lag (in days) between accrual start and fixing.
/// day_count : DayCount, optional
///     Day-count convention for accrual.
/// interp : str, optional
///     Interpolation style label.
///
/// Returns
/// -------
/// ForwardCurve
///     Forward curve wrapper with ``rate`` evaluation helpers.
#[pyclass(
    module = "finstack.core.market_data.term_structures",
    name = "ForwardCurve",
    from_py_object
)]
#[derive(Clone)]
pub struct PyForwardCurve {
    pub(crate) inner: Arc<ForwardCurve>,
}

impl PyForwardCurve {
    pub(crate) fn new_arc(inner: Arc<ForwardCurve>) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyForwardCurve {
    /// Create a forward curve defined on `(time, rate)` knots for a given tenor.
    #[new]
    #[pyo3(signature = (id, tenor_years, knots, base_date=None, reset_lag=None, day_count=None, interp=None))]
    #[allow(clippy::too_many_arguments)]
    fn ctor(
        py: Python<'_>,
        id: &str,
        tenor_years: f64,
        knots: Bound<'_, PyAny>,
        base_date: Option<Bound<'_, PyAny>>,
        reset_lag: Option<i32>,
        day_count: Option<Bound<'_, PyAny>>,
        interp: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let knots_vec = extract_float_pairs(&knots)?;
        if knots_vec.len() < 2 {
            return Err(PyValueError::new_err(
                "knots must contain at least two (time, rate) pairs",
            ));
        }
        let mut builder = ForwardCurve::builder(id, tenor_years).knots(knots_vec);
        if let Some(date_obj) = base_date {
            let d = py_to_date(&date_obj).context("base_date")?;
            builder = builder.base_date(d);
        }
        if let Some(lag) = reset_lag {
            builder = builder.reset_lag(lag);
        }
        if let Some(dc) = parse_day_count(day_count)? {
            builder = builder.day_count(dc);
        }
        if let Some(obj) = interp {
            let style = parse_interp_style(Some(&obj), InterpStyle::Linear)?;
            builder = builder.interp(style);
        }
        let curve = py.detach(|| builder.build()).map_err(core_to_py)?;
        Ok(Self::new_arc(Arc::new(curve)))
    }

    /// Return the curve identifier.
    ///
    /// Returns
    /// -------
    /// str
    ///     Forward curve id.
    #[getter]
    fn id(&self) -> String {
        self.inner.id().to_string()
    }

    /// Tenor in years represented by the forward curve.
    ///
    /// Returns
    /// -------
    /// float
    ///     Forward tenor measured in years.
    #[getter]
    fn tenor_years(&self) -> f64 {
        self.inner.tenor()
    }

    /// Optional base date for forward accrual calculations.
    ///
    /// Returns
    /// -------
    /// datetime.date
    ///     Base date if configured, otherwise the curve's implicit base.
    #[getter]
    fn base_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.base_date())
    }

    /// Day-count convention attached to the curve.
    ///
    /// Returns
    /// -------
    /// DayCount
    ///     Day-count used for forward accruals.
    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count())
    }

    /// Reset lag in business days before the accrual period start.
    ///
    /// Returns
    /// -------
    /// int
    ///     Business-day lag used for fixing.
    #[getter]
    fn reset_lag(&self) -> i32 {
        self.inner.reset_lag()
    }

    /// Underlying ``(time, forward)`` knot points.
    ///
    /// Returns
    /// -------
    /// list[tuple[float, float]]
    ///     Forward rate knots.
    #[getter]
    fn points(&self) -> Vec<(f64, f64)> {
        self.inner
            .knots()
            .iter()
            .zip(self.inner.forwards().iter())
            .map(|(&t, &fwd)| (t, fwd))
            .collect()
    }

    /// Forward rate value ``f(t)`` at the given time in years.
    ///
    /// Parameters
    /// ----------
    /// t : float
    ///     Time in years from the base date.
    ///
    /// Returns
    /// -------
    /// float
    ///     Forward rate at ``t``.
    #[pyo3(text_signature = "(self, t)")]
    fn rate(&self, t: f64) -> f64 {
        self.inner.rate(t)
    }

    /// Implied projection discount factor ``df(t)`` from `0` to `t` (years).
    ///
    /// Notes
    /// -----
    /// This is a *projection DF* implied by chaining the forward curve's simple rates;
    /// it is not a PV discount curve. It is primarily intended for Bloomberg-style
    /// curve table comparisons.
    #[pyo3(text_signature = "(self, t)")]
    fn df(&self, t: f64) -> PyResult<f64> {
        self.inner.df(t).map_err(core_to_py)
    }

    /// Implied projection discount factor on a calendar date using the curve's day-count.
    #[pyo3(text_signature = "(self, date)")]
    fn df_on_date(&self, date: Bound<'_, PyAny>) -> PyResult<f64> {
        let d = py_to_date(&date).context("date")?;
        self.inner.df_on_date_curve(d).map_err(core_to_py)
    }
}
