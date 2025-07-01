//! Python bindings for Date type using `time` crate.
//!
//! This is **boilerplate** for now – it exposes a thin wrapper around
//! `time::Date` (re-exported via `rfin_core::Date`) so that higher-level
//! calendar helpers can be added incrementally without breaking the API.

use pyo3::prelude::*;
use rfin_core::Date as CoreDate;
use time::Month;

/// Python wrapper for a calendar date (YYYY-MM-DD).
#[pyclass(name = "Date", module = "rfin.dates")]
#[derive(Clone)]
pub struct PyDate {
    inner: CoreDate,
}

#[pymethods]
impl PyDate {
    /// Create a new `Date` from `year`, `month`, `day` components.
    ///
    /// Args:
    ///     year (int): four-digit year (e.g. 2025)
    ///     month (int): month number 1-12
    ///     day (int): calendar day 1-31
    #[new]
    #[pyo3(text_signature = "(year, month, day)")]
    pub fn new(year: i32, month: u8, day: u8) -> PyResult<Self> {
        let month_enum = Month::try_from(month).map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Month must be in range 1-12",
            )
        })?;
        let date = CoreDate::from_calendar_date(year, month_enum, day).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid date components: {}", e),
            )
        })?;
        Ok(PyDate { inner: date })
    }

    /// Year component.
    #[getter]
    fn year(&self) -> i32 {
        self.inner.year()
    }

    /// Month component (1-12).
    #[getter]
    fn month(&self) -> u8 {
        self.inner.month() as u8
    }

    /// Day component (1-31).
    #[getter]
    fn day(&self) -> u8 {
        self.inner.day()
    }

    /// Return `YYYY-MM-DD` representation.
    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("Date('{}')", self.inner)
    }

    fn __eq__(&self, other: &PyDate) -> bool {
        self.inner == other.inner
    }

    /// Check if the date falls on a weekend (Saturday or Sunday).
    #[pyo3(text_signature = "(self)")]
    pub fn is_weekend(&self) -> bool {
        use rfin_core::DateExt;
        self.inner.is_weekend()
    }

    /// Return the calendar quarter (1–4).
    #[pyo3(text_signature = "(self)")]
    pub fn quarter(&self) -> u8 {
        use rfin_core::DateExt;
        self.inner.quarter()
    }

    /// Return the fiscal year (currently equal to the calendar year).
    #[pyo3(text_signature = "(self)")]
    pub fn fiscal_year(&self) -> i32 {
        use rfin_core::DateExt;
        self.inner.fiscal_year()
    }

    /// Add or subtract a number of business days (weekend-aware).
    /// Positive numbers move forward, negative numbers backward.
    #[pyo3(text_signature = "(self, n)")]
    pub fn add_business_days(&self, n: i32) -> Self {
        use rfin_core::DateExt;
        let new_date = self.inner.add_business_days(n);
        PyDate { inner: new_date }
    }
}

impl PyDate {
    /// Internal helper to expose the inner date type to other bindings.
    pub fn inner(&self) -> CoreDate {
        self.inner
    }
} 