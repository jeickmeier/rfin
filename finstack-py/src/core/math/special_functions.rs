use finstack_core::math::special_functions::{
    erf as core_erf, norm_cdf as core_norm_cdf, norm_pdf as core_norm_pdf,
    standard_normal_inv_cdf as core_std_norm_inv_cdf, student_t_cdf as core_student_t_cdf,
    student_t_inv_cdf as core_student_t_inv_cdf,
};
use pyo3::exceptions::PyValueError;
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

#[pyfunction(name = "student_t_cdf")]
#[pyo3(text_signature = "(x, df)")]
/// Cumulative distribution function of Student's t-distribution.
///
/// Args:
///     x (float): Point at which to evaluate the CDF.
///     df (float): Degrees of freedom (must be positive).
///
/// Returns:
///     float: Cumulative probability up to ``x``.
///
/// Examples:
///     >>> from finstack.core.math.special_functions import student_t_cdf
///     >>> student_t_cdf(0.0, 10.0)
///     0.5
pub fn student_t_cdf_py(x: f64, df: f64) -> PyResult<f64> {
    if df <= 0.0 {
        return Err(PyValueError::new_err("df must be positive"));
    }
    Ok(core_student_t_cdf(x, df))
}

#[pyfunction(name = "student_t_inv_cdf")]
#[pyo3(text_signature = "(p, df)")]
/// Inverse cumulative distribution function (quantile) of Student's t-distribution.
///
/// Args:
///     p (float): Probability level in [0, 1].
///     df (float): Degrees of freedom (must be positive).
///
/// Returns:
///     float: Quantile at probability ``p``.
///
/// Examples:
///     >>> from finstack.core.math.special_functions import student_t_inv_cdf
///     >>> student_t_inv_cdf(0.975, 10.0)  # doctest: +SKIP
pub fn student_t_inv_cdf_py(p: f64, df: f64) -> PyResult<f64> {
    if !(0.0..=1.0).contains(&p) {
        return Err(PyValueError::new_err("p must be in [0, 1]"));
    }
    if df <= 0.0 {
        return Err(PyValueError::new_err("df must be positive"));
    }
    Ok(core_student_t_inv_cdf(p, df))
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "special_functions")?;
    module.setattr(
        "__doc__",
        concat!(
            "Special mathematical functions (Normal and Student's t distributions, Error function).\n\n",
            "Provides cumulative distribution functions, probability density functions, and quantile functions."
        ),
    )?;
    module.add_function(wrap_pyfunction!(norm_cdf_py, &module)?)?;
    module.add_function(wrap_pyfunction!(norm_pdf_py, &module)?)?;
    module.add_function(wrap_pyfunction!(standard_normal_inv_cdf_py, &module)?)?;
    module.add_function(wrap_pyfunction!(erf_py, &module)?)?;
    module.add_function(wrap_pyfunction!(student_t_cdf_py, &module)?)?;
    module.add_function(wrap_pyfunction!(student_t_inv_cdf_py, &module)?)?;

    let exports = [
        "norm_cdf",
        "norm_pdf",
        "standard_normal_inv_cdf",
        "erf",
        "student_t_cdf",
        "student_t_inv_cdf",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
