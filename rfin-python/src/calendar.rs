//! Python bindings for Calendar functionality (holiday calendars, business-day adjustment).

#![allow(clippy::useless_conversion)]

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyType;

use rfin_core::dates::holiday::calendars::calendar_by_id;
use rfin_core::dates::{adjust, BusinessDayConvention, CompositeCalendar, HolidayCalendar};

use crate::dates::PyDate;

/// Enum of supported business-day conventions (Python side)
#[pyclass(name = "BusDayConvention", module = "rfin.dates", eq)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyBusDayConv {
    Unadjusted,
    Following,
    ModifiedFollowing,
    Preceding,
    ModifiedPreceding,
}

impl From<PyBusDayConv> for BusinessDayConvention {
    fn from(c: PyBusDayConv) -> Self {
        match c {
            PyBusDayConv::Unadjusted => BusinessDayConvention::Unadjusted,
            PyBusDayConv::Following => BusinessDayConvention::Following,
            PyBusDayConv::ModifiedFollowing => BusinessDayConvention::ModifiedFollowing,
            PyBusDayConv::Preceding => BusinessDayConvention::Preceding,
            PyBusDayConv::ModifiedPreceding => BusinessDayConvention::ModifiedPreceding,
        }
    }
}

/// Calendar wrapper exposed to Python.
#[pyclass(name = "Calendar", module = "rfin.dates")]
#[derive(Clone)]
pub struct PyCalendar {
    /// Identifiers of calendars making up this (possibly composite) calendar.
    ids: Vec<String>,
}

#[pymethods]
impl PyCalendar {
    // ----------------------------
    // Constructors / factories
    // ----------------------------

    /// Create calendar from identifier (e.g. "gblo", "target2").
    #[classmethod]
    #[pyo3(text_signature = "(cls, id)")]
    #[allow(clippy::useless_conversion)]
    fn from_id(_cls: &Bound<'_, PyType>, id: &str) -> PyResult<PyCalendar> {
        match calendar_by_id(id) {
            Some(_) => Ok(PyCalendar {
                ids: vec![id.to_lowercase()],
            }),
            None => Err(PyValueError::new_err(format!("Unknown calendar id '{id}'"))),
        }
    }

    /// Return a calendar representing the union of the supplied calendars.
    #[classmethod]
    #[pyo3(text_signature = "(cls, calendars)")]
    fn union(_cls: &Bound<'_, PyType>, calendars: Vec<PyCalendar>) -> Self {
        let mut ids: Vec<String> = Vec::new();
        for cal in calendars {
            for id in cal.ids {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
        }
        PyCalendar { ids }
    }

    // ---------------------------------
    // Core functionality
    // ---------------------------------

    /// Check whether a given date is a holiday (or weekend) in this calendar.
    #[pyo3(text_signature = "(self, date)")]
    fn is_holiday(&self, date: &PyDate) -> bool {
        self.ids.iter().any(|id| {
            calendar_by_id(id.as_str())
                .map(|cal| cal.is_holiday(date.inner()))
                .unwrap_or(false)
        })
    }

    /// Adjust a date according to the specified business-day convention.
    #[pyo3(text_signature = "(self, date, convention)")]
    fn adjust(&self, date: &PyDate, convention: PyBusDayConv) -> PyDate {
        use rfin_core::dates::Date as CoreDate;
        let mut refs: Vec<&dyn HolidayCalendar> = Vec::new();
        for id in &self.ids {
            if let Some(cal) = calendar_by_id(id.as_str()) {
                refs.push(cal);
            }
        }
        // Fallback: if no calendars matched (shouldn't happen due to validation) just return original date.
        if refs.is_empty() {
            return date.clone();
        }
        let adj: CoreDate = if refs.len() == 1 {
            adjust(date.inner(), convention.into(), refs[0])
        } else {
            let comp = CompositeCalendar::merge(&refs);
            adjust(date.inner(), convention.into(), &comp)
        };
        PyDate::from_core(adj)
    }

    fn __repr__(&self) -> String {
        if self.ids.len() == 1 {
            format!("Calendar('{}')", self.ids[0])
        } else {
            format!("Calendar({:?})", self.ids)
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
