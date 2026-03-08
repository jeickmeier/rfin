//! Linear algebra utilities for correlation and covariance matrices.
//!
//! Provides essential matrix operations for financial modeling, particularly
//! Cholesky decomposition for generating correlated random variables in Monte
//! Carlo simulations and portfolio risk calculations.
//!
//! # Algorithms
//!
//! - **Cholesky decomposition**: Factorize Σ = L L^T for positive definite matrices
//! - **Correlation application**: Transform independent normals to correlated via L
//! - **Matrix validation**: Check positive-definiteness and correlation properties
//!
//! # Use Cases
//!
//! - **Monte Carlo**: Generate correlated asset paths
//! - **Portfolio risk**: Covariance matrix factorization for VaR
//! - **Factor models**: Decompose returns into systematic factors
//! - **Copula models**: Correlation structure in credit derivatives
//!
//! # Examples
//!
//! ```
//! use finstack_core::math::linalg::{cholesky_decomposition, apply_correlation};
//!
//! // 2x2 correlation matrix: [[1.0, 0.5], [0.5, 1.0]]
//! let corr = vec![1.0, 0.5, 0.5, 1.0];
//! let chol = cholesky_decomposition(&corr, 2).expect("Cholesky decomposition should succeed");
//!
//! // Transform independent standard normals to correlated
//! let z = vec![1.0, 0.0]; // Independent N(0,1) shocks
//! let mut z_corr = vec![0.0; 2];
//! apply_correlation(&chol, &z, &mut z_corr).expect("dimensions match");
//! // z_corr now contains correlated shocks with correlation 0.5
//! ```
//!
//! # References
//!
//! - **Cholesky Decomposition**:
//!   - Golub, G. H., & Van Loan, C. F. (2013). *Matrix Computations* (4th ed.).
//!     Johns Hopkins University Press. Algorithm 4.2.1.
//!   - Press, W. H., et al. (2007). *Numerical Recipes* (3rd ed.). Section 2.9.
//!
//! - **Correlation Matrices**:
//!   - Rebonato, R., & Jäckel, P. (2000). "The Most General Methodology to Create
//!     a Valid Correlation Matrix for Risk Management and Option Pricing Purposes."
//!     *Journal of Risk*, 2(2), 17-27.
//!
//! - **Monte Carlo Applications**:
//!   - Glasserman, P. (2003). *Monte Carlo Methods in Financial Engineering*.
//!     Springer. Section 2.4 (Generating multivariate samples).

use crate::{error, Result};
use thiserror::Error;

/// Default singular threshold for Cholesky decomposition.
pub const SINGULAR_THRESHOLD: f64 = 1e-10;

/// Default tolerance for diagonal elements in correlation matrices.
pub const DIAGONAL_TOLERANCE: f64 = 1e-6;

/// Default tolerance for symmetry checks in correlation matrices.
pub const SYMMETRY_TOLERANCE: f64 = 1e-6;

/// Error type for Cholesky decomposition failures.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum CholeskyError {
    /// Matrix is not positive semi-definite (diagonal element became negative).
    #[error("Matrix is not positive semi-definite: diagonal element {diag} is negative (position [{row}, {row}])")]
    NotPositiveDefinite {
        /// The negative diagonal value
        diag: f64,
        /// The row/column index where failure occurred
        row: usize,
    },
    /// Matrix is numerically singular (division by near-zero element).
    #[error("Matrix is numerically singular: division by {value} (threshold {threshold}) at position [{row}, {col}])")]
    Singular {
        /// The near-zero value that caused the failure
        value: f64,
        /// The row index
        row: usize,
        /// The column index
        col: usize,
        /// The threshold used (1e-10)
        threshold: f64,
    },
    /// Matrix dimension mismatch.
    #[error("Matrix dimension mismatch: expected {expected}x{expected}, got {actual} elements")]
    DimensionMismatch {
        /// Expected dimension
        expected: usize,
        /// Actual number of elements
        actual: usize,
    },
}

/// Cholesky decomposition of a correlation/covariance matrix.
///
/// Computes L such that Σ = L L^T, where Σ is the correlation matrix.
/// Uses the standard algorithm with numerical stability improvements.
///
/// # Arguments
///
/// * `matrix` - Symmetric positive definite matrix (n x n, row-major)
/// * `n` - Matrix dimension
///
/// # Returns
///
/// Lower triangular Cholesky factor L (n x n, row-major)
///
/// # Errors
///
/// Returns `CholeskyError` if:
/// - Matrix is not positive semi-definite (diagonal becomes negative)
/// - Matrix is numerically singular (division by near-zero)
/// - Matrix dimensions don't match
///
/// # Example
///
/// ```
/// use finstack_core::math::linalg::cholesky_decomposition;
///
/// // Correlation matrix: [[1.0, 0.5], [0.5, 1.0]]
/// let corr = vec![1.0, 0.5, 0.5, 1.0];
/// let chol = cholesky_decomposition(&corr, 2).expect("Cholesky decomposition should succeed");
/// // chol = [[1.0, 0.0], [0.5, 0.866...]]
/// ```
pub fn cholesky_decomposition(
    matrix: &[f64],
    n: usize,
) -> std::result::Result<Vec<f64>, CholeskyError> {
    if matrix.len() != n * n {
        return Err(CholeskyError::DimensionMismatch {
            expected: n,
            actual: matrix.len(),
        });
    }

    let mut l = vec![0.0; n * n];

    for i in 0..n {
        for j in 0..=i {
            let mut sum = 0.0;
            for k in 0..j {
                sum += l[i * n + k] * l[j * n + k];
            }

            if i == j {
                let diag = matrix[i * n + i] - sum;
                if diag < 0.0 {
                    return Err(CholeskyError::NotPositiveDefinite { diag, row: i });
                }
                l[i * n + j] = diag.sqrt();
                // Check if diagonal is too small (singular)
                if l[i * n + j].abs() < SINGULAR_THRESHOLD {
                    return Err(CholeskyError::Singular {
                        value: l[i * n + j],
                        row: i,
                        col: j,
                        threshold: SINGULAR_THRESHOLD,
                    });
                }
            } else {
                if l[j * n + j].abs() < SINGULAR_THRESHOLD {
                    return Err(CholeskyError::Singular {
                        value: l[j * n + j],
                        row: i,
                        col: j,
                        threshold: SINGULAR_THRESHOLD,
                    });
                }
                l[i * n + j] = (matrix[i * n + j] - sum) / l[j * n + j];
            }
        }
    }

    Ok(l)
}

/// Apply correlation via Cholesky factor to independent shocks.
///
/// Transforms independent N(0,1) shocks into correlated shocks.
///
/// # Arguments
///
/// * `chol` - Lower triangular Cholesky factor (n x n, row-major)
/// * `independent` - Independent shocks (length n)
/// * `correlated` - Output correlated shocks (length n)
///
/// # Errors
///
/// Returns `CholeskyError::DimensionMismatch` if:
/// - `chol.len() != independent.len() * independent.len()` (Cholesky factor is not n x n)
/// - `correlated.len() != independent.len()` (output buffer has wrong length)
///
/// # Example
///
/// ```
/// use finstack_core::math::linalg::{cholesky_decomposition, apply_correlation};
///
/// let corr = vec![1.0, 0.5, 0.5, 1.0];
/// let chol = cholesky_decomposition(&corr, 2).expect("Cholesky decomposition should succeed");
///
/// let z = vec![1.0, 0.0]; // Independent shocks
/// let mut z_corr = vec![0.0; 2];
/// apply_correlation(&chol, &z, &mut z_corr).expect("dimensions match");
/// ```
pub fn apply_correlation(
    chol: &[f64],
    independent: &[f64],
    correlated: &mut [f64],
) -> std::result::Result<(), CholeskyError> {
    let n = independent.len();
    if chol.len() != n * n {
        return Err(CholeskyError::DimensionMismatch {
            expected: n * n,
            actual: chol.len(),
        });
    }
    if correlated.len() != n {
        return Err(CholeskyError::DimensionMismatch {
            expected: n,
            actual: correlated.len(),
        });
    }

    for i in 0..n {
        correlated[i] = 0.0;
        for j in 0..=i {
            correlated[i] += chol[i * n + j] * independent[j];
        }
    }

    Ok(())
}

/// Solve linear system Ax = b using Cholesky decomposition L of A (A = L L^T).
///
/// Solves L y = b (forward substitution) then L^T x = y (backward substitution).
///
/// # Arguments
///
/// * `chol` - Lower triangular Cholesky factor L (n x n, row-major)
/// * `b` - Right-hand side vector (length n)
/// * `x` - Output solution vector (length n)
pub fn cholesky_solve(chol: &[f64], b: &[f64], x: &mut [f64]) -> Result<()> {
    let n = b.len();
    if chol.len() != n * n || x.len() != n {
        return Err(crate::error::InputError::DimensionMismatch.into());
    }

    // Forward substitution: Solve L y = b
    for i in 0..n {
        let mut sum = 0.0;
        for j in 0..i {
            sum += chol[i * n + j] * x[j];
        }
        let diag = chol[i * n + i];
        if diag.abs() < SINGULAR_THRESHOLD {
            return Err(crate::error::InputError::Invalid.into());
        }
        x[i] = (b[i] - sum) / diag;
    }

    // Backward substitution: Solve L^T x = y
    // x currently holds y
    for i in (0..n).rev() {
        let mut sum = 0.0;
        for j in (i + 1)..n {
            sum += chol[j * n + i] * x[j]; // L[j][i] is L^T[i][j]
        }
        // diag is the same L[i][i]
        x[i] = (x[i] - sum) / chol[i * n + i];
    }

    Ok(())
}

/// Helper to create correlation matrix from correlation pairs.
///
/// # Arguments
///
/// * `n` - Matrix dimension
/// * `correlations` - List of (i, j, ρ_ij) tuples
///
/// # Returns
///
/// Symmetric correlation matrix (n x n, row-major)
///
/// # Errors
///
/// Returns [`CholeskyError::DimensionMismatch`] if an index in `correlations`
/// is out of bounds, or [`crate::Error::Validation`] if a diagonal pair `(i, i)` is
/// supplied.
///
/// # Example
///
/// ```
/// use finstack_core::math::linalg::build_correlation_matrix;
///
/// let correlations = vec![(0, 1, 0.5)];
/// let matrix = build_correlation_matrix(2, &correlations).unwrap();
/// // matrix = [[1.0, 0.5], [0.5, 1.0]]
/// ```
pub fn build_correlation_matrix(
    n: usize,
    correlations: &[(usize, usize, f64)],
) -> crate::Result<Vec<f64>> {
    let mut matrix = vec![0.0; n * n];

    for i in 0..n {
        matrix[i * n + i] = 1.0;
    }

    for &(i, j, rho) in correlations {
        if i >= n || j >= n {
            return Err(crate::Error::Validation(format!(
                "correlation index out of bounds: ({i}, {j}) for matrix size {n}"
            )));
        }
        if i == j {
            return Err(crate::Error::Validation(format!(
                "correlation entry ({i}, {j}) is on the diagonal; diagonal elements are fixed at 1.0"
            )));
        }
        matrix[i * n + j] = rho;
        matrix[j * n + i] = rho;
    }

    Ok(matrix)
}

/// Validate that a matrix is a valid correlation matrix.
///
/// Checks:
/// 1. Diagonal elements are 1.0
/// 2. Off-diagonal elements are in [-1, 1]
/// 3. Matrix is symmetric
/// 4. Matrix is positive semi-definite (via Cholesky)
///
/// # Example
///
/// ```
/// use finstack_core::math::linalg::validate_correlation_matrix;
///
/// let valid = vec![1.0, 0.5, 0.5, 1.0];
/// assert!(validate_correlation_matrix(&valid, 2).is_ok());
///
/// let invalid = vec![1.0, 1.5, 1.5, 1.0]; // Correlation > 1
/// assert!(validate_correlation_matrix(&invalid, 2).is_err());
/// ```
pub fn validate_correlation_matrix(matrix: &[f64], n: usize) -> Result<()> {
    assert_eq!(matrix.len(), n * n, "Matrix must be n x n");

    // Check diagonal
    for i in 0..n {
        let diag = matrix[i * n + i];
        if (diag - 1.0).abs() > DIAGONAL_TOLERANCE {
            return Err(crate::error::InputError::Invalid.into());
        }
    }

    // Check off-diagonal range and symmetry
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }

            let val = matrix[i * n + j];
            if !(-1.0..=1.0).contains(&val) {
                return Err(crate::error::InputError::Invalid.into());
            }

            // Check symmetry
            let val_sym = matrix[j * n + i];
            if (val - val_sym).abs() > SYMMETRY_TOLERANCE {
                return Err(crate::error::InputError::Invalid.into());
            }
        }
    }

    // Check positive semi-definite via Cholesky
    match cholesky_decomposition(matrix, n) {
        Ok(_) => Ok(()),
        Err(CholeskyError::NotPositiveDefinite { .. }) => Err(error::InputError::Invalid.into()),
        Err(CholeskyError::Singular { .. }) => Err(error::InputError::Invalid.into()),
        Err(CholeskyError::DimensionMismatch { .. }) => {
            Err(error::InputError::DimensionMismatch.into())
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn test_cholesky_2x2() {
        // Correlation matrix: [[1.0, 0.5], [0.5, 1.0]]
        let corr = vec![1.0, 0.5, 0.5, 1.0];
        let chol = cholesky_decomposition(&corr, 2)
            .expect("Cholesky decomposition should succeed in test");

        // Expected: [[1.0, 0.0], [0.5, 0.866...]]
        assert!((chol[0] - 1.0).abs() < 1e-10);
        assert!((chol[1] - 0.0).abs() < 1e-10);
        assert!((chol[2] - 0.5).abs() < 1e-10);
        assert!((chol[3] - 0.8660254037844387).abs() < 1e-10);
    }

    #[test]
    fn test_cholesky_identity() {
        let identity = vec![1.0, 0.0, 0.0, 1.0];
        let chol = cholesky_decomposition(&identity, 2)
            .expect("Cholesky decomposition should succeed in test");

        // Should equal identity
        assert_eq!(chol, identity);
    }

    #[test]
    fn test_apply_correlation() {
        let corr = vec![1.0, 0.5, 0.5, 1.0];
        let chol = cholesky_decomposition(&corr, 2)
            .expect("Cholesky decomposition should succeed in test");

        let z = vec![1.0, 0.0];
        let mut z_corr = vec![0.0; 2];
        apply_correlation(&chol, &z, &mut z_corr)
            .expect("apply_correlation should succeed in test");

        assert!((z_corr[0] - 1.0).abs() < 1e-10);
        assert!((z_corr[1] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_build_correlation_matrix() {
        let correlations = vec![(0, 1, 0.5)];
        let matrix = build_correlation_matrix(2, &correlations).expect("valid correlations");

        assert!((matrix[0] - 1.0).abs() < 1e-10); // [0,0]
        assert!((matrix[1] - 0.5).abs() < 1e-10); // [0,1]
        assert!((matrix[2] - 0.5).abs() < 1e-10); // [1,0]
        assert!((matrix[3] - 1.0).abs() < 1e-10); // [1,1]
    }

    #[test]
    fn test_validate_correlation_matrix() {
        // Valid matrix
        let valid = vec![1.0, 0.5, 0.5, 1.0];
        assert!(validate_correlation_matrix(&valid, 2).is_ok());

        // Invalid: diagonal not 1.0
        let invalid_diag = vec![0.9, 0.5, 0.5, 1.0];
        assert!(validate_correlation_matrix(&invalid_diag, 2).is_err());

        // Invalid: off-diagonal > 1.0
        let invalid_range = vec![1.0, 1.5, 1.5, 1.0];
        assert!(validate_correlation_matrix(&invalid_range, 2).is_err());

        // Invalid: not symmetric
        let invalid_sym = vec![1.0, 0.5, 0.3, 1.0];
        assert!(validate_correlation_matrix(&invalid_sym, 2).is_err());

        // Invalid: not positive definite (correlation > 1 is invalid anyway)
        let invalid_pd = vec![1.0, 1.1, 1.1, 1.0];
        assert!(validate_correlation_matrix(&invalid_pd, 2).is_err());
    }

    #[test]
    fn test_cholesky_fails_on_non_pd() {
        // Not positive definite - use a matrix that fails Cholesky properly
        // Matrix with correlation slightly > 1 (clearly not valid)
        let non_pd = vec![1.0, 1.01, 1.01, 1.0];
        let result = cholesky_decomposition(&non_pd, 2);
        assert!(result.is_err());
        // Verify we get descriptive error
        match result.expect_err("Should fail for non-positive-definite matrix") {
            CholeskyError::NotPositiveDefinite { diag, row } => {
                assert!(diag < 0.0);
                assert!(row < 2);
            }
            _ => panic!("Expected NotPositiveDefinite error"),
        }
    }

    #[test]
    fn test_cholesky_descriptive_errors() {
        // Test dimension mismatch
        let small = vec![1.0, 0.5, 0.5, 1.0];
        match cholesky_decomposition(&small, 3) {
            Err(CholeskyError::DimensionMismatch { expected, actual }) => {
                assert_eq!(expected, 3);
                assert_eq!(actual, 4);
            }
            _ => panic!("Expected DimensionMismatch error"),
        }

        // Test near-singular matrix (correlation ≈ 1)
        let near_singular = vec![1.0, 0.9999, 0.9999, 1.0];
        // This might succeed or fail depending on numerical precision
        let result = cholesky_decomposition(&near_singular, 2);
        // Either way, we should get a descriptive error if it fails
        if let Err(e) = result {
            match e {
                CholeskyError::NotPositiveDefinite { .. } | CholeskyError::Singular { .. } => {}
                _ => panic!("Unexpected error type"),
            }
        }
    }
}
