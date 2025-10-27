//! Generic path-dependent option pricer with event scheduling.
//!
//! Handles payoffs that depend on the entire price path (Asians, barriers, lookbacks)
//! with flexible event scheduling.

use super::super::discretization::exact::ExactGbm;
use super::super::engine::{McEngine, McEngineConfig};
use super::super::process::gbm::GbmProcess;
use super::super::results::MoneyEstimate;
use super::super::rng::philox::PhiloxRng;
use super::super::time_grid::TimeGrid;
use super::super::traits::Payoff;
use finstack_core::currency::Currency;
use finstack_core::Result;

/// Configuration for path-dependent option pricing.
#[derive(Clone, Debug)]
pub struct PathDependentPricerConfig {
    /// Number of Monte Carlo paths
    pub num_paths: usize,
    /// Random seed
    pub seed: u64,
    /// Use parallel execution
    pub use_parallel: bool,
    /// Chunk size for parallel execution
    pub chunk_size: usize,
}

impl Default for PathDependentPricerConfig {
    fn default() -> Self {
        Self {
            num_paths: 100_000,
            seed: 42,
            use_parallel: cfg!(feature = "parallel"),
            chunk_size: 1000,
        }
    }
}

impl PathDependentPricerConfig {
    /// Create a new configuration.
    pub fn new(num_paths: usize) -> Self {
        Self {
            num_paths,
            ..Default::default()
        }
    }

    /// Set random seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Enable/disable parallel execution.
    pub fn with_parallel(mut self, parallel: bool) -> Self {
        self.use_parallel = parallel;
        self
    }

    /// Set chunk size.
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }
}

/// Path-dependent option pricer.
///
/// Prices options that depend on the path history (Asians, barriers, lookbacks).
///
/// # Example
///
/// ```rust,ignore
/// use finstack_valuations::instruments::common::mc::prelude::*;
/// use finstack_core::currency::Currency;
///
/// let pricer = PathDependentPricer::new(PathDependentPricerConfig::default());
///
/// // Arithmetic Asian call with monthly fixings
/// let fixing_steps = (0..=12).map(|i| i * 21).collect();
/// let asian = AsianCall::new(100.0, 1.0, AveragingMethod::Arithmetic, fixing_steps);
///
/// let gbm = GbmProcess::with_params(0.05, 0.02, 0.2);
///
/// let result = pricer.price(
///     &gbm,
///     100.0,  // initial spot
///     1.0,    // time to maturity
///     252,    // num steps
///     &asian,
///     Currency::USD,
///     0.95,   // discount factor
/// )?;
/// ```
pub struct PathDependentPricer {
    config: PathDependentPricerConfig,
}

impl PathDependentPricer {
    /// Create a new path-dependent pricer.
    pub fn new(config: PathDependentPricerConfig) -> Self {
        Self { config }
    }

    /// Price a path-dependent option.
    ///
    /// # Arguments
    ///
    /// * `process` - GBM process
    /// * `initial_spot` - Initial spot price
    /// * `time_to_maturity` - Time to maturity in years
    /// * `num_steps` - Number of time steps
    /// * `payoff` - Path-dependent payoff
    /// * `currency` - Currency for result
    /// * `discount_factor` - Discount factor to maturity
    #[allow(clippy::too_many_arguments)]
    pub fn price<P>(
        &self,
        process: &GbmProcess,
        initial_spot: f64,
        time_to_maturity: f64,
        num_steps: usize,
        payoff: &P,
        currency: Currency,
        discount_factor: f64,
    ) -> Result<MoneyEstimate>
    where
        P: Payoff,
    {
        // Create time grid
        let time_grid = TimeGrid::uniform(time_to_maturity, num_steps)?;

        // Create MC engine
        let engine_config = McEngineConfig {
            num_paths: self.config.num_paths,
            seed: self.config.seed,
            time_grid,
            target_ci_half_width: None,
            use_parallel: self.config.use_parallel,
            chunk_size: self.config.chunk_size,
        };
        let engine = McEngine::new(engine_config);

        // Create RNG and discretization
        let rng = PhiloxRng::new(self.config.seed);
        let disc = ExactGbm::new();

        // Initial state
        let initial_state = vec![initial_spot];

        // Price using engine
        engine.price(
            &rng,
            process,
            &disc,
            &initial_state,
            payoff,
            currency,
            discount_factor,
        )
    }

    /// Get configuration.
    pub fn config(&self) -> &PathDependentPricerConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::payoff::asian::{AsianCall, AveragingMethod};
    use super::super::super::payoff::lookback::LookbackCall;
    use super::super::super::process::gbm::GbmParams;
    use super::*;

    #[test]
    fn test_path_dependent_pricer_asian() {
        let config = PathDependentPricerConfig::new(10_000)
            .with_seed(42)
            .with_parallel(false);
        let pricer = PathDependentPricer::new(config);

        let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));

        // Monthly fixings
        let fixing_steps: Vec<usize> = (0..=12).map(|i| i * 21).collect();
        let asian = AsianCall::new(100.0, 1.0, AveragingMethod::Arithmetic, fixing_steps);

        let result = pricer
            .price(&gbm, 100.0, 1.0, 252, &asian, Currency::USD, 1.0)
            .unwrap();

        // Should get reasonable Asian option value
        assert!(result.mean.amount() > 0.0);
        assert!(result.mean.amount() < 20.0);
    }

    #[test]
    fn test_path_dependent_pricer_lookback() {
        let config = PathDependentPricerConfig::new(10_000)
            .with_seed(42)
            .with_parallel(false);
        let pricer = PathDependentPricer::new(config);

        let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 0.3));
        let lookback = LookbackCall::new(100.0, 1.0, 252);

        let result = pricer
            .price(&gbm, 100.0, 1.0, 252, &lookback, Currency::USD, 1.0)
            .unwrap();

        // Lookback should have positive value
        assert!(result.mean.amount() > 0.0);
    }
}
