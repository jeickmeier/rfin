//! Python bindings for holiday calendars and business day conventions

#![allow(clippy::useless_conversion)]

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyType;

use finstack_core::dates::calendar::calendar_by_id;
use finstack_core::dates::{adjust, BusinessDayConvention, CompositeCalendar, HolidayCalendar};

use super::date::PyDate;

/// Business day convention enumeration for date adjustment.
///
/// This enum defines how dates are adjusted when they fall on holidays or weekends.
/// The conventions follow standard market practices for financial instruments.
///
/// Attributes:
///     Unadjusted: No adjustment - return the original date even if it's a holiday
///     Following: Move to the next business day
///     ModifiedFollowing: Move to the next business day, unless it crosses into the next month,
///                       in which case move to the previous business day
///     Preceding: Move to the previous business day
///     ModifiedPreceding: Move to the previous business day, unless it crosses into the previous month,
///                       in which case move to the next business day
///
/// Examples:
///     >>> from rfin.dates import BusDayConvention, Calendar, Date
///     >>> cal = Calendar.from_id("gblo")
///     >>> date = Date(2023, 12, 25)  # Christmas Day
///     >>> cal.adjust(date, BusDayConvention.Following)
///     Date('2023-12-26')
///     >>> cal.adjust(date, BusDayConvention.Preceding)
///     Date('2023-12-22')
#[pyclass(name = "BusDayConvention", module = "finstack.dates", eq)]
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

impl PyBusDayConv {
    /// Get the inner BusinessDayConvention value
    pub fn inner(&self) -> BusinessDayConvention {
        (*self).into()
    }
}

/// Holiday calendar for business day calculations.
///
/// A Calendar defines which dates are considered holidays in a particular jurisdiction
/// or market. Calendars can be combined to create composite calendars that represent
/// the union of multiple markets' holidays.
///
/// Calendars are used for:
/// - Determining if a date is a business day
/// - Adjusting dates according to business day conventions
/// - Generating payment schedules for financial instruments
///
/// Examples:
///     >>> from rfin.dates import Calendar, Date, BusDayConvention
///     
///     # Create a calendar for London
///     >>> gblo = Calendar.from_id("gblo")
///     >>> date = Date(2023, 12, 25)  # Christmas Day
///     >>> gblo.is_holiday(date)
///     True
///     
///     # Adjust the date using following convention
///     >>> gblo.adjust(date, BusDayConvention.Following)
///     Date('2023-12-26')
///     
///     # Create a composite calendar (London + New York)
///     >>> nyse = Calendar.from_id("nyse")
///     >>> composite = Calendar.union([gblo, nyse])
///     >>> composite.is_holiday(Date(2023, 7, 4))  # US Independence Day
///     True
///     
///     # List available calendars
///     >>> from rfin.dates import available_calendars
///     >>> cals = available_calendars()
///     >>> 'gblo' in cals
///     True
#[pyclass(name = "Calendar", module = "finstack.dates")]
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

    /// Create a calendar from a standard identifier.
    ///
    /// Args:
    ///     id (str): Calendar identifier (e.g., "gblo", "nyse", "target2").
    ///               Case-insensitive. See `available_calendars()` for full list.
    ///
    /// Returns:
    ///     Calendar: A new calendar instance for the specified market.
    ///
    /// Raises:
    ///     ValueError: If the calendar identifier is not recognized.
    ///
    /// Examples:
    ///     >>> from rfin.dates import Calendar
    ///     >>> gblo = Calendar.from_id("gblo")  # London
    ///     >>> nyse = Calendar.from_id("NYSE")  # New York (case-insensitive)
    ///     >>> target2 = Calendar.from_id("target2")  # European Central Bank
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

    /// Create a composite calendar from multiple calendars.
    ///
    /// The resulting calendar considers a date to be a holiday if it's a holiday
    /// in ANY of the constituent calendars (union operation).
    ///
    /// Args:
    ///     calendars (List[Calendar]): List of Calendar instances to combine.
    ///
    /// Returns:
    ///     Calendar: A composite calendar representing the union of all input calendars.
    ///
    /// Examples:
    ///     >>> from rfin.dates import Calendar
    ///     >>> gblo = Calendar.from_id("gblo")
    ///     >>> nyse = Calendar.from_id("nyse")
    ///     >>> composite = Calendar.union([gblo, nyse])
    ///     >>> # Now a date is a holiday if it's a holiday in London OR New York
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

    /// Check if a date is a holiday (including weekends).
    ///
    /// Args:
    ///     date (Date): The date to check.
    ///
    /// Returns:
    ///     bool: True if the date is a holiday or weekend, False otherwise.
    ///
    /// Examples:
    ///     >>> from rfin.dates import Calendar, Date
    ///     >>> gblo = Calendar.from_id("gblo")
    ///     >>> gblo.is_holiday(Date(2023, 12, 25))  # Christmas Day
    ///     True
    ///     >>> gblo.is_holiday(Date(2023, 12, 26))  # Boxing Day
    ///     True
    ///     >>> gblo.is_holiday(Date(2023, 12, 27))  # Regular business day
    ///     False
    ///     >>> gblo.is_holiday(Date(2023, 12, 23))  # Saturday
    ///     True
    #[pyo3(text_signature = "(self, date)")]
    fn is_holiday(&self, date: &PyDate) -> bool {
        self.ids.iter().any(|id| {
            calendar_by_id(id.as_str())
                .map(|cal| cal.is_holiday(date.inner()))
                .unwrap_or(false)
        })
    }

    /// Adjust a date according to a business day convention.
    ///
    /// When a date falls on a holiday or weekend, this method moves it to the
    /// nearest business day according to the specified convention.
    ///
    /// Args:
    ///     date (Date): The date to adjust.
    ///     convention (BusDayConvention): The business day convention to apply.
    ///
    /// Returns:
    ///     Date: The adjusted date.
    ///
    /// Examples:
    ///     >>> from rfin.dates import Calendar, Date, BusDayConvention
    ///     >>> gblo = Calendar.from_id("gblo")
    ///     >>> christmas = Date(2023, 12, 25)  # Monday, Christmas Day
    ///     
    ///     # Move to next business day
    ///     >>> gblo.adjust(christmas, BusDayConvention.Following)
    ///     Date('2023-12-27')  # Wednesday (Boxing Day is also a holiday)
    ///     
    ///     # Move to previous business day
    ///     >>> gblo.adjust(christmas, BusDayConvention.Preceding)
    ///     Date('2023-12-22')  # Friday
    ///     
    ///     # No adjustment
    ///     >>> gblo.adjust(christmas, BusDayConvention.Unadjusted)
    ///     Date('2023-12-25')  # Returns original date
    #[pyo3(text_signature = "(self, date, convention)")]
    fn adjust(&self, date: &PyDate, convention: PyBusDayConv) -> PyDate {
        use finstack_core::dates::Date as CoreDate;
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
                .expect("Date adjustment should not fail")
        } else {
            let comp = CompositeCalendar::new(&refs);
            adjust(date.inner(), convention.into(), &comp).expect("Date adjustment should not fail")
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

/// Get a list of all available built-in calendar identifiers.
///
/// Returns all calendar identifiers that can be used with `Calendar.from_id()`.
/// The identifiers are standardized market codes for major financial centers.
///
/// Returns:
///     List[str]: List of available calendar identifiers.
///
/// Examples:
///     >>> from rfin.dates import available_calendars
///     >>> calendars = available_calendars()
///     >>> 'gblo' in calendars  # London
///     True
///     >>> 'nyse' in calendars  # New York Stock Exchange
///     True
///     >>> 'target2' in calendars  # European Central Bank
///     True
///
/// Common calendar identifiers include:
///     - "gblo": London (UK)
///     - "nyse": New York Stock Exchange (US)
///     - "usny": New York (US)
///     - "target2": European Central Bank
///     - "jpto": Tokyo (Japan)
///     - "hkex": Hong Kong Exchange
///     - "sgsi": Singapore
///     - "asx": Australian Securities Exchange
#[pyfunction(name = "available_calendars")]
pub fn py_available_calendars() -> Vec<String> {
    finstack_core::dates::available_calendars()
        .iter()
        .map(|s| s.to_string())
        .collect()
}
