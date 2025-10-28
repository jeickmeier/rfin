//! MLMC estimator and result types.

use finstack_core::money::Money;

/// MLMC estimate with level-by-level breakdown.
#[derive(Clone, Debug)]
pub struct MlmcEstimate {
    /// Combined estimate across all levels
    pub mean: Money,
    /// Standard error
    pub stderr: f64,
    /// 95% confidence interval
    pub ci_95: (Money, Money),
    /// Total paths used (sum across levels)
    pub total_paths: usize,
    /// Number of levels
    pub num_levels: usize,
    /// Per-level contributions
    pub level_means: Vec<f64>,
    /// Per-level variances
    pub level_variances: Vec<f64>,
    /// Per-level path counts
    pub level_paths: Vec<usize>,
}

impl MlmcEstimate {
    /// Create a new MLMC estimate.
    pub fn new(
        mean: Money,
        stderr: f64,
        ci_95: (Money, Money),
        total_paths: usize,
        num_levels: usize,
        level_means: Vec<f64>,
        level_variances: Vec<f64>,
        level_paths: Vec<usize>,
    ) -> Self {
        Self {
            mean,
            stderr,
            ci_95,
            total_paths,
            num_levels,
            level_means,
            level_variances,
            level_paths,
        }
    }
    
    /// Get mean for a specific level.
    pub fn level_mean(&self, level: usize) -> Option<f64> {
        self.level_means.get(level).copied()
    }
    
    /// Get variance for a specific level.
    pub fn level_variance(&self, level: usize) -> Option<f64> {
        self.level_variances.get(level).copied()
    }
    
    /// Get path count for a specific level.
    pub fn level_path_count(&self, level: usize) -> Option<usize> {
        self.level_paths.get(level).copied()
    }
}

/// Compute optimal path allocation for MLMC.
///
/// Given variances Vℓ and costs Cℓ, computes optimal Nℓ to minimize
/// total cost subject to variance constraint.
///
/// # Formula
///
/// ```text
/// N_ℓ = (2/ε²) * [Σ_j √(V_j C_j)] * √(V_ℓ / C_ℓ)
/// ```
///
/// # Arguments
///
/// * `variances` - Variance of Yℓ = P_hℓ - P_hℓ₋₁ for each level
/// * `costs` - Computational cost for each level
/// * `target_variance` - Target overall variance (ε²)
///
/// # Returns
///
/// Optimal path counts for each level
pub fn optimal_allocation(
    variances: &[f64],
    costs: &[f64],
    target_variance: f64,
) -> Vec<usize> {
    assert_eq!(variances.len(), costs.len());
    let num_levels = variances.len();
    
    // Compute Σ √(V_ℓ C_ℓ)
    let sum_sqrt_vc: f64 = variances
        .iter()
        .zip(costs.iter())
        .map(|(&v, &c)| (v * c).sqrt())
        .sum();
    
    // Compute Nℓ for each level
    let mut path_counts = Vec::with_capacity(num_levels);
    for (&v, &c) in variances.iter().zip(costs.iter()) {
        let ratio = if c > 1e-10 { v / c } else { 0.0 };
        let n_ell = (2.0 / target_variance) * sum_sqrt_vc * ratio.sqrt();
        path_counts.push(n_ell.ceil() as usize);
    }
    
    path_counts
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_optimal_allocation() {
        // Simple test: equal variances and costs
        let variances = vec![1.0, 1.0, 1.0];
        let costs = vec![1.0, 2.0, 4.0]; // Cost doubles each level
        let target_var = 0.01;
        
        let paths = optimal_allocation(&variances, &costs, target_var);
        
        // Nℓ ∝ √(V/C), so with V constant:
        // N0 : N1 : N2 ≈ √(1/1) : √(1/2) : √(1/4) ≈ 1 : 0.707 : 0.5
        
        // All should be positive
        for (i, &n) in paths.iter().enumerate() {
            assert!(n > 0, "Level {} has zero paths", i);
        }
        
        // Higher levels (higher cost) should have fewer paths
        assert!(paths[0] > paths[1]);
        assert!(paths[1] > paths[2]);
        
        println!("Optimal allocation: {:?}", paths);
    }
    
    #[test]
    fn test_optimal_allocation_decreasing_variance() {
        // Realistic: variance decreases with level
        let variances = vec![1.0, 0.25, 0.0625]; // Vℓ ∝ 4^{-ℓ}
        let costs = vec![1.0, 2.0, 4.0];
        let target_var = 0.01;
        
        let paths = optimal_allocation(&variances, &costs, target_var);
        
        println!("Allocation with decreasing variance: {:?}", paths);
        
        // With Vℓ decreasing faster than Cℓ increases,
        // higher levels can have fewer paths
        for &n in &paths {
            assert!(n > 0);
        }
    }
}

