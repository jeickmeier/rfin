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

/// Day count convention for interest accrual calculations.
///
/// Day count conventions define how to calculate the number of days and the
/// year fraction between two dates for interest accrual purposes. Different
/// conventions are used in different markets and for different instruments.
///
/// The convention affects both the day count (numerator) and the year basis
/// (denominator) used in interest calculations.
///
/// Examples:
///     >>> from rfin.dates import Date, DayCount
///     >>> start = Date(2023, 1, 1)
///     >>> end = Date(2023, 7, 1)  # 6 months later
///     
///     # Different conventions give different results
///     >>> act360 = DayCount.act360()
///     >>> act360.days(start, end)
///     181
///     >>> act360.year_fraction(start, end)
///     0.5027777777777778
///     
///     >>> thirty360 = DayCount.thirty360()
///     >>> thirty360.days(start, end)
///     180
///     >>> thirty360.year_fraction(start, end)
///     0.5
///     
///     # Compare different conventions
///     >>> act365f = DayCount.act365f()
///     >>> act365f.year_fraction(start, end)
///     0.4958904109589041
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

    /// Create an ACT/360 day count convention.
    ///
    /// Uses actual days in the numerator and 360 days in the denominator.
    /// Common in money markets and floating rate notes.
    ///
    /// Returns:
    ///     DayCount: An ACT/360 day count convention instance.
    ///
    /// Examples:
    ///     >>> from rfin.dates import DayCount
    ///     >>> dc = DayCount.act360()
    ///     >>> str(dc)
    ///     'ACT/360'
    #[classmethod]
    fn act360(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: CoreDayCount::Act360,
        }
    }

    /// Create an ACT/365F day count convention.
    ///
    /// Uses actual days in the numerator and 365 days in the denominator.
    /// The "F" stands for "Fixed" - always uses 365 days regardless of leap years.
    /// Common in sterling markets and some government bonds.
    ///
    /// Returns:
    ///     DayCount: An ACT/365F day count convention instance.
    ///
    /// Examples:
    ///     >>> from rfin.dates import DayCount
    ///     >>> dc = DayCount.act365f()
    ///     >>> str(dc)
    ///     'ACT/365F'
    #[classmethod]
    fn act365f(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: CoreDayCount::Act365F,
        }
    }

    /// Create a 30/360 day count convention.
    ///
    /// Assumes 30 days in each month and 360 days in a year.
    /// Common in corporate bonds and swaps. Also known as "30/360 Bond Basis".
    ///
    /// Returns:
    ///     DayCount: A 30/360 day count convention instance.
    ///
    /// Examples:
    ///     >>> from rfin.dates import DayCount
    ///     >>> dc = DayCount.thirty360()
    ///     >>> str(dc)
    ///     '30/360'
    #[classmethod]
    fn thirty360(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: CoreDayCount::Thirty360,
        }
    }

    /// Create a 30E/360 day count convention.
    ///
    /// European variant of 30/360. Similar to 30/360 but with slightly
    /// different rules for handling month-end dates.
    ///
    /// Returns:
    ///     DayCount: A 30E/360 day count convention instance.
    ///
    /// Examples:
    ///     >>> from rfin.dates import DayCount
    ///     >>> dc = DayCount.thirty_e_360()
    ///     >>> str(dc)
    ///     '30E/360'
    #[classmethod]
    fn thirty_e_360(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: CoreDayCount::ThirtyE360,
        }
    }

    /// Create an ACT/ACT day count convention.
    ///
    /// Uses actual days in both numerator and denominator, accounting for
    /// leap years. Common in government bonds and some swaps.
    ///
    /// Returns:
    ///     DayCount: An ACT/ACT day count convention instance.
    ///
    /// Examples:
    ///     >>> from rfin.dates import DayCount
    ///     >>> dc = DayCount.actact()
    ///     >>> str(dc)
    ///     'ACT/ACT'
    #[classmethod]
    fn actact(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: CoreDayCount::ActAct,
        }
    }

    // ---------------------------------------------------------------------
    // Methods
    // ---------------------------------------------------------------------

    /// Calculate the number of days between two dates.
    ///
    /// The day count may differ from the actual calendar days depending
    /// on the convention (e.g., 30/360 assumes 30 days per month).
    ///
    /// Args:
    ///     start (Date): The start date (inclusive).
    ///     end (Date): The end date (exclusive).
    ///
    /// Returns:
    ///     int: The number of days according to the convention.
    ///
    /// Raises:
    ///     RuntimeError: If the calculation fails.
    ///
    /// Examples:
    ///     >>> from rfin.dates import Date, DayCount
    ///     >>> start = Date(2023, 1, 1)
    ///     >>> end = Date(2023, 2, 1)
    ///     
    ///     # ACT/360 uses actual days
    ///     >>> DayCount.act360().days(start, end)
    ///     31
    ///     
    ///     # 30/360 assumes 30 days per month
    ///     >>> DayCount.thirty360().days(start, end)
    ///     30
    ///     
    ///     # February has 28 days in 2023
    ///     >>> feb_start = Date(2023, 2, 1)
    ///     >>> feb_end = Date(2023, 3, 1)
    ///     >>> DayCount.act360().days(feb_start, feb_end)
    ///     28
    ///     >>> DayCount.thirty360().days(feb_start, feb_end)
    ///     30
    #[pyo3(text_signature = "(self, start, end)")]
    pub fn days(&self, start: &PyDate, end: &PyDate) -> PyResult<i32> {
        self.inner
            .days(start.inner(), end.inner())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Calculate the year fraction between two dates.
    ///
    /// The year fraction is used to calculate accrued interest:
    /// `interest = principal * rate * year_fraction`
    ///
    /// Args:
    ///     start (Date): The start date (inclusive).
    ///     end (Date): The end date (exclusive).
    ///
    /// Returns:
    ///     float: The year fraction according to the convention.
    ///
    /// Raises:
    ///     RuntimeError: If the calculation fails.
    ///
    /// Examples:
    ///     >>> from rfin.dates import Date, DayCount
    ///     >>> start = Date(2023, 1, 1)
    ///     >>> end = Date(2023, 7, 1)  # 6 months later
    ///     
    ///     # ACT/360: 181 actual days / 360
    ///     >>> DayCount.act360().year_fraction(start, end)
    ///     0.5027777777777778
    ///     
    ///     # 30/360: 180 days / 360 = 0.5
    ///     >>> DayCount.thirty360().year_fraction(start, end)
    ///     0.5
    ///     
    ///     # ACT/365F: 181 actual days / 365
    ///     >>> DayCount.act365f().year_fraction(start, end)
    ///     0.4958904109589041
    ///     
    ///     # Calculate interest for 6 months at 5% on $100,000
    ///     >>> principal = 100000
    ///     >>> rate = 0.05
    ///     >>> yf = DayCount.act360().year_fraction(start, end)
    ///     >>> interest = principal * rate * yf
    ///     >>> interest
    ///     2513.888888888889
    #[pyo3(text_signature = "(self, start, end)")]
    pub fn year_fraction(&self, start: &PyDate, end: &PyDate) -> PyResult<f64> {
        self.inner
            .year_fraction(start.inner(), end.inner())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    // ---------------------------------------------------------------------
    // Dunder / repr helpers
    // ---------------------------------------------------------------------

    /// Return string representation of the day count convention.
    ///
    /// Returns:
    ///     str: A string like "DayCount('ACT/360')".
    ///
    /// Examples:
    ///     >>> from rfin.dates import DayCount
    ///     >>> dc = DayCount.act360()
    ///     >>> repr(dc)
    ///     "DayCount('ACT/360')"
    fn __repr__(&self) -> String {
        format!("DayCount('{}')", self.__str__())
    }

    /// Return the name of the day count convention.
    ///
    /// Returns:
    ///     str: The conventional name of the day count convention.
    ///
    /// Examples:
    ///     >>> from rfin.dates import DayCount
    ///     >>> str(DayCount.act360())
    ///     'ACT/360'
    ///     >>> str(DayCount.thirty360())
    ///     '30/360'
    ///     >>> str(DayCount.act365f())
    ///     'ACT/365F'
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

    /// Check equality between two day count conventions.
    ///
    /// Args:
    ///     other (DayCount): Another DayCount instance.
    ///
    /// Returns:
    ///     bool: True if the conventions are the same.
    ///
    /// Examples:
    ///     >>> from rfin.dates import DayCount
    ///     >>> dc1 = DayCount.act360()
    ///     >>> dc2 = DayCount.act360()
    ///     >>> dc3 = DayCount.thirty360()
    ///     >>> dc1 == dc2
    ///     True
    ///     >>> dc1 == dc3
    ///     False
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
