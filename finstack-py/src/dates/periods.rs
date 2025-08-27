//! Python bindings for period/frequency functionality.

use finstack_core::dates::{build_periods, Period, PeriodId};
use pyo3::prelude::*;
use pyo3::types::PyList;

use super::PyDate;



/// Python wrapper for PeriodId.
///
/// Represents a specific period within a year (e.g., Q1, M03, W01).
#[pyclass(name = "PeriodId")]
#[derive(Clone, Debug)]
pub struct PyPeriodId {
    inner: PeriodId,
}

#[pymethods]
impl PyPeriodId {
    /// Create a quarterly period ID (e.g., Q1-Q4).
    #[staticmethod]
    #[pyo3(name = "quarter")]
    fn py_quarter(year: i32, q: u8) -> PyResult<Self> {
        if !(1..=4).contains(&q) {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Quarter must be 1-4",
            ));
        }
        Ok(Self {
            inner: PeriodId::quarter(year, q),
        })
    }

    /// Create a monthly period ID (e.g., M01-M12).
    #[staticmethod]
    #[pyo3(name = "month")]
    fn py_month(year: i32, m: u8) -> PyResult<Self> {
        if !(1..=12).contains(&m) {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Month must be 1-12",
            ));
        }
        Ok(Self {
            inner: PeriodId::month(year, m),
        })
    }

    /// Create a weekly period ID (e.g., W01-W53).
    #[staticmethod]
    #[pyo3(name = "week")]
    fn py_week(year: i32, w: u8) -> PyResult<Self> {
        if !(1..=53).contains(&w) {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Week must be 1-53",
            ));
        }
        Ok(Self {
            inner: PeriodId::week(year, w),
        })
    }

    /// Create a semi-annual period ID (H1 or H2).
    #[staticmethod]
    #[pyo3(name = "half")]
    fn py_half(year: i32, h: u8) -> PyResult<Self> {
        if !(1..=2).contains(&h) {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Half must be 1-2",
            ));
        }
        Ok(Self {
            inner: PeriodId::half(year, h),
        })
    }

    /// Create an annual period ID.
    #[staticmethod]
    #[pyo3(name = "annual")]
    fn py_annual(year: i32) -> Self {
        Self {
            inner: PeriodId::annual(year),
        }
    }

    /// Parse a period ID from string (e.g., "2025Q1", "2025M03").
    #[new]
    fn new(s: &str) -> PyResult<Self> {
        s.parse::<PeriodId>()
            .map(|inner| Self { inner })
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{}", e)))
    }

    #[getter]
    fn year(&self) -> i32 {
        self.inner.year
    }

    #[getter]
    fn index(&self) -> u8 {
        self.inner.index
    }

    /// Get the frequency type as a string.
    #[getter]
    fn frequency(&self) -> String {
        // Extract the frequency from the string representation
        let s = self.inner.to_string();
        if s.contains('Q') {
            "Quarterly".to_string()
        } else if s.contains('M') {
            "Monthly".to_string()
        } else if s.contains('W') {
            "Weekly".to_string()
        } else if s.contains('H') {
            "SemiAnnual".to_string()
        } else {
            "Annual".to_string()
        }
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("PeriodId('{}')", self.inner)
    }
}

/// Python wrapper for Period.
///
/// Represents a period with start/end dates and an actuals flag.
#[pyclass(name = "Period")]
#[derive(Clone, Debug)]
pub struct PyPeriod {
    inner: Period,
}

#[pymethods]
impl PyPeriod {
    #[getter]
    fn id(&self) -> PyPeriodId {
        PyPeriodId {
            inner: self.inner.id,
        }
    }

    #[getter]
    fn start(&self) -> PyDate {
        PyDate::from_core(self.inner.start)
    }

    #[getter]
    fn end(&self) -> PyDate {
        PyDate::from_core(self.inner.end)
    }

    #[getter]
    fn is_actual(&self) -> bool {
        self.inner.is_actual
    }

    fn __str__(&self) -> String {
        format!(
            "{}: {} to {} (actual={})",
            self.inner.id, self.inner.start, self.inner.end, self.inner.is_actual
        )
    }

    fn __repr__(&self) -> String {
        format!("Period(id='{}', start='{}', end='{}', is_actual={})",
            self.inner.id, self.inner.start, self.inner.end, self.inner.is_actual)
    }
}

/// Generate a sequence of periods.
///
/// Args:
///     range: Period range string (e.g., "2025Q1-2026Q4", "2025M01-2025M12")
///     actuals_until: Optional cutoff for actuals (e.g., "2025Q2", "2025M06")
///
/// Returns:
///     List of Period objects
///
/// Example:
///     >>> periods = build_periods("2025Q1-2026Q4", "2025Q2")
///     >>> for p in periods:
///     ...     print(f"{p.id}: actual={p.is_actual}")
#[pyfunction]
#[pyo3(name = "build_periods")]
pub fn py_build_periods(
    py: Python<'_>,
    range: &str,
    actuals_until: Option<&str>,
) -> PyResult<Py<PyList>> {
    let plan = build_periods(range, actuals_until).map_err(|e| {
        pyo3::exceptions::PyValueError::new_err(format!("Failed to build periods: {}", e))
    })?;

    let periods: Vec<PyPeriod> = plan
        .periods
        .into_iter()
        .map(|p| PyPeriod { inner: p })
        .collect();

    Ok(PyList::new(py, periods)?.into())
}
