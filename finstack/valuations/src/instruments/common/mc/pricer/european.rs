//! European option pricer using Monte Carlo simulation.
//!
//! Provides pricing for European-style options under GBM dynamics.

use super::super::discretization::exact::ExactGbm;
use super::super::engine::{McEngine, McEngineConfig};
use super::super::process::gbm::GbmProcess;
use super::super::results::MoneyEstimate;
use super::super::rng::philox::PhiloxRng;
use super::super::time_grid::TimeGrid;
use super::super::traits::Payoff;
use finstack_core::currency::Currency;
use finstack_core::Result;

/// Configuration for European option pricing.
#[derive(Clone, Debug)]
pub struct EuropeanPricerConfig {
    /// Number of Monte Carlo paths
    pub num_paths: usize,
    /// Random seed
    pub seed: u64,
    /// Use parallel execution
    pub use_parallel: bool,
}

impl Default for EuropeanPricerConfig {
    fn default() -> Self {
        Self {
            num_paths: 100_000,
            seed: 42,
            use_parallel: cfg!(feature = "parallel"),
        }
    }
}

impl EuropeanPricerConfig {
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
}

/// European option pricer.
///
/// Prices European-style payoffs under Geometric Brownian Motion.
///
/// See unit tests and `examples/` for usage.
pub struct EuropeanPricer {
    config: EuropeanPricerConfig,
}

impl EuropeanPricer {
    /// Create a new European pricer.
    pub fn new(config: EuropeanPricerConfig) -> Self {
        Self { config }
    }

    /// Price a European option using Monte Carlo.
    ///
    /// # Arguments
    ///
    /// * `process` - GBM process (drift and volatility)
    /// * `initial_spot` - Initial spot price
    /// * `time_to_maturity` - Time to maturity in years
    /// * `num_steps` - Number of time steps
    /// * `payoff` - Payoff specification
    /// * `currency` - Currency for result
    /// * `discount_factor` - Discount factor to maturity
    ///
    /// # Returns
    ///
    /// Statistical estimate with mean, stderr, and confidence interval.
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
            chunk_size: 1000,
        };
        let engine = McEngine::new(engine_config);

        // Create RNG and discretization
        let rng = PhiloxRng::new(self.config.seed);
        let disc = ExactGbm::new();

        // Initial state (just spot price for 1D GBM)
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
    pub fn config(&self) -> &EuropeanPricerConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::payoff::vanilla::EuropeanCall;
    use super::super::super::process::gbm::GbmParams;
    use super::*;

    #[test]
    fn test_european_pricer_basic() {
        let config = EuropeanPricerConfig::new(1000)
            .with_seed(42)
            .with_parallel(false);
        let pricer = EuropeanPricer::new(config);

        let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 0.2));
        let call = EuropeanCall::new(100.0, 1.0, 10);

        let result = pricer
            .price(&gbm, 100.0, 1.0, 10, &call, Currency::USD, 0.95)
            .unwrap();

        // Should get a reasonable option value
        assert!(result.mean.amount() > 0.0);
        assert!(result.mean.amount() < 50.0); // Sanity check
        assert_eq!(result.num_paths, 1000);
    }

    #[test]
    fn test_european_pricer_atm_call() {
        // ATM call should have value > intrinsic value of 0
        let config = EuropeanPricerConfig::new(10000)
            .with_seed(42)
            .with_parallel(false);
        let pricer = EuropeanPricer::new(config);

        let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2));
        let call = EuropeanCall::new(100.0, 1.0, 252);

        let result = pricer
            .price(&gbm, 100.0, 1.0, 252, &call, Currency::USD, 1.0)
            .unwrap();

        // ATM call with σ=20%, T=1y should have positive value
        assert!(result.mean.amount() > 5.0);
    }

    #[test]
    fn test_european_pricer_deep_itm() {
        // Deep ITM call should be close to intrinsic value
        let config = EuropeanPricerConfig::new(10000)
            .with_seed(42)
            .with_parallel(false);
        let pricer = EuropeanPricer::new(config);

        let gbm = GbmProcess::new(GbmParams::new(0.0, 0.0, 0.01)); // Very low vol, no drift
        let call = EuropeanCall::new(50.0, 1.0, 100);

        let result = pricer
            .price(&gbm, 100.0, 1.0, 100, &call, Currency::USD, 1.0)
            .unwrap();

        // Should be close to intrinsic value of 50
        assert!((result.mean.amount() - 50.0).abs() < 5.0);
    }
}
