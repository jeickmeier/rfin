use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::errors::PyContext;
use finstack_core::dates::{
    imm_option_expiry, next_cds_date, next_equity_option_expiry, next_imm, next_imm_option_expiry,
    third_friday, third_wednesday,
};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;
use time::Month;

/// Next financial IMM date after the provided date.
///
/// Parameters
/// ----------
/// date : datetime.date
///     Reference date.
///
/// Returns
/// -------
/// datetime.date
///     The next IMM date.
#[pyfunction(name = "next_imm", text_signature = "(date)")]
fn next_imm_py(py: Python<'_>, date: Bound<'_, PyAny>) -> PyResult<PyObject> {
    let d = py_to_date(&date).context("date")?;
    date_to_py(py, next_imm(d))
}

/// Next CDS IMM date (quarterly on the 20th) after the provided date.
///
/// Parameters
/// ----------
/// date : datetime.date
///     Reference date.
///
/// Returns
/// -------
/// datetime.date
///     Next CDS IMM date.
#[pyfunction(name = "next_cds_date", text_signature = "(date)")]
fn next_cds_date_py(py: Python<'_>, date: Bound<'_, PyAny>) -> PyResult<PyObject> {
    let d = py_to_date(&date).context("date")?;
    date_to_py(py, next_cds_date(d))
}

/// Next financial IMM option expiry after the provided date.
///
/// Parameters
/// ----------
/// date : datetime.date
///     Reference date.
///
/// Returns
/// -------
/// datetime.date
///     Next IMM option expiry date.
#[pyfunction(name = "next_imm_option_expiry", text_signature = "(date)")]
fn next_imm_option_expiry_py(py: Python<'_>, date: Bound<'_, PyAny>) -> PyResult<PyObject> {
    let d = py_to_date(&date).context("date")?;
    date_to_py(py, next_imm_option_expiry(d))
}

/// IMM option expiry date for a specific year and month.
///
/// Parameters
/// ----------
/// year : int
///     Calendar year.
/// month : int
///     Month (1-12).
///
/// Returns
/// -------
/// datetime.date
///     IMM option expiry date for the month.
#[pyfunction(name = "imm_option_expiry", text_signature = "(year, month)")]
fn imm_option_expiry_py(py: Python<'_>, year: i32, month: u8) -> PyResult<PyObject> {
    let month_enum = Month::try_from(month).map_err(|_| {
        pyo3::exceptions::PyValueError::new_err(format!("Month out of range: {month}"))
    })?;
    let date = imm_option_expiry(month_enum, year);
    date_to_py(py, date)
}

/// Next monthly equity option expiry (third Friday) after the provided date.
///
/// Parameters
/// ----------
/// date : datetime.date
///     Reference date.
///
/// Returns
/// -------
/// datetime.date
///     Next monthly equity option expiry.
#[pyfunction(name = "next_equity_option_expiry", text_signature = "(date)")]
fn next_equity_option_expiry_py(py: Python<'_>, date: Bound<'_, PyAny>) -> PyResult<PyObject> {
    let d = py_to_date(&date).context("date")?;
    date_to_py(py, next_equity_option_expiry(d))
}

/// Third Friday of the specified month/year.
///
/// Parameters
/// ----------
/// year : int
///     Calendar year.
/// month : int
///     Month (1-12).
///
/// Returns
/// -------
/// datetime.date
///     Third Friday date.
#[pyfunction(name = "third_friday", text_signature = "(year, month)")]
fn third_friday_py(py: Python<'_>, year: i32, month: u8) -> PyResult<PyObject> {
    let month_enum = Month::try_from(month).map_err(|_| {
        pyo3::exceptions::PyValueError::new_err(format!("Month out of range: {month}"))
    })?;
    date_to_py(py, third_friday(month_enum, year))
}

/// Third Wednesday of the specified month/year.
///
/// Parameters
/// ----------
/// year : int
///     Calendar year.
/// month : int
///     Month (1-12).
///
/// Returns
/// -------
/// datetime.date
///     Third Wednesday date.
#[pyfunction(name = "third_wednesday", text_signature = "(year, month)")]
fn third_wednesday_py(py: Python<'_>, year: i32, month: u8) -> PyResult<PyObject> {
    let month_enum = Month::try_from(month).map_err(|_| {
        pyo3::exceptions::PyValueError::new_err(format!("Month out of range: {month}"))
    })?;
    date_to_py(py, third_wednesday(month_enum, year))
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "imm")?;
    module.setattr(
        "__doc__",
        "IMM and option expiry helpers from finstack_core::dates::imm.",
    )?;
    module.add_function(wrap_pyfunction!(next_imm_py, &module)?)?;
    module.add_function(wrap_pyfunction!(next_cds_date_py, &module)?)?;
    module.add_function(wrap_pyfunction!(next_imm_option_expiry_py, &module)?)?;
    module.add_function(wrap_pyfunction!(imm_option_expiry_py, &module)?)?;
    module.add_function(wrap_pyfunction!(next_equity_option_expiry_py, &module)?)?;
    module.add_function(wrap_pyfunction!(third_friday_py, &module)?)?;
    module.add_function(wrap_pyfunction!(third_wednesday_py, &module)?)?;
    let exports = [
        "next_imm",
        "next_cds_date",
        "next_imm_option_expiry",
        "imm_option_expiry",
        "next_equity_option_expiry",
        "third_friday",
        "third_wednesday",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
