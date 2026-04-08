//! Python bindings for FX date utilities (joint calendar adjustments and spot rolls).
//!
//! Wraps [`finstack_core::dates::fx`] functions for Python, providing joint-calendar
//! business-day adjustments, joint business-day counting, and spot-date rolling.

use super::utils::{date_to_py, py_to_date};
use crate::core::common::args::BusinessDayConventionArg;
use crate::errors::{core_to_py, PyContext};
use finstack_core::dates::BusinessDayConvention;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

#[pyfunction(name = "adjust_joint_calendar")]
#[pyo3(signature = (date, bdc, base_cal_id = None, quote_cal_id = None))]
/// Adjust a date so it is a business day on both base and quote calendars.
///
/// Applies the business-day convention on the joint (union) calendar where a
/// day is a holiday if either currency market is closed.
///
/// Parameters
/// ----------
/// date : datetime.date
///     Date to adjust.
/// bdc : BusinessDayConvention or str
///     Business-day convention (e.g. ``"following"``, ``"modified_following"``).
/// base_cal_id : str or None, optional
///     Calendar ID for the base currency (default weekends-only).
/// quote_cal_id : str or None, optional
///     Calendar ID for the quote currency (default weekends-only).
///
/// Returns
/// -------
/// datetime.date
///     Adjusted date that is a business day on both calendars.
///
/// Raises
/// ------
/// ValueError
///     If a calendar ID is not recognized or date adjustment fails.
fn py_adjust_joint_calendar(
    py: Python<'_>,
    date: Bound<'_, PyAny>,
    bdc: BusinessDayConventionArg,
    base_cal_id: Option<&str>,
    quote_cal_id: Option<&str>,
) -> PyResult<Py<PyAny>> {
    let d = py_to_date(&date).context("date")?;
    let result =
        finstack_core::dates::fx::adjust_joint_calendar(d, bdc.0, base_cal_id, quote_cal_id)
            .map_err(core_to_py)?;
    date_to_py(py, result)
}

#[pyfunction(name = "add_joint_business_days")]
#[pyo3(signature = (start, n_days, bdc, base_cal_id = None, quote_cal_id = None))]
/// Add N business days on a joint (two-currency) calendar.
///
/// A day is counted as a business day only if it is a business day on **both**
/// the base and quote calendars.
///
/// Parameters
/// ----------
/// start : datetime.date
///     Starting date.
/// n_days : int
///     Number of joint business days to add.
/// bdc : BusinessDayConvention or str
///     Business-day convention (kept for API consistency).
/// base_cal_id : str or None, optional
///     Calendar ID for the base currency (default weekends-only).
/// quote_cal_id : str or None, optional
///     Calendar ID for the quote currency (default weekends-only).
///
/// Returns
/// -------
/// datetime.date
///     Date that is ``n_days`` joint business days after ``start``.
///
/// Raises
/// ------
/// ValueError
///     If a calendar ID is not recognized or iteration limit is exceeded.
fn py_add_joint_business_days(
    py: Python<'_>,
    start: Bound<'_, PyAny>,
    n_days: u32,
    bdc: BusinessDayConventionArg,
    base_cal_id: Option<&str>,
    quote_cal_id: Option<&str>,
) -> PyResult<Py<PyAny>> {
    let d = py_to_date(&start).context("start")?;
    let result = finstack_core::dates::fx::add_joint_business_days(
        d,
        n_days,
        bdc.0,
        base_cal_id,
        quote_cal_id,
    )
    .map_err(core_to_py)?;
    date_to_py(py, result)
}

#[pyfunction(name = "roll_spot_date")]
#[pyo3(signature = (trade_date, spot_lag_days, base_cal_id = None, quote_cal_id = None, settlement_bdc = None))]
/// Roll a trade date to a spot settlement date using joint business-day counting.
///
/// Parameters
/// ----------
/// trade_date : datetime.date
///     Trade execution date.
/// spot_lag_days : int
///     Number of business days to spot (typically 2 for most FX pairs).
/// base_cal_id : str or None, optional
///     Calendar ID for the base currency (default weekends-only).
/// quote_cal_id : str or None, optional
///     Calendar ID for the quote currency (default weekends-only).
/// settlement_bdc : BusinessDayConvention or str or None, optional
///     Business-day convention for settlement (default ``Following``).
///
/// Returns
/// -------
/// datetime.date
///     Spot settlement date.
///
/// Raises
/// ------
/// ValueError
///     If a calendar ID is not recognized or date arithmetic fails.
fn py_roll_spot_date(
    py: Python<'_>,
    trade_date: Bound<'_, PyAny>,
    spot_lag_days: u32,
    base_cal_id: Option<&str>,
    quote_cal_id: Option<&str>,
    settlement_bdc: Option<BusinessDayConventionArg>,
) -> PyResult<Py<PyAny>> {
    let d = py_to_date(&trade_date).context("trade_date")?;
    let bdc = settlement_bdc
        .map(|arg| arg.0)
        .unwrap_or(BusinessDayConvention::Following);
    let result =
        finstack_core::dates::fx::roll_spot_date(d, spot_lag_days, bdc, base_cal_id, quote_cal_id)
            .map_err(core_to_py)?;
    date_to_py(py, result)
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "fx")?;
    module.setattr(
        "__doc__",
        "FX date utilities for joint calendar adjustments and spot rolls.\n\n\
         Functions:\n\
         - adjust_joint_calendar: Adjust a date on a joint (two-currency) calendar\n\
         - add_joint_business_days: Add N business days on a joint calendar\n\
         - roll_spot_date: Roll a trade date to spot using joint business-day counting",
    )?;

    module.add_function(wrap_pyfunction!(py_adjust_joint_calendar, &module)?)?;
    module.add_function(wrap_pyfunction!(py_add_joint_business_days, &module)?)?;
    module.add_function(wrap_pyfunction!(py_roll_spot_date, &module)?)?;

    let exports = [
        "add_joint_business_days",
        "adjust_joint_calendar",
        "roll_spot_date",
    ];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
