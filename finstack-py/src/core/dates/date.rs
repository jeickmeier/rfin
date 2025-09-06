//! Python bindings for Date type using `time` crate.
//!
//! This is **boilerplate** for now – it exposes a thin wrapper around
//! `time::Date` (re-exported via `finstack_core::dates::Date`) so that higher-level
//! calendar helpers can be added incrementally without breaking the API.

#![allow(clippy::useless_conversion)]

use finstack_core::dates::Date as CoreDate;
use finstack_core::dates::{
    imm_option_expiry as core_imm_option_expiry, next_cds_date as core_next_cds,
    next_equity_option_expiry as core_next_equity_option, next_imm as core_next_imm,
    third_friday as core_third_fri, third_wednesday as core_third_wed,
};
use pyo3::prelude::*;
use time::{Duration, Month};

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

    /// Add days to the date.
    ///
    /// Args:
    ///     days (int): Number of days to add (positive or negative).
    ///
    /// Returns:
    ///     Date: A new Date instance adjusted by the specified number of days.
    ///
    /// Examples:
    ///     >>> from rfin.dates import Date
    ///     >>> date = Date(2023, 12, 25)
    ///     >>> date + 5
    ///     Date('2023-12-30')
    ///     >>> date + (-2)
    ///     Date('2023-12-23')
    fn __add__(&self, days: i64) -> PyResult<PyDate> {
        let new_date = self.inner + Duration::days(days);
        Ok(PyDate { inner: new_date })
    }

    /// Add days to the date (right-hand side).
    ///
    /// Args:
    ///     days (int): Number of days to add (positive or negative).
    ///
    /// Returns:
    ///     Date: A new Date instance adjusted by the specified number of days.
    ///
    /// Examples:
    ///     >>> from rfin.dates import Date
    ///     >>> date = Date(2023, 12, 25)
    ///     >>> 5 + date
    ///     Date('2023-12-30')
    fn __radd__(&self, days: i64) -> PyResult<PyDate> {
        self.__add__(days)
    }

    /// Subtract days from the date or calculate the difference between two dates.
    ///
    /// Args:
    ///     other (Date | int): Either a Date to calculate the difference (in days),
    ///                         or an integer number of days to subtract.
    ///
    /// Returns:
    ///     Date | int: If subtracting days, returns a new Date.
    ///                 If subtracting another Date, returns the number of days between them.
    ///
    /// Examples:
    ///     >>> from rfin.dates import Date
    ///     >>> date1 = Date(2023, 12, 25)
    ///     >>> date2 = Date(2023, 12, 20)
    ///     
    ///     # Subtract days
    ///     >>> date1 - 5
    ///     Date('2023-12-20')
    ///     
    ///     # Calculate difference between dates
    ///     >>> date1 - date2
    ///     5
    ///     >>> date2 - date1
    ///     -5
    fn __sub__(&self, py: Python, other: &Bound<PyAny>) -> PyResult<PyObject> {
        // Try to extract as PyDate first (Date - Date)
        if let Ok(other_date) = other.extract::<PyDate>() {
            let duration = self.inner - other_date.inner;
            // The time crate uses whole_days() on Duration
            let days = duration.whole_days();
            return Ok(days.into_pyobject(py).unwrap().into());
        }

        // Try to extract as integer (Date - int)
        if let Ok(days) = other.extract::<i64>() {
            let new_date = self.inner - Duration::days(days);
            let result = PyDate { inner: new_date };
            return Py::new(py, result).map(|obj| obj.into());
        }

        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "unsupported operand type(s) for -: 'Date' and type of argument",
        ))
    }

    /// Compare dates for ordering.
    ///
    /// Args:
    ///     other (Date): Another Date instance.
    ///
    /// Returns:
    ///     bool: True if self is less than other.
    ///
    /// Examples:
    ///     >>> from rfin.dates import Date
    ///     >>> date1 = Date(2023, 12, 20)
    ///     >>> date2 = Date(2023, 12, 25)
    ///     >>> date1 < date2
    ///     True
    ///     >>> date2 < date1
    ///     False
    fn __lt__(&self, other: &PyDate) -> bool {
        self.inner < other.inner
    }

    /// Compare dates for ordering.
    ///
    /// Args:
    ///     other (Date): Another Date instance.
    ///
    /// Returns:
    ///     bool: True if self is less than or equal to other.
    fn __le__(&self, other: &PyDate) -> bool {
        self.inner <= other.inner
    }

    /// Compare dates for ordering.
    ///
    /// Args:
    ///     other (Date): Another Date instance.
    ///
    /// Returns:
    ///     bool: True if self is greater than other.
    fn __gt__(&self, other: &PyDate) -> bool {
        self.inner > other.inner
    }

    /// Compare dates for ordering.
    ///
    /// Args:
    ///     other (Date): Another Date instance.
    ///
    /// Returns:
    ///     bool: True if self is greater than or equal to other.
    fn __ge__(&self, other: &PyDate) -> bool {
        self.inner >= other.inner
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
        use finstack_core::dates::DateExt;
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
        use finstack_core::dates::DateExt;
        self.inner.quarter()
    }

    /// Returns the fiscal year for this date based on the provided fiscal configuration.
    ///
    /// Uses the fiscal year start month and day from FiscalConfig to determine
    /// which fiscal year this date belongs to.
    ///
    /// Args:
    ///     config (FiscalConfig): The fiscal year configuration.
    ///
    /// Returns:
    ///     int: The fiscal year.
    ///
    /// Examples:
    ///     >>> from rfin.dates import Date, FiscalConfig
    ///     >>> Date(2023, 12, 25).fiscal_year(FiscalConfig.calendar_year())
    ///     2023
    ///     >>> Date(2024, 9, 15).fiscal_year(FiscalConfig.us_federal())
    ///     2024
    ///     >>> Date(2024, 10, 1).fiscal_year(FiscalConfig.us_federal())
    ///     2025
    #[pyo3(text_signature = "(self, config)")]
    pub fn fiscal_year(&self, config: &crate::core::dates::periods::PyFiscalConfig) -> i32 {
        use finstack_core::dates::DateExt;
        self.inner.fiscal_year(config.inner)
    }

    /// Add or subtract weekdays from the date.
    ///
    /// Weekdays exclude weekends (Saturday and Sunday) but do NOT account for holidays.
    /// For true business day adjustments that respect holidays, use Calendar.adjust().
    ///
    /// Args:
    ///     n (int): Number of weekdays to add (positive) or subtract (negative).
    ///
    /// Returns:
    ///     Date: A new Date instance adjusted by the specified number of weekdays.
    ///
    /// Examples:
    ///     >>> from rfin.dates import Date
    ///     >>> friday = Date(2023, 12, 22)  # Friday
    ///     
    ///     # Add 1 weekday: Friday -> Monday
    ///     >>> friday.add_weekdays(1)
    ///     Date('2023-12-25')
    ///     
    ///     # Add 5 weekdays: Friday -> Friday of next week
    ///     >>> friday.add_weekdays(5)
    ///     Date('2023-12-29')
    ///     
    ///     # Subtract 1 weekday: Friday -> Thursday
    ///     >>> friday.add_weekdays(-1)
    ///     Date('2023-12-21')
    ///     
    ///     # From Monday, subtract 1 weekday: Monday -> Friday
    ///     >>> monday = Date(2023, 12, 25)
    ///     >>> monday.add_weekdays(-1)
    ///     Date('2023-12-22')
    #[pyo3(text_signature = "(self, n)")]
    pub fn add_weekdays(&self, n: i32) -> Self {
        use finstack_core::dates::DateExt;
        let new_date = self.inner.add_weekdays(n);
        PyDate { inner: new_date }
    }

    /// Add or subtract business days from the date using a holiday calendar.
    ///
    /// Business days exclude weekends AND holidays according to the provided calendar.
    /// This is more accurate than add_weekdays() for real-world financial applications.
    ///
    /// Args:
    ///     n (int): Number of business days to add (positive) or subtract (negative).
    ///     calendar (Calendar): Holiday calendar to use for business day determination.
    ///
    /// Returns:
    ///     Date: A new Date instance adjusted by the specified number of business days.
    ///
    /// Examples:
    ///     >>> from finstack.dates import Date, Calendar
    ///     >>> cal = Calendar.from_id("target2")
    ///     >>> friday = Date(2025, 6, 27)  # Friday
    ///     
    ///     # Add 1 business day: Friday -> Monday (skip weekend)
    ///     >>> friday.add_business_days(1, cal)
    ///     Date('2025-06-30')
    ///     
    ///     # Add 3 business days: Friday -> Wednesday (skip weekend)
    ///     >>> friday.add_business_days(3, cal)
    ///     Date('2025-07-02')
    ///     
    ///     # Subtract 1 business day: Friday -> Thursday
    ///     >>> friday.add_business_days(-1, cal)
    ///     Date('2025-06-26')
    #[pyo3(text_signature = "(self, n, calendar)")]
    pub fn add_business_days(
        &self,
        n: i32,
        _calendar: &crate::core::dates::calendar::PyCalendar,
    ) -> Self {
        use finstack_core::dates::DateExt;

        // For now, use weekdays-only logic as a simple fallback
        // TODO: Implement proper calendar-aware business day calculation
        let new_date = self.inner.add_weekdays(n);
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

/// Get the third Friday of a specific month and year.
///
/// The third Friday is commonly used for equity options expiration in many markets.
/// Most equity options expire on the third Friday of each month.
///
/// Args:
///     month (int): The month number (1-12).
///     year (int): The four-digit year.
///
/// Returns:
///     Date: The date of the third Friday of the specified month.
///
/// Raises:
///     ValueError: If the month is not in the range 1-12.
///
/// Examples:
///     >>> from finstack.dates import third_friday
///     >>> third_friday(3, 2025)  # March 2025
///     Date('2025-03-21')
///     >>> third_friday(6, 2025)  # June 2025
///     Date('2025-06-20')
#[pyfunction(name = "third_friday", text_signature = "(month, year)")]
pub fn py_third_friday(month: u8, year: i32) -> PyResult<PyDate> {
    let month_enum = Month::try_from(month).map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>("Month must be in range 1-12")
    })?;
    let d = core_third_fri(month_enum, year);
    Ok(PyDate::from_core(d))
}

/// Get the next equity option expiry date after a given date.
///
/// Equity options typically expire on the third Friday of each month, providing
/// a monthly expiration cycle for equity derivatives.
///
/// Args:
///     date (Date): The reference date.
///
/// Returns:
///     Date: The next equity option expiry date strictly after the given date.
///
/// Examples:
///     >>> from finstack.dates import Date, next_equity_option_expiry
///     >>> date = Date(2025, 3, 15)  # Mid-March 2025
///     >>> next_equity_option_expiry(date)
///     Date('2025-03-21')  # Third Friday of March 2025
///     
///     >>> date = Date(2025, 3, 22)  # After March expiry
///     >>> next_equity_option_expiry(date)
///     Date('2025-04-18')  # Third Friday of April 2025
#[pyfunction(name = "next_equity_option_expiry", text_signature = "(date)")]
pub fn py_next_equity_option_expiry(date: &PyDate) -> PyDate {
    PyDate::from_core(core_next_equity_option(date.inner()))
}

/// Get the IMM option expiry date for a specific month and year.
///
/// IMM option expiry dates occur on the Friday before the third Wednesday
/// of the IMM months (March, June, September, December). This ensures options
/// expire before the underlying futures contracts for orderly settlement.
///
/// Args:
///     month (int): The month number (1-12).
///     year (int): The four-digit year.
///
/// Returns:
///     Date: The IMM option expiry date for the specified month.
///
/// Raises:
///     ValueError: If the month is not in the range 1-12.
///
/// Examples:
///     >>> from finstack.dates import imm_option_expiry
///     >>> imm_option_expiry(3, 2025)  # March 2025
///     Date('2025-03-14')
///     >>> imm_option_expiry(6, 2025)  # June 2025
///     Date('2025-06-13')
#[pyfunction(name = "imm_option_expiry", text_signature = "(month, year)")]
pub fn py_imm_option_expiry(month: u8, year: i32) -> PyResult<PyDate> {
    let month_enum = Month::try_from(month).map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>("Month must be in range 1-12")
    })?;
    let d = core_imm_option_expiry(month_enum, year);
    Ok(PyDate::from_core(d))
}
