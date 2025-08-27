//! Python bindings for index series (CPI/RPI) functionality.

use finstack_core::dates::{
    IndexId, IndexInterpolation, IndexLag, IndexSeries as CoreIndexSeries,
};
use pyo3::prelude::*;
use rust_decimal::Decimal;


use super::PyDate;

/// Interpolation method for index values between observation points.
#[pyclass(name = "IndexInterpolation")]
#[derive(Clone, Copy, Debug)]
pub enum PyIndexInterpolation {
    /// Last observation carried forward (typical for monthly CPI)
    Step,
    /// Linear interpolation between observed points
    Linear,
}

impl From<PyIndexInterpolation> for IndexInterpolation {
    fn from(interp: PyIndexInterpolation) -> Self {
        match interp {
            PyIndexInterpolation::Step => IndexInterpolation::Step,
            PyIndexInterpolation::Linear => IndexInterpolation::Linear,
        }
    }
}

impl From<IndexInterpolation> for PyIndexInterpolation {
    fn from(interp: IndexInterpolation) -> Self {
        match interp {
            IndexInterpolation::Step => PyIndexInterpolation::Step,
            IndexInterpolation::Linear => PyIndexInterpolation::Linear,
        }
    }
}

/// Lag policy for index application.
#[pyclass(name = "IndexLag")]
#[derive(Clone, Debug)]
pub struct PyIndexLag {
    inner: IndexLag,
}



/// Economic index series with observation data and policies.
///
/// Used for CPI/RPI calculations in inflation-linked bonds.
///
/// Example:
///     >>> from finstack import Date, IndexSeries, IndexInterpolation, IndexLag
///     >>> 
///     >>> # Create CPI series
///     >>> observations = [
///     ...     (Date(2023, 1, 31), 300.0),
///     ...     (Date(2023, 2, 28), 303.0),
///     ...     (Date(2023, 3, 31), 306.0),
///     ... ]
///     >>> 
///     >>> series = IndexSeries("US-CPI", observations)
///     >>> series.with_interpolation(IndexInterpolation.Linear)
///     >>> series.with_lag(IndexLag.months(3))
///     >>> 
///     >>> # Get index value on a date
///     >>> value = series.value_on(Date(2023, 2, 15))
///     >>> 
///     >>> # Calculate index ratio
///     >>> ratio = series.ratio(Date(2023, 1, 31), Date(2023, 3, 31))
#[pyclass(name = "IndexSeries")]
#[derive(Clone)]
pub struct PyIndexSeries {
    inner: CoreIndexSeries,
}

#[pymethods]
impl PyIndexSeries {
    /// Create a new index series.
    ///
    /// Args:
    ///     id: Unique identifier for the series (e.g., "US-CPI", "UK-RPI")
    ///     observations: List of (date, value) tuples
    ///     interpolation: Interpolation method (default: Step)
    ///     lag: Lag policy (default: None)
    #[new]
    #[pyo3(signature = (id, observations, interpolation=None, lag=None))]
    fn new(
        id: String,
        observations: Vec<(PyDate, f64)>,
        interpolation: Option<PyIndexInterpolation>,
        lag: Option<PyIndexLag>,
    ) -> PyResult<Self> {
        let obs: Vec<(finstack_core::Date, Decimal)> = observations
            .into_iter()
            .map(|(date, value)| {
                let decimal_value = Decimal::try_from(value)
                    .map_err(|_| pyo3::exceptions::PyValueError::new_err(
                        format!("Failed to convert value {} to decimal", value)))?;
                Ok((date.inner(), decimal_value))
            })
            .collect::<PyResult<Vec<_>>>()?;

        let mut series = CoreIndexSeries::new(IndexId::new(id), obs).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Failed to create index series: {}", e))
        })?;

        if let Some(interp) = interpolation {
            series = series.with_interpolation(interp.into());
        }

        if let Some(l) = lag {
            series = series.with_lag(l.inner);
        }

        Ok(Self { inner: series })
    }

    /// Set the interpolation method.
    fn with_interpolation(&mut self, interpolation: PyIndexInterpolation) -> PyResult<()> {
        self.inner = self.inner.clone().with_interpolation(interpolation.into());
        Ok(())
    }

    /// Set the lag policy.
    fn with_lag(&mut self, lag: PyIndexLag) -> PyResult<()> {
        self.inner = self.inner.clone().with_lag(lag.inner);
        Ok(())
    }

    /// Set seasonal adjustment factors (12 factors for Jan-Dec).
    ///
    /// Args:
    ///     factors: List of 12 seasonal adjustment factors
    fn with_seasonality(&mut self, factors: Vec<f64>) -> PyResult<()> {
        if factors.len() != 12 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Seasonal factors must have exactly 12 elements (one per month)",
            ));
        }

        let mut arr = [Decimal::from(1); 12];
        for (i, &factor) in factors.iter().enumerate() {
            arr[i] = Decimal::try_from(factor)
                .map_err(|_| pyo3::exceptions::PyValueError::new_err(
                    format!("Failed to convert seasonality factor {} to decimal", factor)))?;
        }
        self.inner = self.inner.clone().with_seasonality(arr);
        Ok(())
    }

    /// Get the index value on a specific date.
    ///
    /// Args:
    ///     date: The date to get the index value for
    ///
    /// Returns:
    ///     The index value after applying lag, interpolation, and seasonality
    fn value_on(&self, date: PyDate) -> PyResult<f64> {
        let decimal_value = self.inner
            .value_on(date.inner())
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Failed to get value: {}", e)))?;
        
        decimal_value.try_into()
            .map_err(|_| pyo3::exceptions::PyValueError::new_err(
                format!("Failed to convert decimal value {} to f64", decimal_value)))
    }

    /// Calculate index ratio I(settle_date)/I(base_date).
    ///
    /// Args:
    ///     base_date: The base date for the ratio
    ///     settle_date: The settlement date for the ratio
    ///
    /// Returns:
    ///     The index ratio
    fn ratio(&self, base_date: PyDate, settle_date: PyDate) -> PyResult<f64> {
        let decimal_ratio = self.inner
            .ratio(base_date.inner(), settle_date.inner())
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Failed to calculate ratio: {}", e)))?;
        
        decimal_ratio.try_into()
            .map_err(|_| pyo3::exceptions::PyValueError::new_err(
                format!("Failed to convert decimal ratio {} to f64", decimal_ratio)))
    }

    /// Get the index identifier.
    #[getter]
    fn id(&self) -> String {
        self.inner.id().as_str().to_string()
    }

    /// Get the number of observations.
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Get the date range covered by observations.
    ///
    /// Returns:
    ///     Tuple of (first_date, last_date) or None if empty
    fn date_range(&self) -> Option<(PyDate, PyDate)> {
        self.inner
            .date_range()
            .map(|(start, end)| (PyDate::from_core(start), PyDate::from_core(end)))
    }

    fn __str__(&self) -> String {
        format!("IndexSeries(id='{}', observations={})", self.inner.id(), self.inner.len())
    }

    fn __repr__(&self) -> String {
        format!("IndexSeries(id='{}', observations={})", self.inner.id(), self.inner.len())
    }
}

#[pymethods]
impl PyIndexLag {
    /// Create a lag of N months.
    #[staticmethod]
    fn months(months: u8) -> Self {
        PyIndexLag { inner: IndexLag::Months(months) }
    }

    /// Create a lag of N days.
    #[staticmethod]
    fn days(days: u16) -> Self {
        PyIndexLag { inner: IndexLag::Days(days) }
    }

    /// No lag.
    #[staticmethod]
    fn none() -> Self {
        PyIndexLag { inner: IndexLag::None }
    }

    fn __str__(&self) -> String {
        match &self.inner {
            IndexLag::Months(months) => format!("{}M lag", months),
            IndexLag::Days(days) => format!("{}D lag", days),
            IndexLag::None => "No lag".to_string(),
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            IndexLag::Months(months) => format!("IndexLag.months({})", months),
            IndexLag::Days(days) => format!("IndexLag.days({})", days),
            IndexLag::None => "IndexLag.none()".to_string(),
        }
    }
}
