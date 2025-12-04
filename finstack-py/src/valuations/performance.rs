//! Python passthrough bindings for performance utilities (XIRR, IRR, NPV).
//!
//! These wrappers perform only Python<->Rust type conversion and delegate
//! calculations to the core finstack implementations.

use crate::core::cashflow::performance::{
    py_irr_periodic as core_py_irr_periodic, py_npv as core_py_npv,
};
use crate::core::cashflow::xirr::py_xirr as core_py_xirr;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

#[pyfunction(name = "xirr")]
#[pyo3(signature = (cash_flows, guess=None), text_signature = "(cash_flows, guess=None)")]
pub fn py_xirr_passthrough(
    cash_flows: Vec<(Bound<'_, PyAny>, f64)>,
    guess: Option<f64>,
) -> PyResult<f64> {
    core_py_xirr(cash_flows, guess)
}

#[pyfunction(name = "npv")]
#[pyo3(
    signature = (cash_flows, discount_rate, base_date=None, day_count=None),
    text_signature = "(cash_flows, discount_rate, base_date=None, day_count=None)"
)]
pub fn py_npv_passthrough(
    cash_flows: Vec<(Bound<'_, PyAny>, f64)>,
    discount_rate: f64,
    base_date: Option<Bound<'_, PyAny>>,
    day_count: Option<&str>,
) -> PyResult<f64> {
    core_py_npv(cash_flows, discount_rate, base_date, day_count)
}

#[pyfunction(name = "irr_periodic")]
#[pyo3(signature = (amounts, guess=None), text_signature = "(amounts, guess=None)")]
pub fn py_irr_periodic_passthrough(amounts: Vec<f64>, guess: Option<f64>) -> PyResult<f64> {
    core_py_irr_periodic(amounts, guess)
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "performance")?;
    module.setattr(
        "__doc__",
        "Performance helpers (XIRR, IRR, NPV) delegated to finstack-core.",
    )?;

    module.add_function(wrap_pyfunction!(py_xirr_passthrough, &module)?)?;
    module.add_function(wrap_pyfunction!(py_npv_passthrough, &module)?)?;
    module.add_function(wrap_pyfunction!(py_irr_periodic_passthrough, &module)?)?;

    let exports = vec!["xirr", "npv", "irr_periodic"];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports)
}
