//! Python bindings for inflation index (CPI/RPI) functionality.

use crate::core::currency::PyCurrency;
use crate::core::dates::PyDate;
use finstack_core::market_data::scalars::inflation_index::{
    InflationIndex as CoreInflationIndex, InflationIndexBuilder as CoreBuilder,
    InflationInterpolation as CoreInterpolation, InflationLag as CoreLag,
};
use pyo3::prelude::*;
use pyo3::types::PyList;

/// Interpolation method for index values between observations.
///
/// Available methods:
/// - `Step`: Last observation carried forward (typical for monthly CPI)
/// - `Linear`: Linear interpolation between observed points
#[pyclass(name = "InflationInterpolation")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PyInflationInterpolation {
    /// Last observation carried forward (typical for monthly CPI)
    Step,
    /// Linear interpolation between observed points
    Linear,
}

#[pymethods]
impl PyInflationInterpolation {
    #[classattr]
    const STEP: Self = Self::Step;

    #[classattr]
    const LINEAR: Self = Self::Linear;

    fn __str__(&self) -> &'static str {
        match self {
            Self::Step => "Step",
            Self::Linear => "Linear",
        }
    }

    fn __repr__(&self) -> String {
        format!("InflationInterpolation.{}", self.__str__())
    }
}

impl From<PyInflationInterpolation> for CoreInterpolation {
    fn from(py: PyInflationInterpolation) -> Self {
        match py {
            PyInflationInterpolation::Step => CoreInterpolation::Step,
            PyInflationInterpolation::Linear => CoreInterpolation::Linear,
        }
    }
}

impl From<CoreInterpolation> for PyInflationInterpolation {
    fn from(core: CoreInterpolation) -> Self {
        match core {
            CoreInterpolation::Step => PyInflationInterpolation::Step,
            CoreInterpolation::Linear => PyInflationInterpolation::Linear,
        }
    }
}

/// Lag policy for index application.
///
/// Available policies:
/// - `months(n)`: Lag by specified number of months (e.g., 3-month lag for TIPS)
/// - `days(n)`: Lag by specified number of calendar days
/// - `none()`: No lag applied
#[pyclass(name = "InflationLag")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PyInflationLag {
    inner: CoreLag,
}

#[pymethods]
impl PyInflationLag {
    /// Create a lag policy with a specified number of months.
    ///
    /// Args:
    ///     months: Number of months to lag (e.g., 3 for US TIPS, 2 for UK ILBs)
    ///
    /// Example:
    ///     >>> lag = InflationLag.months(3)
    #[staticmethod]
    fn months(months: u8) -> Self {
        Self {
            inner: CoreLag::Months(months),
        }
    }

    /// Create a lag policy with a specified number of days.
    ///
    /// Args:
    ///     days: Number of calendar days to lag
    ///
    /// Example:
    ///     >>> lag = InflationLag.days(90)
    #[staticmethod]
    fn days(days: u16) -> Self {
        Self {
            inner: CoreLag::Days(days),
        }
    }

    /// Create a no-lag policy.
    ///
    /// Example:
    ///     >>> lag = InflationLag.none()
    #[staticmethod]
    fn none() -> Self {
        Self {
            inner: CoreLag::None,
        }
    }

    fn __str__(&self) -> String {
        match self.inner {
            CoreLag::Months(m) => format!("{}-month lag", m),
            CoreLag::Days(d) => format!("{}-day lag", d),
            CoreLag::None => "No lag".to_string(),
        }
    }

    fn __repr__(&self) -> String {
        match self.inner {
            CoreLag::Months(m) => format!("InflationLag.months({})", m),
            CoreLag::Days(d) => format!("InflationLag.days({})", d),
            CoreLag::None => "InflationLag.none()".to_string(),
        }
    }
}

impl From<CoreLag> for PyInflationLag {
    fn from(core: CoreLag) -> Self {
        Self { inner: core }
    }
}

/// Inflation index for historical CPI/RPI data.
///
/// Stores economic index observations (CPI/RPI) using Polars DataFrames
/// for efficient time-series operations. Supports interpolation, lag policies,
/// and seasonal adjustments for inflation-linked bond calculations.
///
/// Example:
///     >>> from finstack import Date, Currency
///     >>> from finstack.market_data import InflationIndex, InflationInterpolation, InflationLag
///     >>>
///     >>> # Create CPI observations
///     >>> observations = [
///     ...     (Date(2023, 1, 31), 300.0),
///     ...     (Date(2023, 2, 28), 303.0),
///     ...     (Date(2023, 3, 31), 306.0),
///     ...     (Date(2023, 4, 30), 309.0),
///     ... ]
///     >>>
///     >>> # Create index with step interpolation (default for CPI)
///     >>> cpi = InflationIndex("US-CPI", observations, Currency.USD)
///     >>> cpi = cpi.with_interpolation(InflationInterpolation.STEP)
///     >>> cpi = cpi.with_lag(InflationLag.months(3))  # 3-month lag for TIPS
///     >>>
///     >>> # Get index value on a specific date
///     >>> value = cpi.value_on(Date(2023, 3, 15))
///     >>>
///     >>> # Calculate index ratio for inflation adjustment
///     >>> ratio = cpi.ratio(Date(2023, 1, 31), Date(2023, 4, 30))
#[pyclass(name = "InflationIndex")]
#[derive(Clone)]
pub struct PyInflationIndex {
    inner: CoreInflationIndex,
}

#[pymethods]
impl PyInflationIndex {
    /// Create a new inflation index from observations.
    ///
    /// Args:
    ///     id: Unique identifier (e.g., "US-CPI-U", "UK-RPI")
    ///     observations: List of (date, value) tuples
    ///     currency: Currency of the index
    ///     interpolation: Optional interpolation method (default: Step)
    ///     lag: Optional lag policy (default: no lag)
    ///
    /// Returns:
    ///     A new InflationIndex instance
    ///
    /// Raises:
    ///     ValueError: If observations are empty or contain duplicate dates
    #[new]
    #[pyo3(signature = (id, observations, currency, interpolation=None, lag=None))]
    fn new(
        id: String,
        observations: &Bound<'_, PyList>,
        currency: PyCurrency,
        interpolation: Option<PyInflationInterpolation>,
        lag: Option<PyInflationLag>,
    ) -> PyResult<Self> {
        // Convert Python list of tuples to Rust Vec
        let mut obs_vec = Vec::new();
        for item in observations.iter() {
            let tuple = item.extract::<(PyDate, f64)>()?;
            obs_vec.push((tuple.0.inner(), tuple.1));
        }

        // Create the index
        let mut index = CoreInflationIndex::new(id, obs_vec, currency.inner()).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Failed to create inflation index: {}",
                e
            ))
        })?;

        // Apply optional interpolation
        if let Some(interp) = interpolation {
            index = index.with_interpolation(interp.into());
        }

        // Apply optional lag
        if let Some(lag) = lag {
            index = index.with_lag(lag.inner);
        }

        Ok(Self { inner: index })
    }

    /// Set the interpolation method.
    ///
    /// Args:
    ///     interpolation: The interpolation method to use
    ///
    /// Returns:
    ///     Self for method chaining
    fn with_interpolation(&mut self, interpolation: PyInflationInterpolation) -> PyResult<Self> {
        let new_inner = self.inner.clone().with_interpolation(interpolation.into());
        Ok(Self { inner: new_inner })
    }

    /// Set the lag policy.
    ///
    /// Args:
    ///     lag: The lag policy to apply
    ///
    /// Returns:
    ///     Self for method chaining
    fn with_lag(&mut self, lag: PyInflationLag) -> PyResult<Self> {
        let new_inner = self.inner.clone().with_lag(lag.inner);
        Ok(Self { inner: new_inner })
    }

    /// Add seasonal adjustment factors.
    ///
    /// Args:
    ///     factors: List of 12 monthly seasonal factors (Jan-Dec)
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     ValueError: If factors list doesn't contain exactly 12 elements
    fn with_seasonality(&mut self, factors: Vec<f64>) -> PyResult<Self> {
        if factors.len() != 12 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Seasonal factors must contain exactly 12 values (one per month)",
            ));
        }

        let factors_array: [f64; 12] = factors.try_into().unwrap();
        let new_inner = self
            .inner
            .clone()
            .with_seasonality(factors_array)
            .map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Failed to set seasonality: {}", e))
            })?;

        Ok(Self { inner: new_inner })
    }

    /// Get the index value on a given date.
    ///
    /// Applies interpolation, lag, and seasonal adjustments as configured.
    ///
    /// Args:
    ///     date: The date to get the index value for
    ///
    /// Returns:
    ///     The index value on the given date
    ///
    /// Raises:
    ///     ValueError: If the date is invalid or calculation fails
    fn value_on(&self, date: PyDate) -> PyResult<f64> {
        self.inner.value_on(date.inner()).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Failed to get value: {}", e))
        })
    }

    /// Calculate index ratio I(settle_date)/I(base_date).
    ///
    /// This is commonly used in inflation-linked bond calculations.
    ///
    /// Args:
    ///     base_date: The base date for the ratio
    ///     settle_date: The settlement date for the ratio
    ///
    /// Returns:
    ///     The index ratio
    ///
    /// Raises:
    ///     ValueError: If calculation fails or base value is zero
    fn ratio(&self, base_date: PyDate, settle_date: PyDate) -> PyResult<f64> {
        self.inner
            .ratio(base_date.inner(), settle_date.inner())
            .map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Failed to calculate ratio: {}", e))
            })
    }

    /// Get the date range covered by observations.
    ///
    /// Returns:
    ///     Tuple of (start_date, end_date) or None if no observations
    fn date_range(&self) -> PyResult<Option<(PyDate, PyDate)>> {
        match self.inner.date_range() {
            Ok((start, end)) => Ok(Some((PyDate::from_core(start), PyDate::from_core(end)))),
            Err(_) => Ok(None),
        }
    }

    /// Get the index identifier.
    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    /// Get the currency.
    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::from_inner(self.inner.currency)
    }

    /// Get the interpolation method.
    #[getter]
    fn interpolation(&self) -> PyInflationInterpolation {
        self.inner.interpolation.into()
    }

    /// Get the lag policy.
    #[getter]
    fn lag(&self) -> PyInflationLag {
        self.inner.lag.into()
    }

    fn __str__(&self) -> String {
        format!(
            "InflationIndex(id='{}', currency={})",
            self.inner.id, self.inner.currency
        )
    }

    fn __repr__(&self) -> String {
        format!(
            "InflationIndex(id='{}', currency={}, interpolation={:?}, lag={:?})",
            self.inner.id, self.inner.currency, self.inner.interpolation, self.inner.lag
        )
    }
}

/// Builder for creating inflation indices.
///
/// Provides a fluent interface for constructing InflationIndex instances
/// with various configuration options.
///
/// Example:
///     >>> builder = InflationIndexBuilder("UK-RPI", Currency.GBP)
///     >>> builder.add_observation(Date(2023, 1, 31), 348.7)
///     >>> builder.add_observation(Date(2023, 2, 28), 351.2)
///     >>> builder.with_interpolation(InflationInterpolation.LINEAR)
///     >>> builder.with_lag(InflationLag.months(2))
///     >>> index = builder.build()
#[pyclass(name = "InflationIndexBuilder")]
pub struct PyInflationIndexBuilder {
    id: String,
    currency: finstack_core::currency::Currency,
    observations: Vec<(finstack_core::dates::Date, f64)>,
    interpolation: CoreInterpolation,
    lag: CoreLag,
    seasonality: Option<[f64; 12]>,
}

#[pymethods]
impl PyInflationIndexBuilder {
    /// Create a new inflation index builder.
    ///
    /// Args:
    ///     id: Unique identifier for the index
    ///     currency: Currency of the index
    #[new]
    fn new(id: String, currency: PyCurrency) -> Self {
        Self {
            id,
            currency: currency.inner(),
            observations: Vec::new(),
            interpolation: CoreInterpolation::default(),
            lag: CoreLag::default(),
            seasonality: None,
        }
    }

    /// Add a single observation to the index.
    ///
    /// Args:
    ///     date: Observation date
    ///     value: Index value on that date
    ///
    /// Returns:
    ///     None (mutates in place)
    fn add_observation(&mut self, date: PyDate, value: f64) -> PyResult<()> {
        self.observations.push((date.inner(), value));
        Ok(())
    }

    /// Set all observations at once.
    ///
    /// Args:
    ///     observations: List of (date, value) tuples
    ///
    /// Returns:
    ///     None (mutates in place)
    fn with_observations(&mut self, observations: &Bound<'_, PyList>) -> PyResult<()> {
        self.observations.clear();
        for item in observations.iter() {
            let tuple = item.extract::<(PyDate, f64)>()?;
            self.observations.push((tuple.0.inner(), tuple.1));
        }
        Ok(())
    }

    /// Set the interpolation method.
    ///
    /// Args:
    ///     interpolation: The interpolation method to use
    ///
    /// Returns:
    ///     None (mutates in place)
    fn with_interpolation(&mut self, interpolation: PyInflationInterpolation) -> PyResult<()> {
        self.interpolation = interpolation.into();
        Ok(())
    }

    /// Set the lag policy.
    ///
    /// Args:
    ///     lag: The lag policy to apply
    ///
    /// Returns:
    ///     None (mutates in place)
    fn with_lag(&mut self, lag: PyInflationLag) -> PyResult<()> {
        self.lag = lag.inner;
        Ok(())
    }

    /// Set seasonal adjustment factors.
    ///
    /// Args:
    ///     factors: List of 12 monthly seasonal factors (Jan-Dec)
    ///
    /// Returns:
    ///     None (mutates in place)
    ///
    /// Raises:
    ///     ValueError: If factors list doesn't contain exactly 12 elements
    fn with_seasonality(&mut self, factors: Vec<f64>) -> PyResult<()> {
        if factors.len() != 12 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Seasonal factors must contain exactly 12 values (one per month)",
            ));
        }

        let factors_array: [f64; 12] = factors.try_into().unwrap();
        self.seasonality = Some(factors_array);
        Ok(())
    }

    /// Build the inflation index.
    ///
    /// Returns:
    ///     The constructed InflationIndex
    ///
    /// Raises:
    ///     ValueError: If construction fails (e.g., no observations)
    fn build(&self) -> PyResult<PyInflationIndex> {
        // Use the builder pattern
        let mut builder = CoreBuilder::new(self.id.clone(), self.currency);
        builder = builder.with_observations(self.observations.clone());
        builder = builder.with_interpolation(self.interpolation);
        builder = builder.with_lag(self.lag);

        if let Some(factors) = self.seasonality {
            builder = builder.with_seasonality(factors);
        }

        let index = builder.build().map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Failed to build index: {}", e))
        })?;

        Ok(PyInflationIndex { inner: index })
    }
}
