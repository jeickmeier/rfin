use finstack_core::math::linalg::{
    apply_correlation as core_apply_correlation,
    build_correlation_matrix as core_build_correlation_matrix,
    cholesky_correlation as core_cholesky_correlation, cholesky_decomposition as core_cholesky,
    cholesky_solve as core_cholesky_solve, validate_correlation_matrix as core_validate_corr,
    CholeskyError as CoreCholeskyError, DIAGONAL_TOLERANCE, PIVOT_TOLERANCE_RELATIVE,
    SINGULAR_THRESHOLD, SYMMETRY_TOLERANCE,
};
use finstack_core::{Error as CoreError, InputError};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

fn map_cholesky_error(err: CoreCholeskyError) -> PyErr {
    crate::errors::CholeskyError::new_err(err.to_string())
}

fn map_cholesky_solve_error(err: CoreError) -> PyErr {
    match err {
        CoreError::Input(InputError::DimensionMismatch) => {
            crate::errors::CholeskyError::new_err("Cholesky solve dimension mismatch")
        }
        CoreError::Input(InputError::Invalid) => crate::errors::CholeskyError::new_err(
            "Cholesky solve failed: singular or invalid factor",
        ),
        other => crate::errors::CholeskyError::new_err(other.to_string()),
    }
}

fn flatten_square_matrix(matrix: Vec<Vec<f64>>, name: &str) -> PyResult<(Vec<f64>, usize)> {
    let n = matrix.len();
    if n == 0 {
        return Ok((vec![], 0));
    }

    if matrix.iter().any(|row| row.len() != n) {
        return Err(crate::errors::CholeskyError::new_err(format!(
            "{name} must be a square matrix"
        )));
    }

    Ok((matrix.into_iter().flatten().collect(), n))
}

#[pyfunction(name = "cholesky_decomposition")]
#[pyo3(text_signature = "(matrix)")]
/// Compute the Cholesky decomposition of a symmetric positive-definite matrix.
///
/// Returns lower triangular matrix L such that A = L * L^T.
pub fn cholesky_decomposition_py(matrix: Vec<Vec<f64>>) -> PyResult<Vec<Vec<f64>>> {
    let (flat, n) = flatten_square_matrix(matrix, "Input")?;
    if n == 0 {
        return Ok(vec![]);
    }

    let result_flat = core_cholesky(&flat, n).map_err(map_cholesky_error)?;

    // Re-nest
    let result_nested: Vec<Vec<f64>> = result_flat.chunks(n).map(|chunk| chunk.to_vec()).collect();
    Ok(result_nested)
}

#[pyfunction(name = "cholesky_correlation")]
#[pyo3(text_signature = "(matrix)")]
/// Compute the pivoted Cholesky factorisation of a correlation or covariance matrix.
///
/// Uses complete diagonal pivoting (Higham's algorithm) with a relative pivot tolerance,
/// making it numerically robust for near-singular and positive-semidefinite matrices.
/// The returned factor is in the **original variable ordering** of the input.
///
/// Parameters
/// ----------
/// matrix : list[list[float]]
///     Symmetric positive-semidefinite square matrix.
///
/// Returns
/// -------
/// tuple[list[list[float]], int]
///     ``(L, effective_rank)`` where ``L`` is the lower-triangular Cholesky factor in
///     original variable order and ``effective_rank`` is the number of numerically
///     non-zero pivots (equals ``n`` for full-rank matrices).
///
/// Raises
/// ------
/// CholeskyError
///     If the matrix is indefinite (has a significantly negative pivot).
pub fn cholesky_correlation_py(matrix: Vec<Vec<f64>>) -> PyResult<(Vec<Vec<f64>>, usize)> {
    let (flat, n) = flatten_square_matrix(matrix, "Input")?;
    if n == 0 {
        return Ok((vec![], 0));
    }

    let factor = core_cholesky_correlation(&flat, n).map_err(map_cholesky_error)?;
    let effective_rank = factor.effective_rank();
    let result_nested: Vec<Vec<f64>> = factor
        .factor_matrix()
        .chunks(n)
        .map(|chunk| chunk.to_vec())
        .collect();
    Ok((result_nested, effective_rank))
}

#[pyfunction(name = "validate_correlation_matrix")]
#[pyo3(text_signature = "(matrix)")]
/// Validate that a matrix satisfies correlation matrix properties.
///
/// Checks diagonal elements are 1.0, off-diagonal elements are in [-1, 1],
/// the matrix is symmetric, and the matrix is positive semi-definite.
///
/// Parameters
/// ----------
/// matrix : list[list[float]]
///     Square matrix to validate.
///
/// Returns
/// -------
/// bool
///     ``True`` if the matrix is a valid correlation matrix.
pub fn validate_correlation_matrix_py(matrix: Vec<Vec<f64>>) -> PyResult<bool> {
    let (flat, n) = flatten_square_matrix(matrix, "Input")?;
    if n == 0 {
        return Ok(true);
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

    let (chol_flat, chol_n) = flatten_square_matrix(cholesky, "Cholesky factor")?;
    if chol_n != n {
        return Err(crate::errors::CholeskyError::new_err(
            "Cholesky factor must be an n x n square matrix matching shock dimension",
        ));
    }

    let mut correlated = vec![0.0; n];
    core_apply_correlation(&chol_flat, &independent, &mut correlated)
        .map_err(map_cholesky_error)?;
    Ok(correlated)
}

#[pyfunction(name = "cholesky_solve")]
#[pyo3(text_signature = "(cholesky, b)")]
/// Solve ``A x = b`` from a lower-triangular Cholesky factor ``L`` where ``A = L L^T``.
pub fn cholesky_solve_py(cholesky: Vec<Vec<f64>>, b: Vec<f64>) -> PyResult<Vec<f64>> {
    let (chol_flat, n) = flatten_square_matrix(cholesky, "Cholesky factor")?;
    if n != b.len() {
        return Err(crate::errors::CholeskyError::new_err(
            "Cholesky solve dimension mismatch",
        ));
    }

    let mut x = vec![0.0; n];
    core_cholesky_solve(&chol_flat, &b, &mut x).map_err(map_cholesky_solve_error)?;
    Ok(x)
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
    let flat = core_build_correlation_matrix(n, &correlations)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
    let nested: Vec<Vec<f64>> = flat.chunks(n).map(|chunk| chunk.to_vec()).collect();
    Ok(nested)
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "linalg")?;
    module.setattr("__doc__", "Linear algebra utilities (Cholesky, etc.).")?;
    module.add(
        "CholeskyError",
        py.get_type::<crate::errors::CholeskyError>(),
    )?;
    module.add("SINGULAR_THRESHOLD", SINGULAR_THRESHOLD)?;
    module.add("DIAGONAL_TOLERANCE", DIAGONAL_TOLERANCE)?;
    module.add("SYMMETRY_TOLERANCE", SYMMETRY_TOLERANCE)?;
    module.add("PIVOT_TOLERANCE_RELATIVE", PIVOT_TOLERANCE_RELATIVE)?;
    module.add_function(wrap_pyfunction!(cholesky_decomposition_py, &module)?)?;
    module.add_function(wrap_pyfunction!(cholesky_correlation_py, &module)?)?;
    module.add_function(wrap_pyfunction!(cholesky_solve_py, &module)?)?;
    module.add_function(wrap_pyfunction!(validate_correlation_matrix_py, &module)?)?;
    module.add_function(wrap_pyfunction!(apply_correlation_py, &module)?)?;
    module.add_function(wrap_pyfunction!(build_correlation_matrix_py, &module)?)?;

    let exports = [
        "CholeskyError",
        "SINGULAR_THRESHOLD",
        "DIAGONAL_TOLERANCE",
        "SYMMETRY_TOLERANCE",
        "PIVOT_TOLERANCE_RELATIVE",
        "cholesky_decomposition",
        "cholesky_correlation",
        "cholesky_solve",
        "validate_correlation_matrix",
        "apply_correlation",
        "build_correlation_matrix",
    ];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
