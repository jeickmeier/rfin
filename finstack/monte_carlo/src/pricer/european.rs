//! Convenience pricer for European-style payoffs under GBM dynamics.
//!
//! This module wraps [`crate::engine::McEngine`] for the common case of pricing
//! a European payoff under [`crate::process::gbm::GbmProcess`] with
//! [`crate::discretization::exact::ExactGbm`]. Use it when you want a compact
//! API and do not need custom process / discretization combinations.

use super::super::engine::{McEngine, McEngineConfig};
use super::super::results::MoneyEstimate;
use super::super::traits::Payoff;
use crate::discretization::exact::ExactGbm;
use crate::process::gbm::GbmProcess;
use crate::rng::philox::PhiloxRng;
use crate::time_grid::TimeGrid;
use finstack_core::currency::Currency;
use finstack_core::Result;

/// Configuration for [`EuropeanPricer`].
#[derive(Debug, Clone)]
pub struct EuropeanPricerConfig {
    /// Number of Monte Carlo paths.
    pub num_paths: usize,
    /// Root RNG seed for deterministic replay.
    pub seed: u64,
    /// Whether to request parallel execution.
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
    /// Create a configuration with the given path count.
    pub fn new(num_paths: usize) -> Self {
        Self {
            num_paths,
            ..Default::default()
        }
    }

    /// Override the RNG seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Enable or disable parallel execution.
    ///
    /// If the crate is built without the `parallel` feature the underlying
    /// engine falls back to serial execution.
    pub fn with_parallel(mut self, parallel: bool) -> Self {
        self.use_parallel = parallel;
        self
    }
}

/// Compact GBM-only pricer for European-style contracts.
///
/// The pricer always uses exact GBM transitions and delegates the simulation
/// loop to [`crate::engine::McEngine`].
pub struct EuropeanPricer {
    config: EuropeanPricerConfig,
}

impl EuropeanPricer {
    /// Create a pricer from an explicit configuration.
    pub fn new(config: EuropeanPricerConfig) -> Self {
        Self { config }
    }

    /// Price a European-style payoff under GBM.
    ///
    /// # Arguments
    ///
    /// * `process` - GBM process supplying the risk-neutral drift and volatility.
    /// * `initial_spot` - Spot level at time `0`.
    /// * `time_to_maturity` - Maturity in years.
    /// * `num_steps` - Number of time-grid steps between `0` and maturity.
    /// * `payoff` - European-style payoff evaluated at `maturity_step = num_steps`.
    /// * `currency` - Currency for the returned estimate.
    /// * `discount_factor` - Present-value multiplier for the payoff horizon.
    ///
    /// # Returns
    ///
    /// A discounted Monte Carlo estimate in `currency`.
    ///
    /// # Errors
    ///
    /// Returns an error when the uniform time grid is invalid or when the
    /// underlying engine rejects the runtime configuration.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use finstack_core::currency::Currency;
    /// use finstack_monte_carlo::payoff::vanilla::EuropeanCall;
    /// use finstack_monte_carlo::pricer::european::{EuropeanPricer, EuropeanPricerConfig};
    /// use finstack_monte_carlo::process::gbm::GbmProcess;
    ///
    /// let pricer = EuropeanPricer::new(
    ///     EuropeanPricerConfig::new(25_000)
    ///         .with_seed(19)
    ///         .with_parallel(false),
    /// );
    /// let process = GbmProcess::with_params(0.03, 0.01, 0.20);
    /// let payoff = EuropeanCall::new(100.0, 1.0, 252);
    /// let discount_factor = (-0.03_f64).exp();
    ///
    /// let result = pricer
    ///     .price(&process, 100.0, 1.0, 252, &payoff, Currency::USD, discount_factor)
    ///     .expect("pricing should succeed");
    ///
    /// assert!(result.mean.amount().is_finite());
    /// ```
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
            path_capture: crate::engine::PathCaptureConfig::default(),
            antithetic: false,
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

    /// Borrow the current pricer configuration.
    pub fn config(&self) -> &EuropeanPricerConfig {
        &self.config
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::payoff::vanilla::EuropeanCall;
    use crate::process::gbm::GbmParams;

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
            .expect("should succeed");

        // Should get a reasonable option value
        assert!(result.mean.amount() > 0.0);
        assert!(result.mean.amount() < 50.0); // Sanity check
        assert_eq!(result.num_paths, 1000);
    }

    #[cfg(feature = "slow")]
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
            .expect("should succeed");

        // ATM call with σ=20%, T=1y should have positive value
        assert!(result.mean.amount() > 5.0);
    }

    #[cfg(feature = "slow")]
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
            .expect("should succeed");

        // Should be close to intrinsic value of 50
        assert!((result.mean.amount() - 50.0).abs() < 5.0);
    }
}
