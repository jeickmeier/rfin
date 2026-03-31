//! Python bindings for lookback period selectors.
//!
//! Wraps `finstack_analytics::lookback` functions. Each function takes a list
//! of `datetime.date` objects and a reference date, returning a ``(start, end)``
//! tuple of indices into the date array.

use crate::core::dates::utils::py_to_date;
use finstack_analytics::lookback;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Convert a list of Python dates to Rust `Date` values.
fn py_dates_to_vec(dates: &Bound<'_, PyAny>) -> PyResult<Vec<finstack_core::dates::Date>> {
    let list: Vec<Bound<'_, PyAny>> = dates.extract()?;
    list.iter().map(|d| py_to_date(d)).collect()
}

/// Month-to-date index range.
///
/// Parameters
/// ----------
/// dates : list[datetime.date]
///     Sorted observation dates.
/// ref_date : datetime.date
///     Reference date (typically today).
/// offset_days : int
///     Days to shift the window start backward.
///
/// Returns
/// -------
/// tuple[int, int]
///     ``(start, end)`` indices into ``dates``.
#[pyfunction]
#[pyo3(signature = (dates, ref_date, offset_days=0))]
fn mtd_select(
    dates: &Bound<'_, PyAny>,
    ref_date: &Bound<'_, PyAny>,
    offset_days: i64,
) -> PyResult<(usize, usize)> {
    let d = py_dates_to_vec(dates)?;
    let rd = py_to_date(ref_date)?;
    let range = lookback::mtd_select(&d, rd, offset_days);
    Ok((range.start, range.end))
}

/// Quarter-to-date index range.
///
/// Parameters
/// ----------
/// dates : list[datetime.date]
///     Sorted observation dates.
/// ref_date : datetime.date
///     Reference date.
/// offset_days : int
///     Days to shift the window start backward.
///
/// Returns
/// -------
/// tuple[int, int]
///     ``(start, end)`` indices into ``dates``.
#[pyfunction]
#[pyo3(signature = (dates, ref_date, offset_days=0))]
fn qtd_select(
    dates: &Bound<'_, PyAny>,
    ref_date: &Bound<'_, PyAny>,
    offset_days: i64,
) -> PyResult<(usize, usize)> {
    let d = py_dates_to_vec(dates)?;
    let rd = py_to_date(ref_date)?;
    let range = lookback::qtd_select(&d, rd, offset_days);
    Ok((range.start, range.end))
}

/// Year-to-date index range.
///
/// Parameters
/// ----------
/// dates : list[datetime.date]
///     Sorted observation dates.
/// ref_date : datetime.date
///     Reference date.
/// offset_days : int
///     Days to shift the window start backward.
///
/// Returns
/// -------
/// tuple[int, int]
///     ``(start, end)`` indices into ``dates``.
#[pyfunction]
#[pyo3(signature = (dates, ref_date, offset_days=0))]
fn ytd_select(
    dates: &Bound<'_, PyAny>,
    ref_date: &Bound<'_, PyAny>,
    offset_days: i64,
) -> PyResult<(usize, usize)> {
    let d = py_dates_to_vec(dates)?;
    let rd = py_to_date(ref_date)?;
    let range = lookback::ytd_select(&d, rd, offset_days);
    Ok((range.start, range.end))
}

/// Fiscal-year-to-date index range.
///
/// Parameters
/// ----------
/// dates : list[datetime.date]
///     Sorted observation dates.
/// ref_date : datetime.date
///     Reference date.
/// fiscal_start_month : int
///     Month (1-12) when the fiscal year starts.
/// fiscal_start_day : int
///     Day of month when the fiscal year starts.
/// offset_days : int
///     Days to shift the window start backward.
///
/// Returns
/// -------
/// tuple[int, int]
///     ``(start, end)`` indices into ``dates``.
#[pyfunction]
#[pyo3(signature = (dates, ref_date, fiscal_start_month=10, fiscal_start_day=1, offset_days=0))]
fn fytd_select(
    dates: &Bound<'_, PyAny>,
    ref_date: &Bound<'_, PyAny>,
    fiscal_start_month: u8,
    fiscal_start_day: u8,
    offset_days: i64,
) -> PyResult<(usize, usize)> {
    let d = py_dates_to_vec(dates)?;
    let rd = py_to_date(ref_date)?;
    let config = finstack_core::dates::FiscalConfig::new(fiscal_start_month, fiscal_start_day)
        .map_err(|e| PyValueError::new_err(format!("Invalid fiscal config: {e}")))?;
    let range = lookback::fytd_select(&d, rd, config, offset_days);
    Ok((range.start, range.end))
}

/// Register standalone lookback functions and return export names.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    m.add_function(wrap_pyfunction!(mtd_select, m)?)?;
    m.add_function(wrap_pyfunction!(qtd_select, m)?)?;
    m.add_function(wrap_pyfunction!(ytd_select, m)?)?;
    m.add_function(wrap_pyfunction!(fytd_select, m)?)?;
    Ok(vec![
        "mtd_select",
        "qtd_select",
        "ytd_select",
        "fytd_select",
    ])
}
