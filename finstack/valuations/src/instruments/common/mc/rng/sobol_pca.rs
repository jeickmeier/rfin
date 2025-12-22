//! PCA-based dimension ordering for Sobol sequences.
//!
//! Reduces effective dimension for quasi-Monte Carlo by reordering
//! dimensions based on principal component analysis of the correlation matrix.
//!
//! # Motivation
//!
//! Sobol sequences work best when:
//! - Important dimensions come first (assigned best-quality Sobol points)
//! - Effective dimension is low
//!
//! For correlated assets, PCA identifies the most important directions
//! (eigenvectors with largest eigenvalues) and assigns them to early dimensions.
//!
//! # Algorithm
//!
//! 1. Eigen-decomposition of correlation matrix: ρ = Q Λ Q'
//! 2. Sort eigenvectors by eigenvalue (descending)
//! 3. Reorder Sobol dimensions to match sorted eigenvectors
//!
//! # Example
//!
//! ```rust,no_run
//! use finstack_valuations::instruments::common::mc::rng::sobol_pca::pca_ordering;
//!
//! // 3-asset basket with correlation
//! let correlation = vec![
//!     1.0, 0.8, 0.6,
//!     0.8, 1.0, 0.7,
//!     0.6, 0.7, 1.0,
//! ];
//!
//! let (_eigenvalues, _eigenvectors, permutation) = pca_ordering(&correlation, 3);
//!
//! // permutation maps back to original asset order
//! ```
//!
//! Reference: Acworth et al. (1998) - "A comparison of some MC and QMC techniques"

use nalgebra::{DMatrix, SymmetricEigen};

/// Perform PCA on correlation matrix to find optimal dimension ordering.
///
/// # Arguments
///
/// * `correlation` - Correlation matrix (n x n, row-major)
/// * `num_factors` - Number of factors
///
/// # Returns
///
/// (eigenvalues, eigenvectors, permutation)
/// - eigenvalues: sorted in descending order
/// - eigenvectors: columns are eigenvectors (sorted by eigenvalue)
/// - permutation: maps original dimensions to PCA-ordered dimensions
pub fn pca_ordering(correlation: &[f64], num_factors: usize) -> (Vec<f64>, Vec<f64>, Vec<usize>) {
    assert_eq!(
        correlation.len(),
        num_factors * num_factors,
        "Correlation matrix must be n x n"
    );

    // Convert to nalgebra matrix
    let corr_matrix = DMatrix::from_row_slice(num_factors, num_factors, correlation);

    // Ensure symmetric (average with transpose for numerical stability)
    let corr_sym = (&corr_matrix + &corr_matrix.transpose()) / 2.0;

    // Eigen-decomposition
    let eigen = SymmetricEigen::new(corr_sym);

    // Extract eigenvalues and eigenvectors
    let mut eigenvalues: Vec<(usize, f64)> = eigen
        .eigenvalues
        .iter()
        .enumerate()
        .map(|(i, &val)| (i, val))
        .collect();

    // Sort by eigenvalue (descending) using total_cmp for safe float comparison
    eigenvalues.sort_by(|a, b| b.1.total_cmp(&a.1));

    // Build permutation (original index → sorted index)
    let permutation: Vec<usize> = eigenvalues.iter().map(|(i, _)| *i).collect();

    // Extract sorted eigenvalues
    let sorted_eigenvalues: Vec<f64> = eigenvalues.iter().map(|(_, val)| *val).collect();

    // Extract eigenvectors (column-major from nalgebra)
    let eigenvectors: Vec<f64> = eigen.eigenvectors.as_slice().to_vec();

    (sorted_eigenvalues, eigenvectors, permutation)
}

/// Compute effective dimension for QMC.
///
/// Effective dimension measures how many dimensions contribute significantly
/// to variance. Lower effective dimension → better QMC performance.
///
/// # Formula
///
/// d_eff = (Σ λ_i)² / Σ λ_i²
///
/// where λ_i are eigenvalues of correlation matrix.
pub fn effective_dimension(eigenvalues: &[f64]) -> f64 {
    let sum = eigenvalues.iter().sum::<f64>();
    let sum_sq = eigenvalues.iter().map(|&x| x * x).sum::<f64>();

    if sum_sq > 1e-10 {
        sum * sum / sum_sq
    } else {
        eigenvalues.len() as f64
    }
}

/// Apply PCA transformation to random shocks.
///
/// Transforms shocks from PCA space back to original asset space.
///
/// # Arguments
///
/// * `z_pca` - Shocks in PCA-ordered space
/// * `eigenvectors` - Eigenvector matrix from PCA
/// * `permutation` - Permutation from PCA ordering
/// * `z_out` - Output shocks in original asset order
pub fn transform_pca_to_assets(
    z_pca: &[f64],
    eigenvectors: &[f64],
    permutation: &[usize],
    z_out: &mut [f64],
) {
    let n = z_pca.len();
    assert_eq!(eigenvectors.len(), n * n);
    assert_eq!(z_out.len(), n);

    // z_asset = Q * z_pca (where Q is eigenvector matrix)
    // Need to apply permutation to account for sorted order

    for i in 0..n {
        let mut sum = 0.0;
        for j in 0..n {
            let sorted_j = permutation[j];
            sum += eigenvectors[i * n + sorted_j] * z_pca[j];
        }
        z_out[i] = sum;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pca_identity_matrix() {
        // Identity matrix: eigenvalues all 1, any order is optimal
        let correlation = vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];

        let (eigenvalues, _, _) = pca_ordering(&correlation, 3);

        // All eigenvalues should be 1
        for &val in &eigenvalues {
            assert!((val - 1.0).abs() < 0.01);
        }

        // Effective dimension should be 3
        let d_eff = effective_dimension(&eigenvalues);
        assert!((d_eff - 3.0).abs() < 0.1);
    }

    #[test]
    fn test_pca_high_correlation() {
        // High correlation: effective dimension should be low
        let correlation = vec![1.0, 0.9, 0.9, 0.9, 1.0, 0.9, 0.9, 0.9, 1.0];

        let (eigenvalues, _, _) = pca_ordering(&correlation, 3);

        println!("Eigenvalues (high correlation): {:?}", eigenvalues);

        // First eigenvalue should dominate
        assert!(eigenvalues[0] > 2.0); // Should be ~2.7
        assert!(eigenvalues[1] < 1.0); // Others smaller

        // Effective dimension should be low
        let d_eff = effective_dimension(&eigenvalues);
        println!("Effective dimension: {:.2}", d_eff);
        assert!(d_eff < 2.0);
    }

    #[test]
    fn test_effective_dimension_bounds() {
        // Effective dimension should be between 1 and n
        let eigenvalues_equal = vec![1.0, 1.0, 1.0];
        let d_eff = effective_dimension(&eigenvalues_equal);
        assert!((d_eff - 3.0).abs() < 0.1);

        // Single dominant eigenvalue
        let eigenvalues_dominant = vec![2.9, 0.05, 0.05];
        let d_eff = effective_dimension(&eigenvalues_dominant);
        assert!((1.0..=3.0).contains(&d_eff));
        assert!(d_eff < 1.5); // Should be close to 1
    }

    #[test]
    fn test_pca_transformation() {
        // Simple 2D test
        let correlation = vec![1.0, 0.5, 0.5, 1.0];

        let (_eigenvalues, eigenvectors, permutation) = pca_ordering(&correlation, 2);

        // Transform identity shocks through PCA
        let z_pca = vec![1.0, 0.0]; // Shock in first PC direction
        let mut z_assets = vec![0.0; 2];

        transform_pca_to_assets(&z_pca, &eigenvectors, &permutation, &mut z_assets);

        // Result should be in asset space
        println!("PCA transform: {:?} -> {:?}", z_pca, z_assets);
        assert!(z_assets[0].is_finite());
        assert!(z_assets[1].is_finite());
    }
}
