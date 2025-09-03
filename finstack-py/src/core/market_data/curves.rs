//! Python bindings for term structure curves.

use numpy::{IntoPyArray, PyArray1};
use pyo3::prelude::*;
use pyo3::types::PyList;
use std::sync::Arc;

use finstack_core::{
    market_data::{
        term_structures::{
            DiscountCurve as CoreDiscountCurve, ForwardCurve as CoreForwardCurve,
            HazardCurve as CoreHazardCurve, InflationCurve as CoreInflationCurve,
        },
        traits::{Discount, TermStructure},
    },
    F,
};

use super::interpolation::{PyInterpStyle, PyExtrapolationPolicy};
use crate::core::dates::PyDate;
use crate::core::dates::PyDayCount;

/// Discount factor curve for present value calculations.
///
/// A discount curve represents the time value of money, providing discount
/// factors that convert future cash flows to present values. The curve is
/// constructed from market data and interpolates between known points.
///
/// The curve supports various interpolation methods to ensure smooth and
/// arbitrage-free discount factors between market quotes.
///
/// Examples:
///     >>> from rfin.market_data import DiscountCurve, InterpStyle
///     >>> from rfin import Date
///     >>> import numpy as np
///     
///     # Create a discount curve
///     >>> curve = DiscountCurve(
///     ...     id="USD-OIS",
///     ...     base_date=Date(2025, 1, 1),
///     ...     times=[0.0, 0.5, 1.0, 2.0, 5.0],
///     ...     discount_factors=[1.0, 0.99, 0.98, 0.95, 0.88],
///     ...     interpolation=InterpStyle.MonotoneConvex
///     ... )
///     
///     # Query discount factors
///     >>> curve.df(1.5)  # Interpolated value
///     0.965
///     
///     # Get zero rates
///     >>> curve.zero(2.0)
///     0.025328...
///     
///     # Forward rates
///     >>> curve.forward(1.0, 2.0)
///     0.030459...
///     
///     # Vectorized operations
///     >>> times = np.linspace(0, 5, 50)
///     >>> dfs = curve.df_batch(times)
#[pyclass(name = "DiscountCurve", module = "finstack.market_data")]
#[derive(Clone)]
pub struct PyDiscountCurve {
    inner: Arc<CoreDiscountCurve>,
}

#[pymethods]
impl PyDiscountCurve {
    /// Create a new DiscountCurve.
    ///
    /// Args:
    ///     id (str): Unique identifier for the curve (e.g., "USD-OIS")
    ///     base_date (Date): The valuation date of the curve
    ///     times (List[float] | numpy.ndarray): Time points in years from base_date
    ///     discount_factors (List[float] | numpy.ndarray): Discount factors at each time point
    ///     interpolation (InterpStyle): Interpolation method (default: Linear)
    ///     extrapolation (ExtrapolationPolicy): Extrapolation policy (default: FlatZero)
    ///     require_monotonic (bool): Require strictly decreasing DFs for credit curves (default: False)
    ///
    /// Returns:
    ///     DiscountCurve: A new discount curve instance
    ///
    /// Raises:
    ///     ValueError: If inputs are invalid (e.g., non-monotonic times, non-positive DFs)
    #[new]
    #[pyo3(signature = (id, base_date, times, discount_factors, interpolation=PyInterpStyle::Linear, extrapolation=PyExtrapolationPolicy::FlatZero, require_monotonic=false))]
    fn new(
        id: String,
        base_date: &PyDate,
        times: &Bound<'_, PyAny>,
        discount_factors: &Bound<'_, PyAny>,
        interpolation: PyInterpStyle,
        extrapolation: PyExtrapolationPolicy,
        require_monotonic: bool,
    ) -> PyResult<Self> {
        // Convert times and discount_factors to Vec<F>
        let times_vec = extract_f64_array(times)?;
        let dfs_vec = extract_f64_array(discount_factors)?;

        if times_vec.len() != dfs_vec.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "times and discount_factors must have the same length",
            ));
        }

        // Build the curve - leak the string to get 'static lifetime
        let id_static = Box::leak(id.into_boxed_str());
        let mut builder = CoreDiscountCurve::builder(id_static).base_date(base_date.inner());

        // Add knots
        for (t, df) in times_vec.iter().zip(dfs_vec.iter()) {
            builder = builder.knots([(*t, *df)]);
        }

        // Set interpolation
        builder = match interpolation {
            PyInterpStyle::Linear => builder.linear_df(),
            PyInterpStyle::LogLinear => builder.log_df(),
            PyInterpStyle::MonotoneConvex => builder.monotone_convex(),
            PyInterpStyle::CubicHermite => builder.cubic_hermite(),
            PyInterpStyle::FlatForward => builder.flat_fwd(),
        };

        // Set extrapolation policy
        builder = builder.extrapolation(extrapolation.to_core());

        // Set monotonic validation if required
        if require_monotonic {
            builder = builder.require_monotonic();
        }

        let curve = builder.build().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to build curve: {:?}",
                e
            ))
        })?;

        Ok(PyDiscountCurve {
            inner: Arc::new(curve),
        })
    }

    /// Create a discount curve from zero rates.
    ///
    /// Args:
    ///     id (str): Unique identifier for the curve
    ///     base_date (Date): The valuation date of the curve
    ///     times (List[float] | numpy.ndarray): Time points in years
    ///     zero_rates (List[float] | numpy.ndarray): Continuously compounded zero rates
    ///     interpolation (InterpStyle): Interpolation method (default: Linear)
    ///     extrapolation (ExtrapolationPolicy): Extrapolation policy (default: FlatZero)
    ///     require_monotonic (bool): Require strictly decreasing DFs for credit curves (default: False)
    ///
    /// Returns:
    ///     DiscountCurve: A new discount curve instance
    #[staticmethod]
    #[pyo3(signature = (id, base_date, times, zero_rates, interpolation=PyInterpStyle::Linear, extrapolation=PyExtrapolationPolicy::FlatZero, require_monotonic=false))]
    fn from_zero_rates(
        id: String,
        base_date: &PyDate,
        times: &Bound<'_, PyAny>,
        zero_rates: &Bound<'_, PyAny>,
        interpolation: PyInterpStyle,
        extrapolation: PyExtrapolationPolicy,
        require_monotonic: bool,
    ) -> PyResult<Self> {
        let times_vec = extract_f64_array(times)?;
        let zeros_vec = extract_f64_array(zero_rates)?;

        if times_vec.len() != zeros_vec.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "times and zero_rates must have the same length",
            ));
        }

        // Convert zero rates to discount factors
        let dfs_vec: Vec<F> = times_vec
            .iter()
            .zip(zeros_vec.iter())
            .map(|(t, z)| (-z * t).exp())
            .collect();

        // Use the regular constructor
        let py = times.py();
        let dfs_list = PyList::new(py, dfs_vec)?;
        PyDiscountCurve::new(id, base_date, times, dfs_list.as_any(), interpolation, extrapolation, require_monotonic)
    }

    /// Unique identifier of the curve.
    #[getter]
    fn id(&self) -> String {
        TermStructure::id(&*self.inner).as_str().to_string()
    }

    /// Base (valuation) date of the curve.
    #[getter]
    fn base_date(&self) -> PyDate {
        PyDate::from_core(self.inner.base_date())
    }

    /// Time points used for interpolation (year fractions).
    #[getter]
    fn times<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        self.inner.knots().to_vec().into_pyarray(py)
    }

    /// Discount factors at each time point.
    #[getter]
    fn discount_factors<'py>(&self, py: Python<'py>) -> Bound<'py, PyArray1<f64>> {
        self.inner.dfs().to_vec().into_pyarray(py)
    }

    /// Discount factor at time t.
    ///
    /// Args:
    ///     t (float): Time in years from the base date
    ///
    /// Returns:
    ///     float: The discount factor at time t
    fn df(&self, t: F) -> F {
        self.inner.df(t)
    }

    /// Continuously compounded zero rate at time t.
    ///
    /// Args:
    ///     t (float): Time in years from the base date
    ///
    /// Returns:
    ///     float: The zero rate at time t
    fn zero(&self, t: F) -> F {
        Discount::zero(&*self.inner, t)
    }

    /// Forward rate between t1 and t2.
    ///
    /// Args:
    ///     t1 (float): Start time in years
    ///     t2 (float): End time in years
    ///
    /// Returns:
    ///     float: The forward rate between t1 and t2
    ///
    /// Raises:
    ///     ValueError: If t2 <= t1
    fn forward(&self, t1: F, t2: F) -> PyResult<F> {
        if t2 <= t1 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "t2 must be greater than t1",
            ));
        }
        Ok(Discount::fwd(&*self.inner, t1, t2))
    }

    /// Get discount factors for multiple time points.
    ///
    /// Args:
    ///     times: Array of time points (year fractions)
    ///
    /// Returns:
    ///     np.ndarray: Array of discount factors
    ///
    /// Examples:
    ///     >>> times = np.array([0.5, 1.0, 2.0])
    ///     >>> dfs = curve.df_batch(times)
    fn df_batch<'py>(
        &self,
        py: Python<'py>,
        times: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyArray1<f64>>> {
        let times_vec = extract_f64_array(times)?;
        let dfs: Vec<f64> = times_vec.iter().map(|&t| self.inner.df(t)).collect();

        Ok(dfs.into_pyarray(py))
    }

    fn __repr__(&self) -> String {
        format!(
            "DiscountCurve(id='{}')",
            TermStructure::id(&*self.inner).as_str()
        )
    }
}

/// Forward rate curve for a fixed-tenor index.
///
/// Represents forward rates for a specific tenor (e.g., 3-month SOFR).
/// The curve interpolates between market quotes to provide forward rates
/// at any future time point.
///
/// Examples:
///     >>> from rfin.market_data import ForwardCurve, InterpStyle
///     >>> from rfin import Date, DayCount
///     
///     # Create a 3-month forward curve
///     >>> curve = ForwardCurve(
///     ...     id="USD-SOFR3M",
///     ...     tenor=0.25,  # 3 months in years
///     ...     base_date=Date(2025, 1, 1),
///     ...     times=[0.0, 1.0, 2.0, 5.0],
///     ...     forward_rates=[0.03, 0.035, 0.04, 0.045],
///     ...     day_count=DayCount.Act360
///     ... )
///     
///     # Query forward rates
///     >>> curve.rate(1.5)
///     0.0375
///     
///     # Average rate over a period
///     >>> curve.rate_period(1.0, 2.0)
///     0.0375
#[pyclass(name = "ForwardCurve", module = "finstack.market_data")]
#[derive(Clone)]
pub struct PyForwardCurve {
    inner: Arc<CoreForwardCurve>,
}

#[pymethods]
impl PyForwardCurve {
    /// Create a new ForwardCurve.
    ///
    /// Args:
    ///     id (str): Unique identifier for the curve
    ///     tenor (float): Index tenor in years (e.g., 0.25 for 3-month)
    ///     base_date (Date): The valuation date
    ///     times (List[float] | numpy.ndarray): Time points in years
    ///     forward_rates (List[float] | numpy.ndarray): Forward rates at each time
    ///     interpolation (InterpStyle): Interpolation method (default: Linear)
    ///     reset_lag (int): Days from fixing to spot (default: 2)
    ///     day_count (DayCount): Day count convention (default: Act360)
    #[new]
    #[pyo3(signature = (id, tenor, base_date, times, forward_rates, interpolation=PyInterpStyle::Linear, reset_lag=2, day_count=None))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        id: String,
        tenor: F,
        base_date: &PyDate,
        times: &Bound<'_, PyAny>,
        forward_rates: &Bound<'_, PyAny>,
        interpolation: PyInterpStyle,
        reset_lag: i32,
        day_count: Option<&PyDayCount>,
    ) -> PyResult<Self> {
        let times_vec = extract_f64_array(times)?;
        let rates_vec = extract_f64_array(forward_rates)?;

        if times_vec.len() != rates_vec.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "times and forward_rates must have the same length",
            ));
        }

        let id_static = Box::leak(id.into_boxed_str());
        let mut builder = CoreForwardCurve::builder(id_static, tenor)
            .base_date(base_date.inner())
            .reset_lag(reset_lag);

        if let Some(dc) = day_count {
            builder = builder.day_count(dc.inner());
        }

        // Add knots
        for (t, rate) in times_vec.iter().zip(rates_vec.iter()) {
            builder = builder.knots([(*t, *rate)]);
        }

        // Set interpolation
        builder = match interpolation {
            PyInterpStyle::Linear => builder.linear_df(),
            PyInterpStyle::LogLinear => builder.log_df(),
            PyInterpStyle::MonotoneConvex => builder.monotone_convex(),
            PyInterpStyle::CubicHermite => builder.cubic_hermite(),
            PyInterpStyle::FlatForward => builder.flat_fwd(),
        };

        let curve = builder.build().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to build curve: {:?}",
                e
            ))
        })?;

        Ok(PyForwardCurve {
            inner: Arc::new(curve),
        })
    }

    /// Unique identifier of the curve.
    #[getter]
    fn id(&self) -> String {
        TermStructure::id(&*self.inner).as_str().to_string()
    }

    /// Base (valuation) date of the curve.
    #[getter]
    fn base_date(&self) -> PyDate {
        PyDate::from_core(self.inner.base_date())
    }

    /// Index tenor in years.
    #[getter]
    fn tenor(&self) -> F {
        self.inner.tenor()
    }

    /// Reset lag in calendar days.
    #[getter]
    fn reset_lag(&self) -> i32 {
        self.inner.reset_lag()
    }

    /// Day count convention.
    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::from_inner(self.inner.day_count())
    }

    /// Forward rate at time t.
    fn rate(&self, t: F) -> F {
        self.inner.rate(t)
    }

    /// Average forward rate over [t1, t2].
    fn rate_period(&self, t1: F, t2: F) -> PyResult<F> {
        if t2 <= t1 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "t2 must be greater than t1",
            ));
        }
        Ok(finstack_core::market_data::traits::Forward::rate_period(&*self.inner, t1, t2))
    }

    fn __repr__(&self) -> String {
        format!(
            "ForwardCurve(id='{}', tenor={})",
            TermStructure::id(&*self.inner).as_str(),
            self.inner.tenor()
        )
    }
}

/// Credit hazard curve for survival probability calculations.
///
/// Represents the hazard rate (instantaneous default probability) for a
/// credit entity. The curve assumes piecewise-constant hazard rates between
/// knot points.
///
/// Examples:
///     >>> from rfin.market_data import HazardCurve
///     >>> from rfin import Date
///     
///     # Create a hazard curve
///     >>> curve = HazardCurve(
///     ...     id="CORP-AA",
///     ...     base_date=Date(2025, 1, 1),
///     ...     times=[0.0, 1.0, 3.0, 5.0, 10.0],
///     ...     hazard_rates=[0.001, 0.002, 0.003, 0.004, 0.005]
///     ... )
///     
///     # Survival probability
///     >>> curve.survival_probability(2.0)
///     0.997
///     
///     # Default probability over a period
///     >>> curve.default_probability(0.0, 5.0)
///     0.010
#[pyclass(name = "HazardCurve", module = "finstack.market_data")]
#[derive(Clone)]
pub struct PyHazardCurve {
    inner: Arc<CoreHazardCurve>,
}

#[pymethods]
impl PyHazardCurve {
    /// Create a new HazardCurve.
    ///
    /// Args:
    ///     id (str): Unique identifier for the curve
    ///     base_date (Date): The valuation date
    ///     times (List[float] | numpy.ndarray): Time points in years
    ///     hazard_rates (List[float] | numpy.ndarray): Hazard rates (lambda) at each time
    #[new]
    fn new(
        id: String,
        base_date: &PyDate,
        times: &Bound<'_, PyAny>,
        hazard_rates: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let times_vec = extract_f64_array(times)?;
        let hazards_vec = extract_f64_array(hazard_rates)?;

        if times_vec.len() != hazards_vec.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "times and hazard_rates must have the same length",
            ));
        }

        let id_static = Box::leak(id.into_boxed_str());
        let mut builder = CoreHazardCurve::builder(id_static).base_date(base_date.inner());

        // Add knots
        for (t, h) in times_vec.iter().zip(hazards_vec.iter()) {
            builder = builder.knots([(*t, *h)]);
        }

        let curve = builder.build().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to build curve: {:?}",
                e
            ))
        })?;

        Ok(PyHazardCurve {
            inner: Arc::new(curve),
        })
    }

    /// Unique identifier of the curve.
    #[getter]
    fn id(&self) -> String {
        TermStructure::id(&*self.inner).as_str().to_string()
    }

    /// Base (valuation) date of the curve.
    #[getter]
    fn base_date(&self) -> PyDate {
        PyDate::from_core(self.inner.base_date())
    }

    /// Survival probability to time t.
    fn survival_probability(&self, t: F) -> F {
        self.inner.sp(t)
    }

    /// Survival probability to time t (short alias).
    fn sp(&self, t: F) -> F {
        self.inner.sp(t)
    }

    /// Default probability between t1 and t2.
    fn default_probability(&self, t1: F, t2: F) -> PyResult<F> {
        if t2 < t1 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "t2 must be greater than or equal to t1",
            ));
        }
        Ok(self.inner.default_prob(t1, t2))
    }

    /// Default probability between t1 and t2 (short alias).
    fn dp(&self, t1: F, t2: F) -> PyResult<F> {
        self.default_probability(t1, t2)
    }

    fn __repr__(&self) -> String {
        format!(
            "HazardCurve(id='{}')",
            TermStructure::id(&*self.inner).as_str()
        )
    }
}

/// Consumer Price Index curve for inflation calculations.
///
/// Represents CPI levels over time, allowing calculation of inflation rates
/// between any two time points. The curve interpolates between known CPI levels.
///
/// Examples:
///     >>> from rfin.market_data import InflationCurve, InterpStyle
///     >>> from rfin import Date
///     
///     # Create an inflation curve
///     >>> curve = InflationCurve(
///     ...     id="US-CPI",
///     ...     base_cpi=300.0,
///     ...     times=[0.0, 1.0, 2.0, 5.0],
///     ...     cpi_levels=[300.0, 306.0, 312.24, 331.5],
///     ...     interpolation=InterpStyle.LogLinear
///     ... )
///     
///     # CPI at a future time
///     >>> curve.cpi(3.0)
///     318.7
///     
///     # Inflation rate over a period
///     >>> curve.inflation_rate(0.0, 2.0)  # 2% annualized
///     0.02
#[pyclass(name = "InflationCurve", module = "finstack.market_data")]
#[derive(Clone)]
pub struct PyInflationCurve {
    inner: Arc<CoreInflationCurve>,
}

#[pymethods]
impl PyInflationCurve {
    /// Create a new InflationCurve.
    ///
    /// Args:
    ///     id (str): Unique identifier for the curve
    ///     base_cpi (float): CPI level at time 0
    ///     times (List[float] | numpy.ndarray): Time points in years
    ///     cpi_levels (List[float] | numpy.ndarray): CPI levels at each time
    ///     interpolation (InterpStyle): Interpolation method (default: LogLinear)
    #[new]
    #[pyo3(signature = (id, base_cpi, times, cpi_levels, interpolation=PyInterpStyle::LogLinear))]
    fn new(
        id: String,
        base_cpi: F,
        times: &Bound<'_, PyAny>,
        cpi_levels: &Bound<'_, PyAny>,
        interpolation: PyInterpStyle,
    ) -> PyResult<Self> {
        let times_vec = extract_f64_array(times)?;
        let cpi_vec = extract_f64_array(cpi_levels)?;

        if times_vec.len() != cpi_vec.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "times and cpi_levels must have the same length",
            ));
        }

        let id_static = Box::leak(id.into_boxed_str());
        let mut builder = CoreInflationCurve::builder(id_static).base_cpi(base_cpi);

        // Add knots
        for (t, cpi) in times_vec.iter().zip(cpi_vec.iter()) {
            builder = builder.knots([(*t, *cpi)]);
        }

        // Set interpolation
        builder = match interpolation {
            PyInterpStyle::Linear => builder.linear_df(),
            PyInterpStyle::LogLinear => builder.log_df(),
            PyInterpStyle::MonotoneConvex => builder.monotone_convex(),
            PyInterpStyle::CubicHermite => builder.cubic_hermite(),
            PyInterpStyle::FlatForward => builder.flat_fwd(),
        };

        let curve = builder.build().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Failed to build curve: {:?}",
                e
            ))
        })?;

        Ok(PyInflationCurve {
            inner: Arc::new(curve),
        })
    }

    /// Unique identifier of the curve.
    #[getter]
    fn id(&self) -> String {
        TermStructure::id(&*self.inner).as_str().to_string()
    }

    /// Base CPI level at time 0.
    #[getter]
    fn base_cpi(&self) -> F {
        self.inner.cpi(0.0)
    }

    /// CPI level at time t.
    fn cpi(&self, t: F) -> F {
        self.inner.cpi(t)
    }

    /// Annualized inflation rate between t1 and t2.
    fn inflation_rate(&self, t1: F, t2: F) -> PyResult<F> {
        if t2 <= t1 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "t2 must be greater than t1",
            ));
        }
        Ok(self.inner.inflation_rate(t1, t2))
    }

    fn __repr__(&self) -> String {
        format!(
            "InflationCurve(id='{}')",
            TermStructure::id(&*self.inner).as_str()
        )
    }
}

// Helper function to extract f64 array from Python objects
pub(crate) fn extract_f64_array(obj: &Bound<'_, PyAny>) -> PyResult<Vec<F>> {
    // Try numpy array first
    if let Ok(array) = obj.extract::<numpy::PyReadonlyArray1<F>>() {
        return Ok(array.as_slice()?.to_vec());
    }

    // Try Python list
    if let Ok(list) = obj.downcast::<PyList>() {
        let mut vec = Vec::new();
        for item in list.iter() {
            vec.push(item.extract::<F>()?);
        }
        return Ok(vec);
    }

    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
        "Expected numpy array or list of floats",
    ))
}
