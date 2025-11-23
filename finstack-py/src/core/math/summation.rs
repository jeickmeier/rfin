use finstack_core::math::summation::{
    kahan_sum as core_kahan_sum, pairwise_sum as core_pairwise_sum,
    stable_sum as core_stable_sum,
};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

#[pyfunction(name = "kahan_sum")]
#[pyo3(text_signature = "(values)")]
pub fn kahan_sum_py(values: Vec<f64>) -> PyResult<f64> {
    Ok(core_kahan_sum(values.iter().copied()))
}

#[pyfunction(name = "pairwise_sum")]
#[pyo3(text_signature = "(values)")]
pub fn pairwise_sum_py(values: Vec<f64>) -> PyResult<f64> {
    Ok(core_pairwise_sum(&values))
}

#[pyfunction(name = "stable_sum")]
#[pyo3(text_signature = "(values)")]
/// Determinism‑aware summation using the core stable algorithm.
///
/// Parameters
/// ----------
/// values : list[float]
///     Sequence of values to sum.
///
/// Returns
/// -------
/// float
///     Numerically stable sum.
pub fn stable_sum_py(values: Vec<f64>) -> PyResult<f64> {
    Ok(core_stable_sum(&values))
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "summation")?;
    module.setattr(
        "__doc__",
        "Summation algorithms (Kahan, Pairwise) for reduced floating-point error.",
    )?;
    module.add_function(wrap_pyfunction!(kahan_sum_py, &module)?)?;
    module.add_function(wrap_pyfunction!(pairwise_sum_py, &module)?)?;
    module.add_function(wrap_pyfunction!(stable_sum_py, &module)?)?;

    let exports = ["kahan_sum", "pairwise_sum", "stable_sum"];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}

