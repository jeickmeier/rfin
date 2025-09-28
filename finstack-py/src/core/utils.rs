//! Utilities for Python↔Rust date conversions.
//!
//! These helpers convert between Python's `datetime.date`/`datetime.datetime`
//! and Rust's `time::Date` for use across the bindings. Errors are surfaced as
//! Python exceptions (`TypeError` for type mismatches, `ValueError` for invalid
//! calendar components) to keep behavior predictable for Python callers.
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyDate, PyDateAccess, PyDateTime};
use pyo3::Bound;
use time::{Date, Month};

/// Convert a Python `datetime.date` or `datetime.datetime` into `time::Date`.
///
/// Parameters
/// ----------
/// value : datetime.date or datetime.datetime
///     Python date/time object to convert.
///
/// Returns
/// -------
/// time::Date
///     Rust date representing the same calendar day.
///
/// Raises
/// ------
/// TypeError
///     If `value` is not a `datetime.date` or `datetime.datetime`.
pub(crate) fn py_to_date(value: &Bound<'_, PyAny>) -> PyResult<Date> {
    if let Ok(date) = value.downcast::<PyDate>() {
        return build_date(
            date.get_year(),
            date.get_month() as u8,
            date.get_day() as u8,
        );
    }

    if let Ok(dt) = value.downcast::<PyDateTime>() {
        return build_date(dt.get_year(), dt.get_month() as u8, dt.get_day() as u8);
    }

    Err(PyTypeError::new_err(
        "Expected datetime.date or datetime.datetime",
    ))
}

/// Convert a `time::Date` into a Python `datetime.date` object.
///
/// Parameters
/// ----------
/// py : Python
///     Python interpreter token.
/// date : time::Date
///     Rust date to convert.
///
/// Returns
/// -------
/// datetime.date
///     Python date instance matching `date`.
///
/// Raises
/// ------
/// ValueError
///     If the resulting Python date cannot be constructed.
pub(crate) fn date_to_py(py: Python<'_>, date: Date) -> PyResult<PyObject> {
    PyDate::new(py, date.year(), u8::from(date.month()), date.day())
        .map(|obj| obj.into())
        .map_err(|err| PyValueError::new_err(err.to_string()))
}

/// Build a `time::Date` from (year, month, day) components.
///
/// Parameters
/// ----------
/// year : int
///     Four-digit calendar year.
/// month : int
///     Month number in the range 1..=12.
/// day : int
///     Day-of-month.
///
/// Returns
/// -------
/// time::Date
///     Constructed date.
///
/// Raises
/// ------
/// ValueError
///     If the components do not form a valid calendar date.
fn build_date(year: i32, month: u8, day: u8) -> PyResult<Date> {
    let month = Month::try_from(month)
        .map_err(|_| PyValueError::new_err(format!("Month out of range: {month}")))?;
    Date::from_calendar_date(year, month, day).map_err(|err| PyValueError::new_err(err.to_string()))
}
