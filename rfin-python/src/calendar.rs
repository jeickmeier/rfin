#![allow(deprecated)]

//! Python bindings for Calendar functionality (holiday calendars, business-day adjustment).

use pyo3::prelude::*;
use pyo3::types::PyType;

use rfin_core::dates::{BusDayConv, HolidayCalendar, Target2, adjust};

use crate::dates::PyDate;

/// Enum of supported business-day conventions (Python side)
#[pyclass(name = "BusDayConvention", module = "rfin.dates")]
#[derive(Clone, Copy)]
pub enum PyBusDayConv {
    Unadjusted,
    Following,
    ModifiedFollowing,
    Preceding,
    ModifiedPreceding,
}

impl From<PyBusDayConv> for BusDayConv {
    fn from(c: PyBusDayConv) -> Self {
        match c {
            PyBusDayConv::Unadjusted => BusDayConv::Unadjusted,
            PyBusDayConv::Following => BusDayConv::Following,
            PyBusDayConv::ModifiedFollowing => BusDayConv::ModifiedFollowing,
            PyBusDayConv::Preceding => BusDayConv::Preceding,
            PyBusDayConv::ModifiedPreceding => BusDayConv::ModifiedPreceding,
        }
    }
}

#[derive(Clone)]
enum CalendarKind {
    Target2(Target2),
}

impl CalendarKind {
    fn as_hcal(&self) -> &dyn HolidayCalendar {
        match self {
            CalendarKind::Target2(cal) => cal,
        }
    }
}

/// Calendar wrapper exposed to Python.
#[pyclass(name = "Calendar", module = "rfin.dates")]
#[derive(Clone)]
pub struct PyCalendar {
    kind: CalendarKind,
}

#[pymethods]
impl PyCalendar {
    /// TARGET2 calendar (European settlement days).
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn target2(_cls: &Bound<'_, PyType>) -> Self {
        PyCalendar {
            kind: CalendarKind::Target2(Target2::new()),
        }
    }

    /// Check whether a given date is a holiday (or weekend) in this calendar.
    #[pyo3(text_signature = "(self, date)")]
    fn is_holiday(&self, date: &PyDate) -> bool {
        self.kind.as_hcal().is_holiday(date.inner())
    }

    /// Adjust a date according to the specified business-day convention.
    #[pyo3(text_signature = "(self, date, convention)")]
    fn adjust(&self, date: &PyDate, convention: PyBusDayConv) -> PyDate {
        let adj = match &self.kind {
            CalendarKind::Target2(cal) => adjust(date.inner(), convention.into(), cal),
        };
        PyDate::from_core(adj)
    }

    fn __repr__(&self) -> &'static str {
        match self.kind {
            CalendarKind::Target2(_) => "Calendar('TARGET2')",
        }
    }
}

// -----------------------------------------------------------------------------
// Module-level helpers
// -----------------------------------------------------------------------------

/// Return list of built-in calendar identifiers available in this build.
#[pyfunction(name = "available_calendars")]
pub fn py_available_calendars() -> Vec<String> {
    rfin_core::dates::available_calendars()
        .iter()
        .map(|s| s.to_string())
        .collect()
} 