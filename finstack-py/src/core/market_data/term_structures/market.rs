use crate::core::common::args::{extract_float_pairs, parse_extrap_style, parse_interp_style};
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::dates::PyDayCount;
use crate::errors::{core_to_py, PyContext};
use finstack_core::market_data::term_structures::ForwardVarianceCurve;
use finstack_core::market_data::term_structures::PriceCurve;
use finstack_core::math::interp::{ExtrapolationPolicy, InterpStyle};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::sync::Arc;

use super::parse_day_count;

/// Volatility index curve for forward volatility index levels.
///
/// Used for pricing volatility index futures and options (e.g., VIX futures).
///
/// Parameters
/// ----------
/// id : str
///     Identifier for the volatility index curve.
/// base_date : datetime.date
///     Anchor date for the curve (t = 0).
/// knots : list[tuple[float, float]]
///     `(time, forward_level)` pairs.
/// spot_level : float, optional
///     Current spot level of the volatility index.
/// day_count : DayCount, optional
///     Day-count convention for time calculations.
/// interp : str, optional
///     Interpolation style (defaults to ``"linear"``).
/// extrapolation : str, optional
///     Extrapolation policy (defaults to ``"flat_forward"``).
///
/// Returns
/// -------
/// VolatilityIndexCurve
///     Volatility index curve wrapper.
#[pyclass(
    module = "finstack.core.market_data.term_structures",
    name = "VolatilityIndexCurve",
    from_py_object
)]
#[derive(Clone)]
pub struct PyVolatilityIndexCurve {
    pub(crate) inner: Arc<finstack_core::market_data::term_structures::VolatilityIndexCurve>,
}

impl PyVolatilityIndexCurve {
    pub(crate) fn new_arc(
        inner: Arc<finstack_core::market_data::term_structures::VolatilityIndexCurve>,
    ) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyVolatilityIndexCurve {
    /// Create a volatility index curve from `(time, level)` knot points.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Curve identifier.
    /// base_date : datetime.date
    ///     Date corresponding to t = 0.
    /// knots : list[tuple[float, float]]
    ///     ``(time, forward_level)`` pairs in ascending time order.
    /// spot_level : float, optional
    ///     Current spot level. Defaults to first knot level.
    /// day_count : DayCount, optional
    ///     Override the default Act/365F convention.
    /// interp : str, optional
    ///     Interpolation style (``"linear"``, ``"monotone_convex"``, etc.).
    /// extrapolation : str, optional
    ///     Extrapolation policy name.
    ///
    /// Returns
    /// -------
    /// VolatilityIndexCurve
    ///     Volatility index curve.
    #[new]
    #[pyo3(signature = (id, base_date, knots, spot_level=None, day_count=None, interp=None, extrapolation=None))]
    #[allow(clippy::too_many_arguments)]
    fn ctor(
        py: Python<'_>,
        id: &str,
        base_date: Bound<'_, PyAny>,
        knots: Bound<'_, PyAny>,
        spot_level: Option<f64>,
        day_count: Option<Bound<'_, PyAny>>,
        interp: Option<Bound<'_, PyAny>>,
        extrapolation: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let knots_vec = extract_float_pairs(&knots)?;
        if knots_vec.len() < 2 {
            return Err(PyValueError::new_err(
                "knots must contain at least two (time, level) pairs",
            ));
        }
        let base = py_to_date(&base_date).context("base_date")?;
        let style = parse_interp_style(interp.as_ref(), InterpStyle::Linear)?;
        let extra = parse_extrap_style(extrapolation.as_ref(), ExtrapolationPolicy::FlatForward)?;
        let mut builder =
            finstack_core::market_data::term_structures::VolatilityIndexCurve::builder(id)
                .base_date(base)
                .knots(knots_vec)
                .interp(style)
                .extrapolation(extra);
        if let Some(dc) = parse_day_count(day_count)? {
            builder = builder.day_count(dc);
        }
        if let Some(spot) = spot_level {
            builder = builder.spot_level(spot);
        }
        let curve = py.detach(|| builder.build()).map_err(core_to_py)?;
        Ok(Self::new_arc(Arc::new(curve)))
    }

    /// Return the curve identifier.
    #[getter]
    fn id(&self) -> String {
        self.inner.id().to_string()
    }

    /// Base date used for forward level calculations.
    #[getter]
    fn base_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.base_date())
    }

    /// Day-count convention used for time calculations.
    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count())
    }

    /// Current spot level of the volatility index.
    #[getter]
    fn spot_level(&self) -> f64 {
        self.inner.spot_level()
    }

    /// Knot points as ``(time, level)`` pairs.
    #[getter]
    fn points(&self) -> Vec<(f64, f64)> {
        self.inner
            .knots()
            .iter()
            .zip(self.inner.levels().iter())
            .map(|(&t, &lvl)| (t, lvl))
            .collect()
    }

    /// Forward volatility index level at time ``t`` in years.
    ///
    /// Parameters
    /// ----------
    /// t : float
    ///     Time in years from the base date.
    ///
    /// Returns
    /// -------
    /// float
    ///     Forward volatility index level at ``t``.
    #[pyo3(text_signature = "(self, t)")]
    fn forward_level(&self, t: f64) -> f64 {
        self.inner.forward_level(t)
    }
}

/// Forward price curve for commodities and other price-based assets.
///
/// Used for pricing commodity derivatives, forwards, and options.
///
/// Parameters
/// ----------
/// id : str
///     Identifier for the price curve.
/// base_date : datetime.date
///     Anchor date for the curve (t = 0).
/// knots : list[tuple[float, float]]
///     `(time, forward_price)` pairs.
/// spot_price : float, optional
///     Current spot price. Defaults to first knot price.
/// day_count : DayCount, optional
///     Day-count convention for time calculations.
/// interp : str, optional
///     Interpolation style (defaults to ``"linear"``).
/// extrapolation : str, optional
///     Extrapolation policy (defaults to ``"flat_zero"``).
///
/// Returns
/// -------
/// PriceCurve
///     Forward price curve wrapper.
#[pyclass(
    module = "finstack.core.market_data.term_structures",
    name = "PriceCurve",
    from_py_object
)]
#[derive(Clone)]
pub struct PyPriceCurve {
    pub(crate) inner: Arc<PriceCurve>,
}

impl PyPriceCurve {
    pub(crate) fn new_arc(inner: Arc<PriceCurve>) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPriceCurve {
    /// Create a forward price curve from `(time, price)` knot points.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Curve identifier.
    /// base_date : datetime.date
    ///     Date corresponding to t = 0.
    /// knots : list[tuple[float, float]]
    ///     ``(time, forward_price)`` pairs in ascending time order.
    /// spot_price : float, optional
    ///     Current spot price. Defaults to first knot price.
    /// day_count : DayCount, optional
    ///     Override the default Act/365F convention.
    /// interp : str, optional
    ///     Interpolation style (``"linear"``, ``"monotone_convex"``, etc.).
    /// extrapolation : str, optional
    ///     Extrapolation policy name.
    ///
    /// Returns
    /// -------
    /// PriceCurve
    ///     Forward price curve.
    #[new]
    #[pyo3(signature = (id, base_date, knots, spot_price=None, day_count=None, interp=None, extrapolation=None))]
    #[allow(clippy::too_many_arguments)]
    fn ctor(
        py: Python<'_>,
        id: &str,
        base_date: Bound<'_, PyAny>,
        knots: Bound<'_, PyAny>,
        spot_price: Option<f64>,
        day_count: Option<Bound<'_, PyAny>>,
        interp: Option<Bound<'_, PyAny>>,
        extrapolation: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let knots_vec = extract_float_pairs(&knots)?;
        if knots_vec.len() < 2 {
            return Err(PyValueError::new_err(
                "knots must contain at least two (time, price) pairs",
            ));
        }
        let base = py_to_date(&base_date).context("base_date")?;
        let style = parse_interp_style(interp.as_ref(), InterpStyle::Linear)?;
        let extra = parse_extrap_style(extrapolation.as_ref(), ExtrapolationPolicy::FlatZero)?;
        let mut builder = PriceCurve::builder(id)
            .base_date(base)
            .knots(knots_vec)
            .interp(style)
            .extrapolation(extra);
        if let Some(dc) = parse_day_count(day_count)? {
            builder = builder.day_count(dc);
        }
        if let Some(spot) = spot_price {
            builder = builder.spot_price(spot);
        }
        let curve = py.detach(|| builder.build()).map_err(core_to_py)?;
        Ok(Self::new_arc(Arc::new(curve)))
    }

    /// Return the curve identifier.
    #[getter]
    fn id(&self) -> String {
        self.inner.id().to_string()
    }

    /// Base date used for forward price calculations.
    #[getter]
    fn base_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.base_date())
    }

    /// Day-count convention used for time calculations.
    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count())
    }

    /// Current spot price.
    #[getter]
    fn spot_price(&self) -> f64 {
        self.inner.spot_price()
    }

    /// Knot points as ``(time, price)`` pairs.
    #[getter]
    fn points(&self) -> Vec<(f64, f64)> {
        self.inner
            .knots()
            .iter()
            .zip(self.inner.prices().iter())
            .map(|(&t, &p)| (t, p))
            .collect()
    }

    /// Forward price at time ``t`` in years.
    ///
    /// Parameters
    /// ----------
    /// t : float
    ///     Time in years from the base date.
    ///
    /// Returns
    /// -------
    /// float
    ///     Forward price at ``t``.
    #[pyo3(text_signature = "(self, t)")]
    fn price(&self, t: f64) -> f64 {
        self.inner.price(t)
    }

    /// Forward price on a specific calendar date.
    ///
    /// Parameters
    /// ----------
    /// date : datetime.date
    ///     Calendar date to evaluate.
    ///
    /// Returns
    /// -------
    /// float
    ///     Forward price at the supplied date.
    #[pyo3(text_signature = "(self, date)")]
    fn price_on_date(&self, date: Bound<'_, PyAny>) -> PyResult<f64> {
        let d = py_to_date(&date).context("date")?;
        self.inner.price_on_date(d).map_err(core_to_py)
    }

    /// Create a new curve with a parallel bump applied (additive, in price units).
    ///
    /// Parameters
    /// ----------
    /// bump : float
    ///     Bump size in price units (e.g., 1.0 adds $1 to all prices).
    ///
    /// Returns
    /// -------
    /// PriceCurve
    ///     A new price curve with bumped prices.
    #[pyo3(text_signature = "(self, bump)")]
    fn bumped_parallel(&self, bump: f64) -> PyResult<Self> {
        let bumped = self.inner.with_parallel_bump(bump).map_err(core_to_py)?;
        Ok(Self::new_arc(Arc::new(bumped)))
    }

    /// Create a new curve with a percentage bump applied (multiplicative).
    ///
    /// Parameters
    /// ----------
    /// pct : float
    ///     Percentage bump (e.g., 0.01 = +1%, -0.05 = -5%).
    ///
    /// Returns
    /// -------
    /// PriceCurve
    ///     A new price curve with scaled prices.
    #[pyo3(text_signature = "(self, pct)")]
    fn bumped_percentage(&self, pct: f64) -> PyResult<Self> {
        let bumped = self.inner.with_percentage_bump(pct).map_err(core_to_py)?;
        Ok(Self::new_arc(Arc::new(bumped)))
    }

    /// Roll the curve forward by a specified number of days.
    ///
    /// Parameters
    /// ----------
    /// days : int
    ///     Number of days to roll forward.
    ///
    /// Returns
    /// -------
    /// PriceCurve
    ///     A new price curve with updated base date and shifted knots.
    #[pyo3(text_signature = "(self, days)")]
    fn roll_forward(&self, days: i64) -> PyResult<Self> {
        let rolled = self.inner.roll_forward(days).map_err(core_to_py)?;
        Ok(Self::new_arc(Arc::new(rolled)))
    }
}

// ---------------------------------------------------------------------------
// ForwardVarianceCurve
// ---------------------------------------------------------------------------

/// Forward variance curve for rough volatility models.
///
/// Represents the market-implied forward variance strip used as input to
/// rBergomi and related rough volatility models.
#[pyclass(
    module = "finstack.core.market_data.term_structures",
    name = "ForwardVarianceCurve",
    from_py_object
)]
#[derive(Clone)]
pub struct PyForwardVarianceCurve {
    pub(crate) inner: ForwardVarianceCurve,
}

#[pymethods]
impl PyForwardVarianceCurve {
    /// Create a flat forward variance curve (constant variance).
    ///
    /// Parameters
    /// ----------
    /// v0 : float
    ///     Constant forward variance (must be positive).
    ///
    /// Returns
    /// -------
    /// ForwardVarianceCurve
    ///     Flat forward variance curve.
    #[staticmethod]
    fn flat(v0: f64) -> PyResult<Self> {
        let curve = ForwardVarianceCurve::flat(v0).map_err(core_to_py)?;
        Ok(Self { inner: curve })
    }

    /// Create a forward variance curve from ``(time, forward_variance)`` pairs.
    ///
    /// Points are sorted by time internally before validation.
    ///
    /// Parameters
    /// ----------
    /// points : list[tuple[float, float]]
    ///     ``(time, forward_variance)`` pairs. Times must be non-negative and
    ///     strictly increasing; variances must be positive.
    ///
    /// Returns
    /// -------
    /// ForwardVarianceCurve
    ///     Forward variance curve with piecewise linear interpolation.
    #[staticmethod]
    fn from_points(points: Vec<(f64, f64)>) -> PyResult<Self> {
        let curve = ForwardVarianceCurve::from_points(&points).map_err(core_to_py)?;
        Ok(Self { inner: curve })
    }

    /// Evaluate the forward variance at time ``t``.
    ///
    /// Parameters
    /// ----------
    /// t : float
    ///     Time in years.
    ///
    /// Returns
    /// -------
    /// float
    ///     Forward variance at ``t``.
    #[pyo3(text_signature = "(self, t)")]
    fn value(&self, t: f64) -> f64 {
        self.inner.value(t)
    }

    /// Compute the integrated variance from 0 to ``t``.
    ///
    /// Parameters
    /// ----------
    /// t : float
    ///     Time in years.
    ///
    /// Returns
    /// -------
    /// float
    ///     Integrated variance over ``[0, t]``.
    #[pyo3(text_signature = "(self, t)")]
    fn integrated_variance(&self, t: f64) -> f64 {
        self.inner.integrated_variance(t)
    }

    fn __repr__(&self) -> String {
        "ForwardVarianceCurve(...)".to_string()
    }
}
