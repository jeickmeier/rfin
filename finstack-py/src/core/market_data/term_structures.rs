use crate::core::common::args::{
    extract_float_pairs, DayCountArg, ExtrapolationPolicyArg, InterpStyleArg,
};
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::dates::PyDayCount;
use crate::core::math::interp::{
    parse_extrapolation, parse_interp, PyExtrapolationPolicy, PyInterpStyle,
};
use crate::core::money::{extract_money, PyMoney};
use crate::errors::{core_to_py, PyContext};
use finstack_core::cashflow::discounting::npv_static;
use finstack_core::market_data::term_structures::base_correlation::BaseCorrelationCurve;
use finstack_core::market_data::term_structures::credit_index::CreditIndexData;
use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
use finstack_core::market_data::term_structures::forward_curve::ForwardCurve;
use finstack_core::market_data::term_structures::hazard_curve::{HazardCurve, Seniority};
use finstack_core::market_data::term_structures::inflation::InflationCurve;
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::{Bound, PyRef};
use std::collections::HashMap;
use std::sync::Arc;

fn parse_day_count(
    dc: Option<Bound<'_, PyAny>>,
) -> PyResult<Option<finstack_core::dates::DayCount>> {
    match dc {
        None => Ok(None),
        Some(value) => {
            if let Ok(dc) = value.extract::<PyRef<PyDayCount>>() {
                return Ok(Some(dc.inner));
            }
            if let Ok(DayCountArg(inner)) = value.extract::<DayCountArg>() {
                return Ok(Some(inner));
            }
            Err(pyo3::exceptions::PyTypeError::new_err(
                "day_count must be DayCount or string",
            ))
        }
    }
}

fn parse_interp_enum(value: Option<&str>, default: InterpStyle) -> PyResult<InterpStyle> {
    parse_interp(value, default)
}

fn parse_extrap_enum(value: Option<&str>) -> PyResult<ExtrapolationPolicy> {
    parse_extrapolation(value)
}

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
///     Interpolation style such as ``"linear"`` or ``"monotone_convex"``.
/// extrapolation : str, optional
///     Extrapolation policy name (e.g. ``"flat_zero"``).
/// require_monotonic : bool, default True
///     Enforce monotonic discount factors across knots (set False to allow non-monotonic DFs).
///
/// Returns
/// -------
/// DiscountCurve
///     Curve object exposing discount factor, zero rate, and forward helpers.
#[pyclass(
    module = "finstack.core.market_data.term_structures",
    name = "DiscountCurve"
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
    ///     Interpolation style (``"linear"``, ``"monotone_convex"``, etc.).
    /// extrapolation : str, optional
    ///     Extrapolation policy name (``"flat_zero"``, ``"flat_forward"`` ...).
    /// require_monotonic : bool, default True
    ///     Enforce monotonic discount factors across knots (set False to allow non-monotonic DFs).
    ///
    /// Returns
    /// -------
    /// DiscountCurve
    ///     Discount curve with pre-computed interpolation.
    #[new]
    #[pyo3(signature = (id, base_date, knots, day_count=None, interp=None, extrapolation=None, require_monotonic=true))]
    fn ctor(
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
        let style = match interp {
            None => InterpStyle::Linear,
            Some(obj) => {
                if let Ok(InterpStyleArg(v)) = obj.extract::<InterpStyleArg>() {
                    v
                } else if let Ok(py_style) = obj.extract::<PyRef<PyInterpStyle>>() {
                    py_style.inner
                } else if let Ok(name) = obj.extract::<&str>() {
                    parse_interp_enum(Some(name), InterpStyle::Linear)?
                } else {
                    return Err(pyo3::exceptions::PyTypeError::new_err(
                        "interp must be InterpStyle or string",
                    ));
                }
            }
        };
        let extra = match extrapolation {
            None => parse_extrap_enum(None)?,
            Some(obj) => {
                if let Ok(ExtrapolationPolicyArg(v)) = obj.extract::<ExtrapolationPolicyArg>() {
                    v
                } else if let Ok(py_ex) = obj.extract::<PyRef<PyExtrapolationPolicy>>() {
                    py_ex.inner
                } else if let Ok(name) = obj.extract::<&str>() {
                    parse_extrap_enum(Some(name))?
                } else {
                    return Err(pyo3::exceptions::PyTypeError::new_err(
                        "extrapolation must be ExtrapolationPolicy or string",
                    ));
                }
            }
        };
        let mut builder = DiscountCurve::builder(id)
            .base_date(base)
            .knots(knots_vec)
            .set_interp(style)
            .extrapolation(extra);
        if let Some(dc) = parse_day_count(day_count)? {
            builder = builder.day_count(dc);
        }
        if !require_monotonic {
            builder = builder.allow_non_monotonic();
        }
        let curve = Python::attach(|py| py.detach(|| builder.build().map_err(core_to_py)))?;
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
    fn forward(&self, t1: f64, t2: f64) -> f64 {
        self.inner.forward(t1, t2)
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
        Ok(self.inner.df_on_date_curve(d))
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
        Ok(self.inner.zero_on_date(d))
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
        Ok(self.inner.zero_annual_on_date(d))
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
        let d = py_to_date(&date).context("date")?;
        Ok(self.inner.zero_periodic_on_date(d, n))
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
        Ok(self.inner.forward_on_dates(date1, date2))
    }

    #[pyo3(text_signature = "(self, cash_flows, day_count=None)")]
    /// Calculate the Net Present Value of a series of cashflows.
    ///
    /// Parameters
    /// ----------
    /// cash_flows : list[tuple[date, Money]]
    ///     List of dated cashflows to discount.
    /// day_count : DayCount, optional
    ///     Day count convention for discounting (defaults to curve's day count).
    ///
    /// Returns
    /// -------
    /// Money
    ///     The NPV in the currency of the cashflows.
    fn npv(
        &self,
        cash_flows: Bound<'_, PyAny>,
        day_count: Option<Bound<'_, PyAny>>,
    ) -> PyResult<PyMoney> {
        let flows_iter = cash_flows.try_iter()?;
        let mut flows = Vec::new();
        for item in flows_iter {
            let item = item?;
            // Expect tuple (date, money_like)
            if let Ok(tuple) = item.downcast::<pyo3::types::PyTuple>() {
                if tuple.len() == 2 {
                    let date = py_to_date(&tuple.get_item(0).context("date")?)
                        .context("cash_flow_date")?;
                    let money = extract_money(&tuple.get_item(1).context("money")?)
                        .context("cash_flow_amount")?;
                    flows.push((date, money));
                } else {
                    return Err(PyValueError::new_err(
                        "cash_flows must be list of (date, money) tuples",
                    ));
                }
            } else {
                return Err(PyTypeError::new_err(
                    "cash_flows must be list of (date, money) tuples",
                ));
            }
        }

        let dc = parse_day_count(day_count)?.unwrap_or(self.inner.day_count());

        let result =
            npv_static(&*self.inner, self.inner.base_date(), dc, &flows).map_err(core_to_py)?;
        Ok(PyMoney::new(result))
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
    name = "ForwardCurve"
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
    fn ctor(
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
            let style = if let Ok(InterpStyleArg(v)) = obj.extract::<InterpStyleArg>() {
                v
            } else if let Ok(py_style) = obj.extract::<PyRef<PyInterpStyle>>() {
                py_style.inner
            } else if let Ok(name) = obj.extract::<&str>() {
                parse_interp_enum(Some(name), InterpStyle::Linear)?
            } else {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "interp must be InterpStyle or string",
                ));
            };
            builder = builder.set_interp(style);
        }
        let curve = Python::attach(|py| py.detach(|| builder.build().map_err(core_to_py)))?;
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
    name = "HazardCurve"
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

fn parse_seniority(value: Option<&str>) -> PyResult<Option<Seniority>> {
    match value {
        None => Ok(None),
        Some(name) => match name.to_ascii_lowercase().as_str() {
            "senior_secured" | "senior-secured" => Ok(Some(Seniority::SeniorSecured)),
            "senior" => Ok(Some(Seniority::Senior)),
            "subordinated" => Ok(Some(Seniority::Subordinated)),
            "junior" => Ok(Some(Seniority::Junior)),
            other => Err(PyValueError::new_err(format!("Unknown seniority: {other}"))),
        },
    }
}

#[pymethods]
impl PyHazardCurve {
    /// Construct a hazard (default intensity) curve from `(time, hazard)` knots.
    #[new]
    #[pyo3(signature = (id, base_date, knots, recovery_rate=None, day_count=None, issuer=None, seniority=None, currency=None, par_points=None))]
    fn ctor(
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
        let curve = Python::attach(|py| py.detach(|| builder.build().map_err(core_to_py)))?;
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
    fn default_prob(&self, t1: f64, t2: f64) -> f64 {
        self.inner.default_prob(t1, t2)
    }
}

/// Inflation curve used for CPI projections.
///
/// Parameters
/// ----------
/// id : str
///     Identifier for the inflation curve.
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
    name = "InflationCurve"
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
    #[pyo3(signature = (id, base_cpi, knots, interp=None))]
    fn ctor(
        id: &str,
        base_cpi: f64,
        knots: Bound<'_, PyAny>,
        interp: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let knots_vec = extract_float_pairs(&knots)?;
        if knots_vec.is_empty() {
            return Err(PyValueError::new_err("knots must not be empty"));
        }
        let style = match interp {
            None => InterpStyle::LogLinear,
            Some(obj) => {
                if let Ok(InterpStyleArg(v)) = obj.extract::<InterpStyleArg>() {
                    v
                } else if let Ok(py_style) = obj.extract::<PyRef<PyInterpStyle>>() {
                    py_style.inner
                } else if let Ok(name) = obj.extract::<&str>() {
                    parse_interp_enum(Some(name), InterpStyle::LogLinear)?
                } else {
                    return Err(pyo3::exceptions::PyTypeError::new_err(
                        "interp must be InterpStyle or string",
                    ));
                }
            }
        };
        let builder = InflationCurve::builder(id)
            .base_cpi(base_cpi)
            .knots(knots_vec)
            .set_interp(style);
        let curve = Python::attach(|py| py.detach(|| builder.build().map_err(core_to_py)))?;
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
    name = "BaseCorrelationCurve"
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
    fn ctor(id: &str, points: Bound<'_, PyAny>) -> PyResult<Self> {
        let points_vec = extract_float_pairs(&points)?;
        if points_vec.len() < 2 {
            return Err(PyValueError::new_err(
                "points must contain at least two entries",
            ));
        }
        let curve = Python::attach(|py| {
            py.detach(|| {
                BaseCorrelationCurve::builder(id)
                    .points(points_vec)
                    .build()
                    .map_err(core_to_py)
            })
        })?;
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
    name = "CreditIndexData"
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
            builder = builder.with_issuer_curves(mapped);
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

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "term_structures")?;
    module.setattr(
        "__doc__",
        "One-dimensional market curves: discount, forward, hazard, inflation, base correlation, and credit index aggregates.",
    )?;
    module.add_class::<PyDiscountCurve>()?;
    module.add_class::<PyForwardCurve>()?;
    module.add_class::<PyHazardCurve>()?;
    module.add_class::<PyInflationCurve>()?;
    module.add_class::<PyBaseCorrelationCurve>()?;
    module.add_class::<PyCreditIndexData>()?;

    let exports = [
        "DiscountCurve",
        "ForwardCurve",
        "HazardCurve",
        "InflationCurve",
        "BaseCorrelationCurve",
        "CreditIndexData",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
