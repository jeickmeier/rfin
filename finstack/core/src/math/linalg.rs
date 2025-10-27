//! Linear algebra utilities for correlation matrices and Cholesky decomposition.
//!
//! This module provides general-purpose linear algebra operations used across
//! the finstack library, including Monte Carlo simulations, portfolio optimization,
//! and factor models.
//!
//! # Examples
//!
//! ```
//! use finstack_core::math::linalg::{cholesky_decomposition, apply_correlation};
//!
//! // Correlation matrix: [[1.0, 0.5], [0.5, 1.0]]
//! let corr = vec![1.0, 0.5, 0.5, 1.0];
//! let chol = cholesky_decomposition(&corr, 2).unwrap();
//!
//! // Apply correlation to independent shocks
//! let z = vec![1.0, 0.0];
//! let mut z_corr = vec![0.0; 2];
//! apply_correlation(&chol, &z, &mut z_corr);
//! ```

use crate::Result;

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
/// # Example
///
/// ```
/// use finstack_core::math::linalg::cholesky_decomposition;
///
/// // Correlation matrix: [[1.0, 0.5], [0.5, 1.0]]
/// let corr = vec![1.0, 0.5, 0.5, 1.0];
/// let chol = cholesky_decomposition(&corr, 2).unwrap();
/// // chol = [[1.0, 0.0], [0.5, 0.866...]]
/// ```
pub fn cholesky_decomposition(matrix: &[f64], n: usize) -> Result<Vec<f64>> {
    assert_eq!(matrix.len(), n * n, "Matrix must be n x n");

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
                    return Err(crate::Error::Internal);
                }
                l[i * n + j] = diag.sqrt();
            } else {
                if l[j * n + j].abs() < 1e-10 {
                    return Err(crate::Error::Internal);
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
/// # Example
///
/// ```
/// use finstack_core::math::linalg::{cholesky_decomposition, apply_correlation};
///
/// let corr = vec![1.0, 0.5, 0.5, 1.0];
/// let chol = cholesky_decomposition(&corr, 2).unwrap();
///
/// let z = vec![1.0, 0.0]; // Independent shocks
/// let mut z_corr = vec![0.0; 2];
/// apply_correlation(&chol, &z, &mut z_corr);
/// ```
pub fn apply_correlation(chol: &[f64], independent: &[f64], correlated: &mut [f64]) {
    let n = independent.len();
    assert_eq!(chol.len(), n * n, "Cholesky factor must be n x n");
    assert_eq!(correlated.len(), n, "Output must have length n");

    for i in 0..n {
        correlated[i] = 0.0;
        for j in 0..=i {
            correlated[i] += chol[i * n + j] * independent[j];
        }
    }
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
/// # Example
///
/// ```
/// use finstack_core::math::linalg::build_correlation_matrix;
///
/// let correlations = vec![(0, 1, 0.5)];
/// let matrix = build_correlation_matrix(2, &correlations);
/// // matrix = [[1.0, 0.5], [0.5, 1.0]]
/// ```
pub fn build_correlation_matrix(n: usize, correlations: &[(usize, usize, f64)]) -> Vec<f64> {
    let mut matrix = vec![0.0; n * n];

    // Set diagonal to 1.0
    for i in 0..n {
        matrix[i * n + i] = 1.0;
    }

    // Set off-diagonal elements (symmetric)
    for &(i, j, rho) in correlations {
        assert!(i < n && j < n, "Indices out of bounds");
        assert!(i != j, "Diagonal elements must be 1.0");
        matrix[i * n + j] = rho;
        matrix[j * n + i] = rho; // Symmetric
    }

    matrix
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
        if (diag - 1.0).abs() > 1e-6 {
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
            if (val - val_sym).abs() > 1e-6 {
                return Err(crate::error::InputError::Invalid.into());
            }
        }
    }

    // Check positive semi-definite via Cholesky
    cholesky_decomposition(matrix, n)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cholesky_2x2() {
        // Correlation matrix: [[1.0, 0.5], [0.5, 1.0]]
        let corr = vec![1.0, 0.5, 0.5, 1.0];
        let chol = cholesky_decomposition(&corr, 2).unwrap();

        // Expected: [[1.0, 0.0], [0.5, 0.866...]]
        assert!((chol[0] - 1.0).abs() < 1e-10);
        assert!((chol[1] - 0.0).abs() < 1e-10);
        assert!((chol[2] - 0.5).abs() < 1e-10);
        assert!((chol[3] - 0.8660254037844387).abs() < 1e-10);
    }

    #[test]
    fn test_cholesky_identity() {
        let identity = vec![1.0, 0.0, 0.0, 1.0];
        let chol = cholesky_decomposition(&identity, 2).unwrap();

        // Should equal identity
        assert_eq!(chol, identity);
    }

    #[test]
    fn test_apply_correlation() {
        let corr = vec![1.0, 0.5, 0.5, 1.0];
        let chol = cholesky_decomposition(&corr, 2).unwrap();

        let z = vec![1.0, 0.0];
        let mut z_corr = vec![0.0; 2];
        apply_correlation(&chol, &z, &mut z_corr);

        assert!((z_corr[0] - 1.0).abs() < 1e-10);
        assert!((z_corr[1] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_build_correlation_matrix() {
        let correlations = vec![(0, 1, 0.5)];
        let matrix = build_correlation_matrix(2, &correlations);

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
        assert!(cholesky_decomposition(&non_pd, 2).is_err());
    }
}

