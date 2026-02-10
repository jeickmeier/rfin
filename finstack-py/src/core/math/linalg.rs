use finstack_core::math::linalg::{
    apply_correlation as core_apply_correlation,
    build_correlation_matrix as core_build_correlation_matrix,
    cholesky_decomposition as core_cholesky, validate_correlation_matrix as core_validate_corr,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

#[pyfunction(name = "cholesky_decomposition")]
#[pyo3(text_signature = "(matrix)")]
/// Compute the Cholesky decomposition of a symmetric positive-definite matrix.
///
/// Returns lower triangular matrix L such that A = L * L^T.
pub fn cholesky_decomposition_py(matrix: Vec<Vec<f64>>) -> PyResult<Vec<Vec<f64>>> {
    // Flatten for core
    let n = matrix.len();
    if n == 0 {
        return Ok(vec![]);
    }
    let flat: Vec<f64> = matrix.into_iter().flatten().collect();
    if flat.len() != n * n {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Input must be a square matrix",
        ));
    }

    let result_flat = core_cholesky(&flat, n).map_err(|e| match e {
        finstack_core::math::linalg::CholeskyError::DimensionMismatch { expected, actual } => {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Matrix dimension mismatch: expected {expected}x{expected}, got {actual} elements"
            ))
        }
        _ => pyo3::exceptions::PyValueError::new_err(e.to_string()),
    })?;

    // Re-nest
    let result_nested: Vec<Vec<f64>> = result_flat.chunks(n).map(|chunk| chunk.to_vec()).collect();
    Ok(result_nested)
}

#[pyfunction(name = "validate_correlation_matrix")]
#[pyo3(text_signature = "(matrix, tolerance=1e-10)")]
pub fn validate_correlation_matrix_py(
    matrix: Vec<Vec<f64>>,
    _tolerance: Option<f64>,
) -> PyResult<bool> {
    let n = matrix.len();
    if n == 0 {
        return Ok(true);
    }
    let flat: Vec<f64> = matrix.into_iter().flatten().collect();
    if flat.len() != n * n {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Input must be a square matrix",
        ));
    }

    Ok(core_validate_corr(&flat, n).is_ok())
}

#[pyfunction(name = "apply_correlation")]
#[pyo3(text_signature = "(cholesky, independent)")]
/// Apply a Cholesky factor to independent shocks to obtain correlated shocks.
///
/// Parameters
/// ----------
/// cholesky : list[list[float]]
///     Lower‑triangular Cholesky factor ``L`` of a correlation/covariance matrix.
/// independent : list[float]
///     Vector of independent standard normal shocks.
///
/// Returns
/// -------
/// list[float]
///     Vector of correlated shocks ``L * z``.
pub fn apply_correlation_py(cholesky: Vec<Vec<f64>>, independent: Vec<f64>) -> PyResult<Vec<f64>> {
    let n = independent.len();
    if n == 0 {
        return Ok(vec![]);
    }

    if cholesky.len() != n || cholesky.iter().any(|row| row.len() != n) {
        return Err(PyValueError::new_err(
            "Cholesky factor must be an n x n square matrix matching shock dimension",
        ));
    }

    let mut chol_flat = Vec::with_capacity(n * n);
    for row in cholesky {
        chol_flat.extend_from_slice(&row);
    }

    let mut correlated = vec![0.0; n];
    core_apply_correlation(&chol_flat, &independent, &mut correlated);
    Ok(correlated)
}

#[pyfunction(name = "build_correlation_matrix")]
#[pyo3(text_signature = "(n, correlations)")]
/// Build a correlation matrix from index/ρ pairs.
///
/// Parameters
/// ----------
/// n : int
///     Dimension of the (square) correlation matrix.
/// correlations : sequence[tuple[int, int, float]]
///     Triples ``(i, j, rho_ij)`` specifying off‑diagonal correlations.
///
/// Returns
/// -------
/// list[list[float]]
///     ``n x n`` symmetric correlation matrix with ones on the diagonal.
pub fn build_correlation_matrix_py(
    n: usize,
    correlations: Vec<(usize, usize, f64)>,
) -> PyResult<Vec<Vec<f64>>> {
    if n == 0 {
        return Ok(vec![]);
    }
    let flat = core_build_correlation_matrix(n, &correlations);
    let nested: Vec<Vec<f64>> = flat.chunks(n).map(|chunk| chunk.to_vec()).collect();
    Ok(nested)
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "linalg")?;
    module.setattr("__doc__", "Linear algebra utilities (Cholesky, etc.).")?;
    module.add_function(wrap_pyfunction!(cholesky_decomposition_py, &module)?)?;
    module.add_function(wrap_pyfunction!(validate_correlation_matrix_py, &module)?)?;
    module.add_function(wrap_pyfunction!(apply_correlation_py, &module)?)?;
    module.add_function(wrap_pyfunction!(build_correlation_matrix_py, &module)?)?;

    let exports = [
        "cholesky_decomposition",
        "validate_correlation_matrix",
        "apply_correlation",
        "build_correlation_matrix",
    ];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
