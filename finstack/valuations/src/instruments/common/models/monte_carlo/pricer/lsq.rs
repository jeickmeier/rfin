//! Shared least-squares solver for LSMC regression.
//!
//! Provides a robust SVD-based solver used by both equity and swaption LSMC pricers.

use finstack_core::Result;

/// Solve least squares problem using SVD (Singular Value Decomposition).
///
/// Solves: min || Xβ - y ||²
///
/// where X is n x k design matrix (row-major).
///
/// Uses nalgebra's SVD decomposition which is numerically more stable
/// than normal equations (Cholesky) or QR for ill-conditioned systems.
///
/// # Numerical Stability
///
/// SVD is the most robust method for least squares:
/// - Avoids forming X'X which squares the condition number: cond(X'X) ≈ cond(X)²
/// - Handles rank-deficient matrices gracefully
/// - Uses threshold-based pseudo-inverse for numerical stability
///
/// This is critical for LSMC with high-degree polynomials or extreme spot/rate ranges
/// where regression matrices can be ill-conditioned.
///
/// # Arguments
///
/// * `design` - Design matrix X in row-major order (n x k)
/// * `y` - Response vector (n elements)
/// * `n` - Number of observations (rows)
/// * `k` - Number of basis functions (columns)
///
/// # Returns
///
/// Coefficient vector β (k elements)
pub fn solve_least_squares(design: &[f64], y: &[f64], n: usize, k: usize) -> Result<Vec<f64>> {
    use nalgebra::{DMatrix, DVector};

    // Check for degenerate cases
    if n < k {
        return Err(finstack_core::Error::Internal);
    }

    // Convert to nalgebra matrices
    let x_matrix = DMatrix::from_row_slice(n, k, design);
    let y_vector = DVector::from_vec(y.to_vec());

    // Solve least squares problem using SVD (more robust than QR for overdetermined systems)
    let svd = x_matrix.svd(true, true);

    match svd.solve(&y_vector, 1e-10) {
        Ok(beta) => {
            // Convert back to Vec<f64>
            Ok(beta.as_slice().to_vec())
        }
        Err(_) => {
            // SVD decomposition failed (singular or near-singular matrix)
            // This can happen with:
            // - Linearly dependent basis functions
            // - Too few ITM paths for regression
            // - Numerical issues with extreme values

            // Fallback: return zero coefficients (exercise immediately)
            // This is conservative but safe
            tracing::warn!(
                "LSMC regression failed (singular matrix), using zero continuation value"
            );
            Ok(vec![0.0; k])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solve_least_squares_simple() {
        // Simple regression: y = 2 + 3x
        // Design matrix: [1, x_i] for each observation
        let design = vec![
            1.0, 1.0, // observation 1: x=1
            1.0, 2.0, // observation 2: x=2
            1.0, 3.0, // observation 3: x=3
        ];
        let y = vec![5.0, 8.0, 11.0]; // y = 2 + 3x

        let solution = solve_least_squares(&design, &y, 3, 2).unwrap();

        // Should recover β₀=2, β₁=3
        assert!((solution[0] - 2.0).abs() < 1e-10);
        assert!((solution[1] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_solve_least_squares_singular() {
        // Test with singular matrix (linearly dependent columns)
        let design = vec![
            1.0, 1.0, 2.0, // Column 3 = 2 * Column 2
            1.0, 2.0, 4.0, 1.0, 3.0, 6.0,
        ];
        let y = vec![1.0, 2.0, 3.0];

        let solution = solve_least_squares(&design, &y, 3, 3).unwrap();

        // Should return fallback zero vector or a valid solution
        assert!(solution.len() == 3);
        assert!(solution.iter().all(|&x| x.is_finite()));
    }

    #[test]
    fn test_solve_least_squares_ill_conditioned() {
        // Test with ill-conditioned polynomial design matrix
        // (narrow x range with high-degree polynomial)
        let x_values = vec![1.0, 1.1, 1.2, 1.3, 1.4];
        let mut design = Vec::new();

        for &x in &x_values {
            design.push(1.0);
            design.push(x);
            design.push(x * x);
            design.push(x * x * x);
        }

        let y = vec![1.0, 1.2, 1.5, 1.8, 2.0];

        let solution = solve_least_squares(&design, &y, 5, 4);

        // SVD should handle ill-conditioning gracefully
        assert!(solution.is_ok());
        let beta = solution.unwrap();
        assert_eq!(beta.len(), 4);
        assert!(beta.iter().all(|&x| x.is_finite()));
    }
}

