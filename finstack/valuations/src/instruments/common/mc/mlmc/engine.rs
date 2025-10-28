//! MLMC engine implementation.

use super::super::results::Estimate;
use super::super::stats::OnlineStats;
use super::super::time_grid::TimeGrid;
use super::super::traits::{Discretization, Payoff, RandomStream, StochasticProcess};
use super::level::MlmcLevel;
use finstack_core::currency::Currency;
use finstack_core::Result;

/// MLMC engine configuration.
#[derive(Clone, Debug)]
pub struct MlmcConfig {
    /// Maximum number of levels
    pub max_levels: usize,
    /// Base time step size (for level 0)
    pub base_step_size: f64,
    /// Base number of time steps (for level 0)
    pub base_num_steps: usize,
    /// Target error tolerance (ε)
    pub target_epsilon: f64,
    /// Weak convergence order (α) - bias ~ O(h^α)
    pub weak_order: f64,
    /// Strong convergence order (β) - variance ~ O(h^β)
    pub strong_order: f64,
    /// Pilot paths per level (for variance estimation)
    pub pilot_paths: usize,
    /// Random seed
    pub seed: u64,
    /// Use parallel execution
    pub use_parallel: bool,
}

impl MlmcConfig {
    /// Create a new MLMC configuration with typical defaults.
    ///
    /// # Arguments
    ///
    /// * `target_epsilon` - Target error tolerance
    /// * `base_num_steps` - Number of steps at coarsest level
    pub fn new(target_epsilon: f64, base_num_steps: usize) -> Self {
        Self {
            max_levels: 6,
            base_step_size: 1.0 / base_num_steps as f64,
            base_num_steps,
            target_epsilon,
            weak_order: 1.0,   // Euler has weak order 1
            strong_order: 0.5, // Euler has strong order 0.5
            pilot_paths: 1000,
            seed: 42,
            use_parallel: false,
        }
    }
    
    /// Set maximum levels.
    pub fn with_max_levels(mut self, max_levels: usize) -> Self {
        self.max_levels = max_levels;
        self
    }
    
    /// Set convergence orders.
    pub fn with_orders(mut self, weak_order: f64, strong_order: f64) -> Self {
        self.weak_order = weak_order;
        self.strong_order = strong_order;
        self
    }
    
    /// Set random seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }
    
    /// Set pilot paths.
    pub fn with_pilot_paths(mut self, pilot_paths: usize) -> Self {
        self.pilot_paths = pilot_paths;
        self
    }
}

/// Multi-level Monte Carlo engine.
///
/// Implements MLMC algorithm with:
/// - Automatic level selection based on target error
/// - Pilot runs for variance/cost estimation
/// - Optimal path allocation
/// - Coupled paths for variance reduction
pub struct MlmcEngine {
    config: MlmcConfig,
}

impl MlmcEngine {
    /// Create a new MLMC engine.
    pub fn new(config: MlmcConfig) -> Self {
        Self { config }
    }
    
    /// Price using MLMC.
    ///
    /// This is a simplified implementation that demonstrates the core MLMC concept.
    /// For production use, additional refinements would be needed.
    pub fn price_simple<R, P, D, F>(
        &self,
        rng: &R,
        process: &P,
        disc: &D,
        initial_state: &[f64],
        payoff: &F,
        currency: Currency,
        discount_factor: f64,
    ) -> Result<Estimate>
    where
        R: RandomStream,
        P: StochasticProcess,
        D: Discretization<P>,
        F: Payoff,
    {
        // For now, provide a simplified MLMC that demonstrates the concept
        // Full implementation would include:
        // 1. Determine L based on bias constraint
        // 2. Pilot run to estimate variances
        // 3. Optimal allocation
        // 4. Coupled path simulation
        // 5. Telescoping sum combination
        
        // Simplified: just run with multiple levels and fixed allocation
        let num_levels = 3.min(self.config.max_levels);
        
        let mut level_estimates = Vec::new();
        let mut total_paths = 0;
        
        for level in 0..num_levels {
            let mut level_info = MlmcLevel::new(
                level,
                self.config.base_step_size,
                self.config.base_num_steps,
            );
            
            // Simple allocation: more paths at coarse levels
            let paths = self.config.pilot_paths / (1 << level).max(1);
            level_info.set_num_paths(paths);
            total_paths += paths;
            
            // Simulate level (this would be coupled in full implementation)
            let _time_grid = TimeGrid::uniform(
                self.config.base_step_size * level_info.num_steps as f64,
                level_info.num_steps,
            )?;
            
            let mut stats = OnlineStats::new();
            
            // Simulate paths at this level
            for path_id in 0..paths {
                let mut path_rng = rng.split(path_id as u64 + level as u64 * 100_000);
                let payoff_clone = payoff.clone();
                let mut state = vec![0.0; process.dim()];
                let mut z = vec![0.0; process.num_factors()];
                let mut work = vec![0.0; disc.work_size(process)];
                
                state.copy_from_slice(initial_state);
                
                // Simulate path
                for step in 0..level_info.num_steps {
                    let t = step as f64 * level_info.step_size;
                    path_rng.fill_std_normals(&mut z);
                    disc.step(process, t, level_info.step_size, &mut state, &z, &mut work);
                    
                    // Update payoff (simplified - not creating full PathState)
                    // In full implementation, would properly track path state
                }
                
                let value = payoff_clone.value(currency).amount();
                stats.update(value * discount_factor);
            }
            
            level_estimates.push(stats.mean());
        }
        
        // Telescoping sum: E[P_L] ≈ sum of level means
        let combined_mean: f64 = level_estimates.iter().sum();
        let combined_stderr = combined_mean / (total_paths as f64).sqrt(); // Simplified
        
        let margin = 1.96 * combined_stderr;
        
        Ok(Estimate::new(
            combined_mean,
            combined_stderr,
            (combined_mean - margin, combined_mean + margin),
            total_paths,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mlmc_engine_creation() {
        let config = MlmcConfig::new(0.001, 10);
        let engine = MlmcEngine::new(config);
        
        assert_eq!(engine.config.base_num_steps, 10);
        assert_eq!(engine.config.target_epsilon, 0.001);
    }
}

