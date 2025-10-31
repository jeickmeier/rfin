//! Correlation handling for multi-factor processes.
//!
//! Re-exports linear algebra utilities from finstack_core::math::linalg
//! for convenient access in Monte Carlo simulations.

// Re-export from core/math/linalg
pub use finstack_core::math::linalg::{
    apply_correlation, build_correlation_matrix, cholesky_decomposition,
    validate_correlation_matrix, CholeskyError,
};

// Tests are now in core/math/linalg - no need to duplicate here
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_re_exports_work() {
        // Quick smoke test to ensure re-exports are accessible
        let corr = vec![1.0, 0.5, 0.5, 1.0];
        let chol = cholesky_decomposition(&corr, 2).unwrap();
        assert_eq!(chol.len(), 4);

        let z = vec![1.0, 0.0];
        let mut z_corr = vec![0.0; 2];
        apply_correlation(&chol, &z, &mut z_corr);
        assert!((z_corr[0] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cholesky_error_handling() {
        use finstack_core::math::linalg::CholeskyError;

        // Test that we get descriptive errors
        let invalid = vec![1.0, 1.01, 1.01, 1.0];
        match cholesky_decomposition(&invalid, 2) {
            Err(CholeskyError::NotPositiveDefinite { diag, row }) => {
                assert!(diag < 0.0);
                assert!(row < 2);
            }
            _ => panic!("Expected NotPositiveDefinite error"),
        }
    }
}
