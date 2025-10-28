//! MLMC level management.
//!
//! Tracks variance, cost, and path counts for each MLMC level.

/// MLMC level statistics.
///
/// Each level ℓ has:
/// - Step size hℓ = h0/2^ℓ
/// - Number of steps nℓ = n0 * 2^ℓ
/// - Path count Nℓ (optimally allocated)
/// - Estimated variance Vℓ of correction Yℓ = P_hℓ - P_hℓ₋₁
/// - Computational cost Cℓ = Nℓ * nℓ
#[derive(Clone, Debug)]
pub struct MlmcLevel {
    /// Level index (0 = coarsest)
    pub level: usize,
    /// Time step size for this level
    pub step_size: f64,
    /// Number of time steps
    pub num_steps: usize,
    /// Number of paths to simulate at this level
    pub num_paths: usize,
    /// Estimated variance of correction term
    pub variance: Option<f64>,
    /// Estimated cost (paths × steps)
    pub cost: Option<f64>,
}

impl MlmcLevel {
    /// Create a new MLMC level.
    pub fn new(level: usize, base_step_size: f64, base_num_steps: usize) -> Self {
        let step_size = base_step_size / (1 << level) as f64;
        let num_steps = base_num_steps * (1 << level);
        
        Self {
            level,
            step_size,
            num_steps,
            num_paths: 0,
            variance: None,
            cost: None,
        }
    }
    
    /// Update variance estimate from pilot run.
    pub fn set_variance(&mut self, variance: f64) {
        self.variance = Some(variance);
        self.update_cost();
    }
    
    /// Update path count and recalculate cost.
    pub fn set_num_paths(&mut self, num_paths: usize) {
        self.num_paths = num_paths;
        self.update_cost();
    }
    
    /// Compute computational cost.
    fn update_cost(&mut self) {
        if self.num_paths > 0 {
            self.cost = Some((self.num_paths * self.num_steps) as f64);
        }
    }
    
    /// Get variance (or panic if not set).
    pub fn variance_value(&self) -> f64 {
        self.variance.expect("Variance not set for level")
    }
    
    /// Get cost (or panic if not set).
    pub fn cost_value(&self) -> f64 {
        self.cost.expect("Cost not set for level")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mlmc_level_step_sizes() {
        let base_step = 0.1;
        let base_steps = 10;
        
        let level0 = MlmcLevel::new(0, base_step, base_steps);
        assert_eq!(level0.step_size, 0.1);
        assert_eq!(level0.num_steps, 10);
        
        let level1 = MlmcLevel::new(1, base_step, base_steps);
        assert_eq!(level1.step_size, 0.05);
        assert_eq!(level1.num_steps, 20);
        
        let level2 = MlmcLevel::new(2, base_step, base_steps);
        assert_eq!(level2.step_size, 0.025);
        assert_eq!(level2.num_steps, 40);
    }
    
    #[test]
    fn test_mlmc_level_cost_calculation() {
        let mut level = MlmcLevel::new(1, 0.1, 10);
        level.set_num_paths(1000);
        
        // Cost = 1000 paths × 20 steps = 20,000
        assert_eq!(level.cost_value(), 20_000.0);
    }
}

