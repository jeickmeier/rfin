//! Python bindings for correlation utilities.

use finstack_correlation::{
    cholesky_decompose, correlation_bounds, joint_probabilities, validate_correlation_matrix,
    CorrelatedBernoulli,
};
use pyo3::prelude::*;
use pyo3::types::PyModule;

use crate::errors::ValidationError;

// ---------------------------------------------------------------------------
// CorrelatedBernoulli
// ---------------------------------------------------------------------------

/// Correlated Bernoulli distribution for paired binary events.
///
/// Precomputes joint probabilities from marginal probabilities and a
/// correlation, enabling fast repeated sampling.
#[pyclass(
    name = "CorrelatedBernoulli",
    module = "finstack.correlation",
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyCorrelatedBernoulli {
    pub(crate) inner: CorrelatedBernoulli,
}

#[pymethods]
impl PyCorrelatedBernoulli {
    /// Create a correlated Bernoulli distribution.
    ///
    /// Correlation is clamped to the Fréchet-Hoeffding bounds.
    ///
    /// Parameters
    /// ----------
    /// p1 : float
    ///     Marginal probability of first event (clamped to [0, 1]).
    /// p2 : float
    ///     Marginal probability of second event (clamped to [0, 1]).
    /// correlation : float
    ///     Correlation between events (clamped to feasible bounds).
    #[new]
    fn new(p1: f64, p2: f64, correlation: f64) -> Self {
        Self {
            inner: CorrelatedBernoulli::new(p1, p2, correlation),
        }
    }

    /// Sample a pair of correlated binary outcomes.
    ///
    /// Parameters
    /// ----------
    /// u : float
    ///     Uniform random value in [0, 1].
    ///
    /// Returns
    /// -------
    /// tuple[int, int]
    ///     A pair (x1, x2) where each is 0 or 1.
    fn sample_from_uniform(&self, u: f64) -> (u8, u8) {
        self.inner.sample_from_uniform(u)
    }

    /// Joint probabilities (p11, p10, p01, p00).
    fn joint_probabilities(&self) -> (f64, f64, f64, f64) {
        self.inner.joint_probabilities()
    }

    fn __repr__(&self) -> String {
        let (p11, p10, p01, p00) = self.inner.joint_probabilities();
        format!(
            "CorrelatedBernoulli(p11={:.4}, p10={:.4}, p01={:.4}, p00={:.4})",
            p11, p10, p01, p00
        )
    }
}

// ---------------------------------------------------------------------------
// Free Functions
// ---------------------------------------------------------------------------

/// Validate a correlation matrix.
///
/// Checks symmetry, unit diagonal, bounds, and positive semi-definiteness.
///
/// Parameters
/// ----------
/// matrix : list[float]
///     Flattened row-major correlation matrix.
/// n : int
///     Number of factors (matrix should be n×n).
///
/// Raises
/// ------
/// ValidationError
///     If the matrix is invalid.
#[pyfunction]
#[pyo3(name = "validate_correlation_matrix")]
fn py_validate_correlation_matrix(matrix: Vec<f64>, n: usize) -> PyResult<()> {
    validate_correlation_matrix(&matrix, n)
        .map_err(|e| ValidationError::new_err(format!("Invalid correlation matrix: {e}")))
}

/// Cholesky decomposition of a correlation matrix.
///
/// Uses diagonal pivoting to handle near-singular matrices.
///
/// Parameters
/// ----------
/// matrix : list[float]
///     Flattened row-major correlation matrix.
/// n : int
///     Matrix dimension.
///
/// Returns
/// -------
/// list[float]
///     Flattened lower-triangular Cholesky factor (row-major).
///
/// Raises
/// ------
/// ValidationError
///     If the matrix is not positive semi-definite.
#[pyfunction]
#[pyo3(name = "cholesky_decompose")]
fn py_cholesky_decompose(matrix: Vec<f64>, n: usize) -> PyResult<Vec<f64>> {
    let factor = cholesky_decompose(&matrix, n)
        .map_err(|e| ValidationError::new_err(format!("Cholesky decomposition failed: {e}")))?;
    Ok(factor.factor_matrix().to_vec())
}

/// Compute Fréchet-Hoeffding correlation bounds.
///
/// Parameters
/// ----------
/// p1 : float
///     Marginal probability P(X₁=1).
/// p2 : float
///     Marginal probability P(X₂=1).
///
/// Returns
/// -------
/// tuple[float, float]
///     (ρ_min, ρ_max) of achievable correlation.
#[pyfunction]
#[pyo3(name = "correlation_bounds")]
fn py_correlation_bounds(p1: f64, p2: f64) -> (f64, f64) {
    correlation_bounds(p1, p2)
}

/// Compute joint probabilities for correlated Bernoulli events.
///
/// Parameters
/// ----------
/// p1 : float
///     Marginal probability P(X₁=1).
/// p2 : float
///     Marginal probability P(X₂=1).
/// correlation : float
///     Correlation between events.
///
/// Returns
/// -------
/// tuple[float, float, float, float]
///     (p11, p10, p01, p00) joint probabilities.
#[pyfunction]
#[pyo3(name = "joint_probabilities")]
fn py_joint_probabilities(p1: f64, p2: f64, correlation: f64) -> (f64, f64, f64, f64) {
    joint_probabilities(p1, p2, correlation)
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

pub(crate) fn register<'py>(
    _py: Python<'py>,
    m: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    m.add_class::<PyCorrelatedBernoulli>()?;
    m.add_function(wrap_pyfunction!(py_validate_correlation_matrix, m)?)?;
    m.add_function(wrap_pyfunction!(py_cholesky_decompose, m)?)?;
    m.add_function(wrap_pyfunction!(py_correlation_bounds, m)?)?;
    m.add_function(wrap_pyfunction!(py_joint_probabilities, m)?)?;

    Ok(vec![
        "CorrelatedBernoulli",
        "validate_correlation_matrix",
        "cholesky_decompose",
        "correlation_bounds",
        "joint_probabilities",
    ])
}
