//! Python bindings for Date type using `time` crate.
//!
//! This is **boilerplate** for now – it exposes a thin wrapper around
//! `time::Date` (re-exported via `finstack_core::Date`) so that higher-level
//! calendar helpers can be added incrementally without breaking the API.

#![allow(clippy::useless_conversion)]

use finstack_core::dates::{
    next_cds_date as core_next_cds, next_imm as core_next_imm, third_wednesday as core_third_wed,
};
use finstack_core::Date as CoreDate;
use pyo3::prelude::*;
use time::Month;

/// Calendar date representation (YYYY-MM-DD).
///
/// A Date represents a specific calendar date with year, month, and day components.
/// It provides various utilities for date manipulation and business day calculations.
///
/// Dates are immutable and support various operations including:
/// - Business day arithmetic
/// - Weekend detection
/// - Quarter and fiscal year calculations
/// - Integration with holiday calendars
///
/// Examples:
///     >>> from rfin.dates import Date
///     
///     # Create a date
///     >>> date = Date(2023, 12, 25)
///     >>> date
///     Date('2023-12-25')
///     
///     # Access components
///     >>> date.year
///     2023
///     >>> date.month
///     12
///     >>> date.day
///     25
///     
///     # Check if it's a weekend
///     >>> date.is_weekend()
///     False  # Monday
///     
///     # Business day arithmetic
///     >>> date.add_business_days(1)
///     Date('2023-12-26')
///     >>> date.add_business_days(-1)
///     Date('2023-12-22')  # Friday
///     
///     # Get quarter
///     >>> date.quarter()
///     4
#[pyclass(name = "Date", module = "finstack.dates")]
#[derive(Clone)]
pub struct PyDate {
    inner: CoreDate,
}

#[pymethods]
impl PyDate {
    /// Create a new Date from year, month, and day components.
    ///
    /// Args:
    ///     year (int): Four-digit year (e.g., 2023).
    ///     month (int): Month number (1-12, where 1=January, 12=December).
    ///     day (int): Day of the month (1-31, depending on the month).
    ///
    /// Returns:
    ///     Date: A new Date instance.
    ///
    /// Raises:
    ///     ValueError: If the date components are invalid (e.g., February 30).
    ///
    /// Examples:
    ///     >>> from rfin.dates import Date
    ///     >>> date = Date(2023, 12, 25)
    ///     >>> date
    ///     Date('2023-12-25')
    ///     
    ///     # Invalid dates raise ValueError
    ///     >>> Date(2023, 2, 30)  # February 30th doesn't exist
    ///     Traceback (most recent call last):
    ///         ...
    ///     ValueError: Invalid date components: ...
    ///     
    ///     # Month must be 1-12
    ///     >>> Date(2023, 13, 1)
    ///     Traceback (most recent call last):
    ///         ...
    ///     ValueError: Month must be in range 1-12
    #[new]
    #[pyo3(text_signature = "(year, month, day)")]
    pub fn new(year: i32, month: u8, day: u8) -> PyResult<Self> {
        let month_enum = Month::try_from(month).map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>("Month must be in range 1-12")
        })?;
        let date = CoreDate::from_calendar_date(year, month_enum, day).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid date components: {}",
                e
            ))
        })?;
        Ok(PyDate { inner: date })
    }

    /// The year component of the date.
    ///
    /// Returns:
    ///     int: The four-digit year.
    ///
    /// Examples:
    ///     >>> from rfin.dates import Date
    ///     >>> date = Date(2023, 12, 25)
    ///     >>> date.year
    ///     2023
    #[getter]
    fn year(&self) -> i32 {
        self.inner.year()
    }

    /// The month component of the date.
    ///
    /// Returns:
    ///     int: The month number (1-12, where 1=January, 12=December).
    ///
    /// Examples:
    ///     >>> from rfin.dates import Date
    ///     >>> date = Date(2023, 12, 25)
    ///     >>> date.month
    ///     12
    #[getter]
    fn month(&self) -> u8 {
        self.inner.month() as u8
    }

    /// The day component of the date.
    ///
    /// Returns:
    ///     int: The day of the month (1-31).
    ///
    /// Examples:
    ///     >>> from rfin.dates import Date
    ///     >>> date = Date(2023, 12, 25)
    ///     >>> date.day
    ///     25
    #[getter]
    fn day(&self) -> u8 {
        self.inner.day()
    }

    /// Return the date in YYYY-MM-DD format.
    ///
    /// Returns:
    ///     str: The date formatted as YYYY-MM-DD.
    ///
    /// Examples:
    ///     >>> from rfin.dates import Date
    ///     >>> date = Date(2023, 12, 25)
    ///     >>> str(date)
    ///     '2023-12-25'
    fn __str__(&self) -> String {
        self.inner.to_string()
    }

    /// Return the string representation of the date.
    ///
    /// Returns:
    ///     str: A string like "Date('2023-12-25')".
    ///
    /// Examples:
    ///     >>> from rfin.dates import Date
    ///     >>> date = Date(2023, 12, 25)
    ///     >>> repr(date)
    ///     "Date('2023-12-25')"
    fn __repr__(&self) -> String {
        format!("Date('{}')", self.inner)
    }

    /// Check equality between two dates.
    ///
    /// Args:
    ///     other (Date): Another Date instance.
    ///
    /// Returns:
    ///     bool: True if the dates are the same.
    ///
    /// Examples:
    ///     >>> from rfin.dates import Date
    ///     >>> date1 = Date(2023, 12, 25)
    ///     >>> date2 = Date(2023, 12, 25)
    ///     >>> date3 = Date(2023, 12, 26)
    ///     >>> date1 == date2
    ///     True
    ///     >>> date1 == date3
    ///     False
    fn __eq__(&self, other: &PyDate) -> bool {
        self.inner == other.inner
    }

    /// Check if the date falls on a weekend.
    ///
    /// Returns:
    ///     bool: True if the date is a Saturday or Sunday.
    ///
    /// Examples:
    ///     >>> from rfin.dates import Date
    ///     >>> Date(2023, 12, 23).is_weekend()  # Saturday
    ///     True
    ///     >>> Date(2023, 12, 24).is_weekend()  # Sunday
    ///     True
    ///     >>> Date(2023, 12, 25).is_weekend()  # Monday
    ///     False
    #[pyo3(text_signature = "(self)")]
    pub fn is_weekend(&self) -> bool {
        use finstack_core::DateExt;
        self.inner.is_weekend()
    }

    /// Get the calendar quarter of the date.
    ///
    /// Returns:
    ///     int: The quarter number (1-4).
    ///
    /// Examples:
    ///     >>> from rfin.dates import Date
    ///     >>> Date(2023, 1, 15).quarter()  # January
    ///     1
    ///     >>> Date(2023, 4, 15).quarter()  # April
    ///     2
    ///     >>> Date(2023, 7, 15).quarter()  # July
    ///     3
    ///     >>> Date(2023, 10, 15).quarter()  # October
    ///     4
    #[pyo3(text_signature = "(self)")]
    pub fn quarter(&self) -> u8 {
        use finstack_core::DateExt;
        self.inner.quarter()
    }

    /// Get the fiscal year of the date.
    ///
    /// Currently returns the same as the calendar year.
    /// Future versions may support different fiscal year conventions.
    ///
    /// Returns:
    ///     int: The fiscal year.
    ///
    /// Examples:
    ///     >>> from rfin.dates import Date
    ///     >>> Date(2023, 12, 25).fiscal_year()
    ///     2023
    #[pyo3(text_signature = "(self)")]
    pub fn fiscal_year(&self) -> i32 {
        use finstack_core::DateExt;
        self.inner.fiscal_year()
    }

    /// Add or subtract business days from the date.
    ///
    /// Business days exclude weekends (Saturday and Sunday).
    /// This method does not account for holidays - use Calendar.adjust() for that.
    ///
    /// Args:
    ///     n (int): Number of business days to add (positive) or subtract (negative).
    ///
    /// Returns:
    ///     Date: A new Date instance adjusted by the specified number of business days.
    ///
    /// Examples:
    ///     >>> from rfin.dates import Date
    ///     >>> friday = Date(2023, 12, 22)  # Friday
    ///     
    ///     # Add 1 business day: Friday -> Monday
    ///     >>> friday.add_business_days(1)
    ///     Date('2023-12-25')
    ///     
    ///     # Add 5 business days: Friday -> Friday of next week
    ///     >>> friday.add_business_days(5)
    ///     Date('2023-12-29')
    ///     
    ///     # Subtract 1 business day: Friday -> Thursday
    ///     >>> friday.add_business_days(-1)
    ///     Date('2023-12-21')
    ///     
    ///     # From Monday, subtract 1 business day: Monday -> Friday
    ///     >>> monday = Date(2023, 12, 25)
    ///     >>> monday.add_business_days(-1)
    ///     Date('2023-12-22')
    #[pyo3(text_signature = "(self, n)")]
    pub fn add_business_days(&self, n: i32) -> Self {
        use finstack_core::DateExt;
        let new_date = self.inner.add_business_days(n);
        PyDate { inner: new_date }
    }
}

impl PyDate {
    /// Internal helper to expose the inner date type to other bindings.
    pub fn inner(&self) -> CoreDate {
        self.inner
    }

    /// Internal helper to construct a PyDate from a core value.
    pub fn from_core(inner: CoreDate) -> Self {
        PyDate { inner }
    }
}

/// Get the third Wednesday of a specific month and year.
///
/// The third Wednesday is commonly used in financial markets for various
/// purposes, including options expiration dates and futures settlement.
///
/// Args:
///     month (int): The month number (1-12).
///     year (int): The four-digit year.
///
/// Returns:
///     Date: The date of the third Wednesday of the specified month.
///
/// Raises:
///     ValueError: If the month is not in the range 1-12.
///
/// Examples:
///     >>> from rfin.dates import third_wednesday
///     >>> third_wednesday(12, 2023)  # December 2023
///     Date('2023-12-20')
///     >>> third_wednesday(1, 2024)   # January 2024
///     Date('2024-01-17')
///     
///     # Invalid month raises ValueError
///     >>> third_wednesday(13, 2023)
///     Traceback (most recent call last):
///         ...
///     ValueError: Month must be in range 1-12
#[pyfunction(name = "third_wednesday", text_signature = "(month, year)")]
pub fn py_third_wednesday(month: u8, year: i32) -> PyResult<PyDate> {
    let month_enum = Month::try_from(month).map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>("Month must be in range 1-12")
    })?;
    let d = core_third_wed(month_enum, year);
    Ok(PyDate::from_core(d))
}

/// Get the next IMM (International Monetary Market) date after a given date.
///
/// IMM dates are the third Wednesday of March, June, September, and December.
/// They are widely used in derivatives markets for futures and options expiration.
///
/// Args:
///     date (Date): The reference date.
///
/// Returns:
///     Date: The next IMM date strictly after the given date.
///
/// Examples:
///     >>> from rfin.dates import Date, next_imm
///     >>> date = Date(2023, 11, 15)  # November 15, 2023
///     >>> next_imm(date)
///     Date('2023-12-20')  # Third Wednesday of December 2023
///     
///     >>> date = Date(2023, 12, 20)  # On the IMM date
///     >>> next_imm(date)
///     Date('2024-03-20')  # Next IMM date in March 2024
///     
///     >>> date = Date(2023, 12, 25)  # After December IMM date
///     >>> next_imm(date)
///     Date('2024-03-20')  # Next IMM date in March 2024
#[pyfunction(name = "next_imm", text_signature = "(date)")]
pub fn py_next_imm(date: &PyDate) -> PyDate {
    PyDate::from_core(core_next_imm(date.inner()))
}

/// Get the next CDS (Credit Default Swap) roll date after a given date.
///
/// CDS roll dates are standardized dates used in credit derivatives markets,
/// typically the 20th of March, June, September, and December.
/// If the 20th falls on a weekend, the actual roll date may be adjusted.
///
/// Args:
///     date (Date): The reference date.
///
/// Returns:
///     Date: The next CDS roll date strictly after the given date.
///
/// Examples:
///     >>> from rfin.dates import Date, next_cds_date
///     >>> date = Date(2023, 11, 15)  # November 15, 2023
///     >>> next_cds_date(date)
///     Date('2023-12-20')  # December 20, 2023
///     
///     >>> date = Date(2023, 12, 20)  # On the CDS roll date
///     >>> next_cds_date(date)
///     Date('2024-03-20')  # Next CDS roll date in March 2024
///     
///     >>> date = Date(2023, 12, 25)  # After December CDS roll date
///     >>> next_cds_date(date)
///     Date('2024-03-20')  # Next CDS roll date in March 2024
#[pyfunction(name = "next_cds_date", text_signature = "(date)")]
pub fn py_next_cds_date(date: &PyDate) -> PyDate {
    PyDate::from_core(core_next_cds(date.inner()))
}
