#![allow(deprecated)]

//! Python bindings for Calendar functionality (holiday calendars, business-day adjustment).

use pyo3::prelude::*;
use pyo3::types::PyType;

use rfin_core::dates::{BusDayConv, HolidayCalendar, Target2, adjust, CompositeCalendar};
use rfin_core::dates::calendars::Gblo;

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
    Gblo(Gblo),
    Union(Vec<CalendarKind>),
}

impl CalendarKind {
    fn as_hcal(&self) -> &dyn HolidayCalendar {
        match self {
            CalendarKind::Target2(cal) => cal,
            CalendarKind::Gblo(cal) => cal,
            CalendarKind::Union(_) => unreachable!("Composite calendars handled separately"),
        }
    }

    fn is_holiday(&self, date: rfin_core::dates::Date) -> bool {
        match self {
            CalendarKind::Target2(cal) => cal.is_holiday(date),
            CalendarKind::Gblo(cal) => cal.is_holiday(date),
            CalendarKind::Union(list) => list.iter().any(|c| c.is_holiday(date)),
        }
    }

    fn collect_refs<'a>(&'a self, out: &mut Vec<&'a dyn HolidayCalendar>) {
        match self {
            CalendarKind::Target2(cal) => out.push(cal),
            CalendarKind::Gblo(cal) => out.push(cal),
            CalendarKind::Union(list) => {
                for c in list {
                    c.collect_refs(out);
                }
            }
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

    /// U.K. inter-bank calendar (GBLO).
    #[classmethod]
    #[pyo3(text_signature = "(cls)")]
    fn gblo(_cls: &Bound<'_, PyType>) -> Self {
        PyCalendar {
            kind: CalendarKind::Gblo(Gblo::new()),
        }
    }

    /// Return a calendar that is the union of the supplied calendars.
    #[classmethod]
    #[pyo3(text_signature = "(cls, calendars)")]
    fn union(_cls: &Bound<'_, PyType>, calendars: Vec<PyCalendar>) -> Self {
        let kinds = calendars.into_iter().map(|c| c.kind).collect();
        PyCalendar {
            kind: CalendarKind::Union(kinds),
        }
    }

    /// Check whether a given date is a holiday (or weekend) in this calendar.
    #[pyo3(text_signature = "(self, date)")]
    fn is_holiday(&self, date: &PyDate) -> bool {
        self.kind.is_holiday(date.inner())
    }

    /// Adjust a date according to the specified business-day convention.
    #[pyo3(text_signature = "(self, date, convention)")]
    fn adjust(&self, date: &PyDate, convention: PyBusDayConv) -> PyDate {
        use rfin_core::dates::Date as CoreDate;
        let adj: CoreDate = match &self.kind {
            CalendarKind::Target2(cal) => adjust(date.inner(), convention.into(), cal),
            CalendarKind::Gblo(cal) => adjust(date.inner(), convention.into(), cal),
            CalendarKind::Union(list) => {
                let mut refs: Vec<&dyn HolidayCalendar> = Vec::new();
                for c in list {
                    c.collect_refs(&mut refs);
                }
                let comp = CompositeCalendar::merge(&refs);
                adjust(date.inner(), convention.into(), &comp)
            }
        };
        PyDate::from_core(adj)
    }

    fn __repr__(&self) -> &'static str {
        match self.kind {
            CalendarKind::Target2(_) => "Calendar('TARGET2')",
            CalendarKind::Gblo(_) => "Calendar('GBLO')",
            CalendarKind::Union(_) => "Calendar('Union')",
        }
    }
}

// Separate inherent impl block for internal helpers (not exposed to Python)
impl PyCalendar {
    /// Internal: obtain reference to underlying HolidayCalendar trait object.
    pub(crate) fn hcal(&self) -> &dyn HolidayCalendar {
        self.kind.as_hcal()
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