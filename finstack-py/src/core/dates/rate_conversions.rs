//! Rate conversion utilities.
//!
//! Provides functions to convert between simple, periodic, and continuous interest rates.
//!
//! - Simple: Linear accrual (Money Market)
//! - Periodic: Compounded n times per year (Bond/Swap)
//! - Continuous: Exponential accrual (Options/Trees)

use crate::errors::core_to_py;
use finstack_core::dates::rate_conversions::{
    continuous_to_periodic, continuous_to_simple, periodic_to_continuous, periodic_to_simple,
    simple_to_continuous, simple_to_periodic,
};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

#[pyfunction(name = "simple_to_periodic")]
#[pyo3(text_signature = "(simple_rate, year_fraction, periods_per_year)")]
/// Convert a simple (linear) interest rate to a periodically compounded rate.
///
/// Parameters
/// ----------
/// simple_rate : float
///     The simple interest rate.
/// year_fraction : float
///     Time period as a fraction of a year.
/// periods_per_year : int
///     Compounding frequency (e.g., 2 for semi-annual).
///
/// Returns
/// -------
/// float
///     Equivalent periodically compounded rate.
fn simple_to_periodic_py(
    simple_rate: f64,
    year_fraction: f64,
    periods_per_year: u32,
) -> PyResult<f64> {
    simple_to_periodic(simple_rate, year_fraction, periods_per_year).map_err(core_to_py)
}

#[pyfunction(name = "periodic_to_simple")]
#[pyo3(text_signature = "(periodic_rate, year_fraction, periods_per_year)")]
/// Convert a periodically compounded rate to a simple (linear) rate.
///
/// Parameters
/// ----------
/// periodic_rate : float
///     Periodically compounded rate.
/// year_fraction : float
///     Time period as a fraction of a year.
/// periods_per_year : int
///     Compounding frequency.
///
/// Returns
/// -------
/// float
///     Equivalent simple interest rate.
fn periodic_to_simple_py(
    periodic_rate: f64,
    year_fraction: f64,
    periods_per_year: u32,
) -> PyResult<f64> {
    periodic_to_simple(periodic_rate, year_fraction, periods_per_year).map_err(core_to_py)
}

#[pyfunction(name = "periodic_to_continuous")]
#[pyo3(text_signature = "(periodic_rate, periods_per_year)")]
/// Convert a periodically compounded rate to a continuously compounded rate.
///
/// Parameters
/// ----------
/// periodic_rate : float
///     Periodically compounded rate.
/// periods_per_year : int
///     Compounding frequency.
///
/// Returns
/// -------
/// float
///     Equivalent continuously compounded rate.
fn periodic_to_continuous_py(periodic_rate: f64, periods_per_year: u32) -> PyResult<f64> {
    periodic_to_continuous(periodic_rate, periods_per_year).map_err(core_to_py)
}

#[pyfunction(name = "continuous_to_periodic")]
#[pyo3(text_signature = "(continuous_rate, periods_per_year)")]
/// Convert a continuously compounded rate to a periodically compounded rate.
///
/// Parameters
/// ----------
/// continuous_rate : float
///     Continuously compounded rate.
/// periods_per_year : int
///     Target compounding frequency.
///
/// Returns
/// -------
/// float
///     Equivalent periodically compounded rate.
fn continuous_to_periodic_py(continuous_rate: f64, periods_per_year: u32) -> PyResult<f64> {
    continuous_to_periodic(continuous_rate, periods_per_year).map_err(core_to_py)
}

#[pyfunction(name = "simple_to_continuous")]
#[pyo3(text_signature = "(simple_rate, year_fraction)")]
/// Convert a simple rate to a continuously compounded rate.
///
/// Parameters
/// ----------
/// simple_rate : float
///     Simple interest rate.
/// year_fraction : float
///     Time period as a fraction of a year.
///
/// Returns
/// -------
/// float
///     Equivalent continuously compounded rate.
fn simple_to_continuous_py(simple_rate: f64, year_fraction: f64) -> PyResult<f64> {
    simple_to_continuous(simple_rate, year_fraction).map_err(core_to_py)
}

#[pyfunction(name = "continuous_to_simple")]
#[pyo3(text_signature = "(continuous_rate, year_fraction)")]
/// Convert a continuously compounded rate to a simple rate.
///
/// Parameters
/// ----------
/// continuous_rate : float
///     Continuously compounded rate.
/// year_fraction : float
///     Time period as a fraction of a year.
///
/// Returns
/// -------
/// float
///     Equivalent simple interest rate.
fn continuous_to_simple_py(continuous_rate: f64, year_fraction: f64) -> PyResult<f64> {
    continuous_to_simple(continuous_rate, year_fraction).map_err(core_to_py)
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "rate_conversions")?;
    module.setattr(
        "__doc__",
        "Interest rate conversion utilities (simple, periodic, continuous).",
    )?;
    module.add_function(wrap_pyfunction!(simple_to_periodic_py, &module)?)?;
    module.add_function(wrap_pyfunction!(periodic_to_simple_py, &module)?)?;
    module.add_function(wrap_pyfunction!(periodic_to_continuous_py, &module)?)?;
    module.add_function(wrap_pyfunction!(continuous_to_periodic_py, &module)?)?;
    module.add_function(wrap_pyfunction!(simple_to_continuous_py, &module)?)?;
    module.add_function(wrap_pyfunction!(continuous_to_simple_py, &module)?)?;

    let exports = [
        "simple_to_periodic",
        "periodic_to_simple",
        "periodic_to_continuous",
        "continuous_to_periodic",
        "simple_to_continuous",
        "continuous_to_simple",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}


