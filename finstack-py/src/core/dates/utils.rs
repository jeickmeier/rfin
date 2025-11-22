//! Date utilities: month math, month boundaries, leap years, and epoch conversions.
//!
//! These helpers mirror `finstack_core::dates::utils` and provide Python-friendly
//! functions for common calendar operations. All functions accept/return
//! `datetime.date` where applicable and surface `ValueError` for invalid inputs.
use crate::core::utils::{date_to_py, py_to_date};
use finstack_core::dates::DateExt;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;
use time::{Date, Duration, Month};

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
fn add_months_py(py: Python<'_>, date: Bound<'_, PyAny>, months: i32) -> PyResult<PyObject> {
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
fn last_day_of_month_py(py: Python<'_>, date: Bound<'_, PyAny>) -> PyResult<PyObject> {
    let d = py_to_date(&date)?;
    let result = d.end_of_month();
    date_to_py(py, result)
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
    let m = Month::try_from(month).expect("Month validated in range");
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
    let epoch = Date::from_calendar_date(1970, Month::January, 1).expect("Epoch valid");
    Ok((d - epoch).whole_days() as i32)
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
fn days_since_epoch_to_date_py(py: Python<'_>, days: i32) -> PyResult<PyObject> {
    let epoch = Date::from_calendar_date(1970, Month::January, 1).expect("Epoch valid");
    let date = epoch + Duration::days(days as i64);
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
    module.add_function(wrap_pyfunction!(days_in_month_py, &module)?)?;
    module.add_function(wrap_pyfunction!(is_leap_year_py, &module)?)?;
    module.add_function(wrap_pyfunction!(date_to_days_since_epoch_py, &module)?)?;
    module.add_function(wrap_pyfunction!(days_since_epoch_to_date_py, &module)?)?;
    let exports = [
        "add_months",
        "last_day_of_month",
        "days_in_month",
        "is_leap_year",
        "date_to_days_since_epoch",
        "days_since_epoch_to_date",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
