#![allow(clippy::useless_conversion)]

//! Python bindings for DayCount conventions.
//!
//! Exposes the `DayCount` enum with helper methods `days` and `year_fraction`.
//! Usage example:
//! ```python
//! from rfin.dates import Date, DayCount
//! d1, d2 = Date(2025,1,1), Date(2026,1,1)
//! yf = DayCount.act360().year_fraction(d1, d2)
//! ```

use crate::dates::PyDate;
use pyo3::prelude::*;
use pyo3::types::PyType;
use rfin_core::dates::DayCount as CoreDayCount; // need PyDate

/// Python wrapper around the `DayCount` enum
#[pyclass(name = "DayCount", module = "rfin.dates")]
#[derive(Clone)]
pub struct PyDayCount {
    inner: CoreDayCount,
}

#[pymethods]
impl PyDayCount {
    // ---------------------------------------------------------------------
    // Constructors / constants
    // ---------------------------------------------------------------------
    #[classmethod]
    fn act360(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: CoreDayCount::Act360,
        }
    }

    #[classmethod]
    fn act365f(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: CoreDayCount::Act365F,
        }
    }

    #[classmethod]
    fn thirty360(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: CoreDayCount::Thirty360,
        }
    }

    #[classmethod]
    fn thirty_e_360(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: CoreDayCount::ThirtyE360,
        }
    }

    #[classmethod]
    fn actact(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: CoreDayCount::ActAct,
        }
    }

    // ---------------------------------------------------------------------
    // Methods
    // ---------------------------------------------------------------------
    /// Return day-count between two dates (number of days) following the convention.
    #[pyo3(text_signature = "(self, start, end)")]
    pub fn days(&self, start: &PyDate, end: &PyDate) -> PyResult<i32> {
        self.inner
            .days(start.inner(), end.inner())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Return year fraction between two dates following the convention.
    #[pyo3(text_signature = "(self, start, end)")]
    pub fn year_fraction(&self, start: &PyDate, end: &PyDate) -> PyResult<f64> {
        self.inner
            .year_fraction(start.inner(), end.inner())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    // ---------------------------------------------------------------------
    // Dunder / repr helpers
    // ---------------------------------------------------------------------
    fn __repr__(&self) -> String {
        format!("DayCount('{}')", self.__str__())
    }

    fn __str__(&self) -> String {
        let s = match self.inner {
            CoreDayCount::Act360 => "ACT/360",
            CoreDayCount::Act365F => "ACT/365F",
            CoreDayCount::Thirty360 => "30/360",
            CoreDayCount::ThirtyE360 => "30E/360",
            CoreDayCount::ActAct => "ACT/ACT",
            _ => "<unknown>",
        };
        s.to_string()
    }

    fn __eq__(&self, other: &PyDayCount) -> bool {
        self.inner == other.inner
    }
}

impl PyDayCount {
    /// Internal helper to get inner enum
    pub fn inner(&self) -> CoreDayCount {
        self.inner
    }
}
