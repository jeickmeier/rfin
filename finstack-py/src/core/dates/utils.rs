//! Date utilities: month math, month boundaries, leap years, and epoch conversions.
//!
//! These helpers mirror `finstack_core::dates::utils` and selected methods from
//! `finstack_core::dates::DateExt`, providing Python-friendly functions for common
//! calendar operations. All functions accept/return `datetime.date` where applicable
//! and surface `ValueError` for invalid inputs.
use super::calendar::extract_calendar;
use super::periods::PyFiscalConfig;
use crate::errors::{core_to_py, PyContext};
use finstack_core::dates::DateExt;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAnyMethods, PyDate, PyDateAccess, PyDateTime, PyList, PyModule};
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
    if let Ok(date) = value.cast::<PyDate>() {
        return build_date(date.get_year(), date.get_month(), date.get_day());
    }

    if let Ok(dt) = value.cast::<PyDateTime>() {
        return build_date(dt.get_year(), dt.get_month(), dt.get_day());
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
pub(crate) fn date_to_py(py: Python<'_>, date: Date) -> PyResult<Py<PyAny>> {
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

/// Add a number of calendar months to a date (preserving end-of-month logic).
///
/// Parameters
/// ----------
/// date : datetime.date
///     Anchor date.
/// months : int
///     Number of calendar months to add (negative to subtract).
///
/// Returns
/// -------
/// datetime.date
///     Resulting date after applying month arithmetic with EOM handling.
///
/// Examples
/// --------
/// If `date` is January 31st and `months=1`, the result is February's last day.
#[pyfunction(name = "add_months", text_signature = "(date, months)")]
fn add_months_py(py: Python<'_>, date: Bound<'_, PyAny>, months: i32) -> PyResult<Py<PyAny>> {
    let d = py_to_date(&date)?;
    let result = d.add_months(months);
    date_to_py(py, result)
}

/// Last calendar day in the month of the provided date.
///
/// Parameters
/// ----------
/// date : datetime.date
///     Any date within the target month.
///
/// Returns
/// -------
/// datetime.date
///     Month-end date for `date`'s month.
#[pyfunction(name = "last_day_of_month", text_signature = "(date)")]
fn last_day_of_month_py(py: Python<'_>, date: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let d = py_to_date(&date)?;
    let result = d.end_of_month();
    date_to_py(py, result)
}

/// True if the provided date falls on a weekend (Saturday or Sunday).
///
/// Parameters
/// ----------
/// date : datetime.date
///     Date to test.
///
/// Returns
/// -------
/// bool
///     ``True`` when the date is Saturday/Sunday, otherwise ``False``.
#[pyfunction(name = "is_weekend", text_signature = "(date)")]
fn is_weekend_py(date: Bound<'_, PyAny>) -> PyResult<bool> {
    let d = py_to_date(&date)?;
    Ok(d.is_weekend())
}

/// Calendar quarter of the provided date (1..=4).
///
/// Parameters
/// ----------
/// date : datetime.date
///     Date to classify.
///
/// Returns
/// -------
/// int
///     Quarter index in the range 1..=4.
#[pyfunction(name = "quarter", text_signature = "(date)")]
fn quarter_py(date: Bound<'_, PyAny>) -> PyResult<u8> {
    let d = py_to_date(&date)?;
    Ok(d.quarter())
}

#[pyfunction(name = "months_until", text_signature = "(start, end)")]
fn months_until_py(start: Bound<'_, PyAny>, end: Bound<'_, PyAny>) -> PyResult<u32> {
    let start_date = py_to_date(&start)?;
    let end_date = py_to_date(&end)?;
    Ok(start_date.months_until(end_date))
}

/// Fiscal year corresponding to the date under a given fiscal configuration.
///
/// Parameters
/// ----------
/// date : datetime.date
///     Date to classify.
/// config : FiscalConfig
///     Fiscal-year configuration such as ``FiscalConfig.US_FEDERAL``.
///
/// Returns
/// -------
/// int
///     Fiscal year identifier.
#[pyfunction(name = "fiscal_year", text_signature = "(date, config)")]
fn fiscal_year_py(date: Bound<'_, PyAny>, config: PyRef<PyFiscalConfig>) -> PyResult<i32> {
    let d = py_to_date(&date)?;
    Ok(d.fiscal_year(config.inner))
}

/// Add / subtract a number of weekdays (Mon–Fri) from the provided date.
///
/// Weekends are skipped but **holidays are ignored**; for true business-day
/// arithmetic, use :func:`add_business_days`.
///
/// Parameters
/// ----------
/// date : datetime.date
///     Anchor date.
/// n : int
///     Number of weekdays to add (negative to subtract).
///
/// Returns
/// -------
/// datetime.date
///     Adjusted date after skipping weekends.
#[pyfunction(name = "add_weekdays", text_signature = "(date, n)")]
fn add_weekdays_py(py: Python<'_>, date: Bound<'_, PyAny>, n: i32) -> PyResult<Py<PyAny>> {
    let d = py_to_date(&date)?;
    let result = d.add_weekdays(n);
    date_to_py(py, result)
}

/// Add / subtract a number of business days from the provided date.
///
/// Weekends and holidays as defined by the supplied calendar are skipped.
///
/// Parameters
/// ----------
/// date : datetime.date
///     Anchor date.
/// n : int
///     Number of business days to add (negative permitted).
/// calendar : Calendar or str
///     Holiday calendar or calendar code.
///
/// Returns
/// -------
/// datetime.date
///     Adjusted business-day date.
#[pyfunction(name = "add_business_days", text_signature = "(date, n, calendar)")]
fn add_business_days_py(
    py: Python<'_>,
    date: Bound<'_, PyAny>,
    n: i32,
    calendar: Bound<'_, PyAny>,
) -> PyResult<Py<PyAny>> {
    let d = py_to_date(&date).context("date")?;
    let cal = extract_calendar(&calendar).context("calendar")?;
    let result = d.add_business_days(n, cal.inner).map_err(core_to_py)?;
    date_to_py(py, result)
}

/// Return ``True`` if the date is a business day under the supplied calendar.
///
/// Parameters
/// ----------
/// date : datetime.date
///     Date to test.
/// calendar : Calendar or str
///     Holiday calendar or calendar code.
///
/// Returns
/// -------
/// bool
///     ``True`` if the date is a business day, otherwise ``False``.
#[pyfunction(name = "is_business_day", text_signature = "(date, calendar)")]
fn is_business_day_py(date: Bound<'_, PyAny>, calendar: Bound<'_, PyAny>) -> PyResult<bool> {
    let d = py_to_date(&date).context("date")?;
    let cal = extract_calendar(&calendar).context("calendar")?;
    Ok(cal.inner.is_business_day(d))
}

/// Number of days in a given month of a year.
///
/// Parameters
/// ----------
/// year : int
///     Calendar year.
/// month : int
///     Month number (1-12).
///
/// Returns
/// -------
/// int
///     Number of days in the month.
///
/// Raises
/// ------
/// ValueError
///     If `month` is not in 1..=12.
#[pyfunction(name = "days_in_month", text_signature = "(year, month)")]
fn days_in_month_py(_year: i32, month: u8) -> PyResult<u8> {
    if !(1..=12).contains(&month) {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Month out of range: {month}"
        )));
    }
    let m = Month::try_from(month).map_err(|e| {
        pyo3::exceptions::PyValueError::new_err(format!("Invalid month {month}: {e}"))
    })?;
    Ok(m.length(_year))
}

/// True if the given year is a leap year.
///
/// Parameters
/// ----------
/// year : int
///     Calendar year.
///
/// Returns
/// -------
/// bool
///     `True` if `year` is a leap year, otherwise `False`.
#[pyfunction(name = "is_leap_year", text_signature = "(year)")]
fn is_leap_year_py(year: i32) -> bool {
    time::util::is_leap_year(year)
}

/// Convert a date into a day count offset from the Unix epoch (1970-01-01).
///
/// Parameters
/// ----------
/// date : datetime.date
///     Date to convert to an epoch day offset.
///
/// Returns
/// -------
/// int
///     Days since 1970-01-01 (negative for dates before the epoch).
#[pyfunction(name = "date_to_days_since_epoch", text_signature = "(date)")]
fn date_to_days_since_epoch_py(date: Bound<'_, PyAny>) -> PyResult<i32> {
    let d = py_to_date(&date)?;
    Ok(finstack_core::dates::days_since_epoch(d))
}

/// Convert a day-count offset from the Unix epoch back to a date.
///
/// Parameters
/// ----------
/// days : int
///     Days since 1970-01-01 (negative allowed).
///
/// Returns
/// -------
/// datetime.date
///     Date corresponding to the epoch offset.
#[pyfunction(name = "days_since_epoch_to_date", text_signature = "(days)")]
fn days_since_epoch_to_date_py(py: Python<'_>, days: i32) -> PyResult<Py<PyAny>> {
    finstack_core::dates::date_from_epoch_days(days)
        .ok_or_else(|| PyValueError::new_err("days out of range"))
        .and_then(|d| date_to_py(py, d))
}

/// Safe helper to construct a :class:`datetime.date` with validation.
///
/// Parameters
/// ----------
/// year : int
///     Calendar year.
/// month : int
///     Month (1-12).
/// day : int
///     Day-of-month.
///
/// Returns
/// -------
/// datetime.date
///     Constructed date.
///
/// Raises
/// ------
/// ValueError
///     If the combination does not form a valid calendar date.
#[pyfunction(name = "create_date", text_signature = "(year, month, day)")]
fn create_date_py(py: Python<'_>, year: i32, month: u8, day: u8) -> PyResult<Py<PyAny>> {
    if !(1..=12).contains(&month) {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Month out of range: {month}"
        )));
    }
    let month_enum = Month::try_from(month).map_err(|e| {
        pyo3::exceptions::PyValueError::new_err(format!("Invalid month {month}: {e}"))
    })?;
    let date = finstack_core::dates::create_date(year, month_enum, day).map_err(core_to_py)?;
    date_to_py(py, date)
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "utils")?;
    module.setattr(
        "__doc__",
        "Date utility helpers mirroring finstack_core::dates::utils.",
    )?;
    module.add_function(wrap_pyfunction!(add_months_py, &module)?)?;
    module.add_function(wrap_pyfunction!(last_day_of_month_py, &module)?)?;
    module.add_function(wrap_pyfunction!(is_weekend_py, &module)?)?;
    module.add_function(wrap_pyfunction!(quarter_py, &module)?)?;
    module.add_function(wrap_pyfunction!(months_until_py, &module)?)?;
    module.add_function(wrap_pyfunction!(fiscal_year_py, &module)?)?;
    module.add_function(wrap_pyfunction!(add_weekdays_py, &module)?)?;
    module.add_function(wrap_pyfunction!(add_business_days_py, &module)?)?;
    module.add_function(wrap_pyfunction!(is_business_day_py, &module)?)?;
    module.add_function(wrap_pyfunction!(days_in_month_py, &module)?)?;
    module.add_function(wrap_pyfunction!(is_leap_year_py, &module)?)?;
    module.add_function(wrap_pyfunction!(date_to_days_since_epoch_py, &module)?)?;
    module.add_function(wrap_pyfunction!(days_since_epoch_to_date_py, &module)?)?;
    module.add_function(wrap_pyfunction!(create_date_py, &module)?)?;
    let exports = [
        "add_business_days",
        "add_months",
        "add_weekdays",
        "create_date",
        "date_to_days_since_epoch",
        "days_in_month",
        "days_since_epoch_to_date",
        "fiscal_year",
        "is_business_day",
        "is_leap_year",
        "is_weekend",
        "last_day_of_month",
        "months_until",
        "quarter",
    ];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
