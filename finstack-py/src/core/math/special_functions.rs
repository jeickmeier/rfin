use finstack_core::math::special_functions::{
    erf as core_erf, norm_cdf as core_norm_cdf, norm_pdf as core_norm_pdf,
    standard_normal_inv_cdf as core_std_norm_inv_cdf,
};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

#[pyfunction(name = "norm_cdf")]
#[pyo3(text_signature = "(x, mean=0.0, std_dev=1.0)")]
pub fn norm_cdf_py(x: f64, mean: Option<f64>, std_dev: Option<f64>) -> PyResult<f64> {
    let m = mean.unwrap_or(0.0);
    let s = std_dev.unwrap_or(1.0);
    if s <= 0.0 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "std_dev must be positive",
        ));
    }
    // Standardize: z = (x - mu) / sigma
    Ok(core_norm_cdf((x - m) / s))
}

#[pyfunction(name = "norm_pdf")]
#[pyo3(text_signature = "(x, mean=0.0, std_dev=1.0)")]
pub fn norm_pdf_py(x: f64, mean: Option<f64>, std_dev: Option<f64>) -> PyResult<f64> {
    let m = mean.unwrap_or(0.0);
    let s = std_dev.unwrap_or(1.0);
    if s <= 0.0 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "std_dev must be positive",
        ));
    }
    // PDF(x) = (1/sigma) * phi((x - mu) / sigma)
    Ok(core_norm_pdf((x - m) / s) / s)
}

#[pyfunction(name = "standard_normal_inv_cdf")]
#[pyo3(text_signature = "(p)")]
pub fn standard_normal_inv_cdf_py(p: f64) -> PyResult<f64> {
    Ok(core_std_norm_inv_cdf(p))
}

#[pyfunction(name = "erf")]
#[pyo3(text_signature = "(x)")]
pub fn erf_py(x: f64) -> PyResult<f64> {
    Ok(core_erf(x))
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "special_functions")?;
    module.setattr(
        "__doc__",
        "Special mathematical functions (Normal CDF/PDF, Error function, etc.).",
    )?;
    module.add_function(wrap_pyfunction!(norm_cdf_py, &module)?)?;
    module.add_function(wrap_pyfunction!(norm_pdf_py, &module)?)?;
    module.add_function(wrap_pyfunction!(standard_normal_inv_cdf_py, &module)?)?;
    module.add_function(wrap_pyfunction!(erf_py, &module)?)?;

    let exports = [
        "norm_cdf",
        "norm_pdf",
        "standard_normal_inv_cdf",
        "erf",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
