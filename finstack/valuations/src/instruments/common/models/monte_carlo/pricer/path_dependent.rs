//! Generic path-dependent option pricer with event scheduling.
//!
//! Handles payoffs that depend on the entire price path (Asians, barriers, lookbacks)
//! with flexible event scheduling.

use super::super::engine::{McEngine, McEngineConfig, PathCaptureConfig};
use super::super::results::{MoneyEstimate, MonteCarloResult};
use super::super::traits::Payoff;
use crate::instruments::common::mc::discretization::exact::ExactGbm;
use crate::instruments::common::mc::process::gbm::GbmProcess;
use crate::instruments::common::mc::process::metadata::ProcessMetadata;
use crate::instruments::common::mc::rng::philox::PhiloxRng;
#[cfg(feature = "mc")]
use crate::instruments::common::mc::rng::sobol::SobolRng;
use crate::instruments::common::mc::time_grid::TimeGrid;
use crate::instruments::common::mc::traits::StochasticProcess;
use crate::instruments::common::mc::traits::{Discretization, RandomStream};
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
    /// Path capture configuration
    pub path_capture: PathCaptureConfig,
    /// Steps per year for time discretization (default: 252.0)
    pub steps_per_year: f64,
    /// Minimum number of steps regardless of maturity (default: 8)
    pub min_steps: usize,
    /// Use Sobol quasi-random sequence (default: false)
    pub use_sobol: bool,
    /// Enable antithetic variates (default: false)
    pub antithetic: bool,
    /// Use Brownian bridge ordering for Sobol (QMC) paths (default: false)
    pub use_brownian_bridge: bool,
}

impl Default for PathDependentPricerConfig {
    fn default() -> Self {
        Self {
            num_paths: 100_000,
            seed: 42,
            use_parallel: cfg!(feature = "parallel"),
            chunk_size: 1000,
            path_capture: PathCaptureConfig::default(),
            steps_per_year: 252.0,
            min_steps: 8,
            use_sobol: false,
            antithetic: false,
            use_brownian_bridge: false,
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

    /// Set path capture configuration.
    pub fn with_path_capture(mut self, config: PathCaptureConfig) -> Self {
        self.path_capture = config;
        self
    }

    /// Enable path capture for all paths.
    pub fn capture_all_paths(mut self) -> Self {
        self.path_capture = PathCaptureConfig::all();
        self
    }

    /// Enable path capture for a sample.
    pub fn capture_sample_paths(mut self, count: usize, seed: u64) -> Self {
        self.path_capture = PathCaptureConfig::sample(count, seed);
        self
    }

    /// Set steps per year for time discretization.
    pub fn with_steps_per_year(mut self, steps: f64) -> Self {
        self.steps_per_year = steps;
        self
    }

    /// Set minimum number of steps.
    pub fn with_min_steps(mut self, min_steps: usize) -> Self {
        self.min_steps = min_steps;
        self
    }

    /// Enable Sobol quasi-random sequence.
    pub fn with_sobol(mut self, use_sobol: bool) -> Self {
        self.use_sobol = use_sobol;
        if use_sobol {
            // Default to enabling Brownian bridge when using Sobol unless explicitly turned off later
            self.use_brownian_bridge = true;
        }
        self
    }

    /// Enable antithetic variates.
    pub fn with_antithetic(mut self, antithetic: bool) -> Self {
        self.antithetic = antithetic;
        self
    }

    /// Enable Brownian bridge (only used with Sobol RNG).
    pub fn with_brownian_bridge(mut self, enable: bool) -> Self {
        self.use_brownian_bridge = enable;
        self
    }
}

/// Path-dependent option pricer.
///
/// Prices options that depend on the path history (Asians, barriers, lookbacks).
///
/// See unit tests and `examples/` for usage.
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
            path_capture: self.config.path_capture.clone(),
            antithetic: false,
        };

        // If path capture is enabled, use price_with_capture
        if engine_config.path_capture.enabled {
            let engine = McEngine::new(engine_config);
            // Path capture path uses Philox for determinism; Sobol + capture not yet supported
            let rng = PhiloxRng::new(self.config.seed);
            let disc = ExactGbm::new();
            let initial_state = vec![initial_spot];

            // Get process metadata
            let process_params = process.metadata();

            // Price with path capture
            let result = engine.price_with_capture(
                &rng,
                process,
                &disc,
                &initial_state,
                payoff,
                currency,
                discount_factor,
                process_params,
            )?;

            // Return just the estimate (paths are dropped)
            Ok(result.estimate)
        } else {
            // Use regular pricing without path capture
            let engine = McEngine::new(engine_config);
            let disc = ExactGbm::new();
            let initial_state = vec![initial_spot];

            // RNG selection
            if self.config.use_sobol {
                #[cfg(feature = "mc")]
                {
                    let rng = SobolRng::new(1, self.config.seed);
                    if self.config.antithetic {
                        // Antithetic pricing path
                        use crate::instruments::common::models::monte_carlo::variance_reduction::antithetic::{antithetic_price, AntitheticConfig};
                        let time_grid =
                            crate::instruments::common::mc::time_grid::TimeGrid::uniform(
                                time_to_maturity,
                                num_steps,
                            )?;
                        let mut rng_clone = rng.clone();
                        let cfg = AntitheticConfig {
                            num_pairs: self.config.num_paths / 2,
                            time_grid: &time_grid,
                            currency,
                            discount_factor,
                        };
                        let stats = antithetic_price(
                            &mut rng_clone,
                            process,
                            &disc,
                            &initial_state,
                            payoff,
                            &cfg,
                        );
                        let est = crate::instruments::common::mc::estimate::Estimate::new(
                            stats.mean(),
                            stats.stderr(),
                            stats.ci_95(),
                            stats.count(),
                        );
                        Ok(MoneyEstimate::from_estimate(est, currency))
                    } else if self.config.use_brownian_bridge {
                        use crate::instruments::common::mc::online_stats::OnlineStats;
                        use crate::instruments::common::mc::rng::brownian_bridge::BrownianBridge;

                        let time_grid =
                            crate::instruments::common::mc::time_grid::TimeGrid::uniform(
                                time_to_maturity,
                                num_steps,
                            )?;
                        let dt = time_grid.dt(0);
                        let bridge = BrownianBridge::new(num_steps);
                        let mut stats = OnlineStats::new();

                        let mut state = vec![0.0; process.dim()];
                        let mut work = vec![0.0; disc.work_size(process)];
                        let mut z_step = vec![0.0; process.num_factors()];
                        let mut z_bridge = vec![0.0; num_steps];
                        let mut w_path = vec![f64::NAN; num_steps + 1];

                        let mut sobol = rng.clone();
                        for _path_id in 0..self.config.num_paths {
                            let mut payoff_clone = payoff.clone();
                            payoff_clone.reset();
                            state.copy_from_slice(&initial_state);

                            sobol.fill_std_normals(&mut z_bridge);
                            bridge.construct_path(&z_bridge, &mut w_path, dt);

                            let mut path_state =
                                crate::instruments::common::mc::traits::PathState::new(0, 0.0);
                            path_state.set(
                                crate::instruments::common::mc::traits::state_keys::SPOT,
                                state[0],
                            );
                            payoff_clone.on_event(&mut path_state);

                            for step in 0..num_steps {
                                let t = time_grid.time(step);
                                let dt = time_grid.dt(step);
                                let w_inc = (w_path[step + 1] - w_path[step]) / dt.sqrt();
                                z_step[0] = w_inc;
                                disc.step(process, t, dt, &mut state, &z_step, &mut work);
                                path_state.step = step + 1;
                                path_state.time = t + dt;
                                path_state.set(
                                    crate::instruments::common::mc::traits::state_keys::SPOT,
                                    state[0],
                                );
                                payoff_clone.on_event(&mut path_state);
                            }

                            let payoff_money = payoff_clone.value(currency);
                            stats.update(payoff_money.amount() * discount_factor);
                        }

                        let est = crate::instruments::common::mc::estimate::Estimate::new(
                            stats.mean(),
                            stats.stderr(),
                            stats.ci_95(),
                            stats.count(),
                        )
                        .with_std_dev(stats.std_dev());
                        Ok(MoneyEstimate::from_estimate(est, currency))
                    } else {
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
                }
                #[cfg(not(feature = "mc"))]
                {
                    let rng = PhiloxRng::new(self.config.seed);
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
            } else {
                let rng = PhiloxRng::new(self.config.seed);
                if self.config.antithetic {
                    use crate::instruments::common::models::monte_carlo::variance_reduction::antithetic::{antithetic_price, AntitheticConfig};
                    let time_grid = crate::instruments::common::mc::time_grid::TimeGrid::uniform(
                        time_to_maturity,
                        num_steps,
                    )?;
                    let mut rng_clone = rng.clone();
                    let cfg = AntitheticConfig {
                        num_pairs: self.config.num_paths / 2,
                        time_grid: &time_grid,
                        currency,
                        discount_factor,
                    };
                    let stats = antithetic_price(
                        &mut rng_clone,
                        process,
                        &disc,
                        &initial_state,
                        payoff,
                        &cfg,
                    );
                    let est = crate::instruments::common::mc::estimate::Estimate::new(
                        stats.mean(),
                        stats.stderr(),
                        stats.ci_95(),
                        stats.count(),
                    );
                    Ok(MoneyEstimate::from_estimate(est, currency))
                } else {
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
            }
        }
    }

    /// Price with full Monte Carlo result (including captured paths if enabled).
    ///
    /// This method returns a `MonteCarloResult` which includes the estimate
    /// and optionally captured paths based on the pricer configuration.
    #[allow(clippy::too_many_arguments)]
    pub fn price_with_paths<P>(
        &self,
        process: &GbmProcess,
        initial_spot: f64,
        time_to_maturity: f64,
        num_steps: usize,
        payoff: &P,
        currency: Currency,
        discount_factor: f64,
    ) -> Result<MonteCarloResult>
    where
        P: Payoff,
    {
        // Create time grid
        let time_grid = TimeGrid::uniform(time_to_maturity, num_steps)?;

        // Create MC engine with path capture
        let engine_config = McEngineConfig {
            num_paths: self.config.num_paths,
            seed: self.config.seed,
            time_grid,
            target_ci_half_width: None,
            use_parallel: self.config.use_parallel,
            chunk_size: self.config.chunk_size,
            path_capture: self.config.path_capture.clone(),
            antithetic: false,
        };
        let engine = McEngine::new(engine_config);

        // Create RNG and discretization
        let rng = PhiloxRng::new(self.config.seed);
        let disc = ExactGbm::new();

        // Initial state
        let initial_state = vec![initial_spot];

        // Get process metadata
        let process_params = process.metadata();

        // Price with path capture support
        engine.price_with_capture(
            &rng,
            process,
            &disc,
            &initial_state,
            payoff,
            currency,
            discount_factor,
            process_params,
        )
    }

    /// Price and compute LRM Greeks (delta, vega) for GBM using captured paths.
    ///
    /// This uses the Likelihood Ratio Method, deriving W_T from terminal spots
    /// under GBM: W_T = (ln(S_T/S_0) - (r - q - 0.5 σ^2) T) / σ
    #[allow(clippy::too_many_arguments)]
    pub fn price_with_lrm_greeks<P>(
        &self,
        process: &GbmProcess,
        initial_spot: f64,
        time_to_maturity: f64,
        num_steps: usize,
        payoff: &P,
        currency: Currency,
        discount_factor: f64,
        rate: f64,
        dividend_yield: f64,
        volatility: f64,
    ) -> Result<(MoneyEstimate, Option<(f64, f64)>)>
    where
        P: Payoff,
    {
        // Force path capture to get terminal spots and final discounted payoff values
        let time_grid = TimeGrid::uniform(time_to_maturity, num_steps)?;
        let engine_config = McEngineConfig {
            num_paths: self.config.num_paths,
            seed: self.config.seed,
            time_grid,
            target_ci_half_width: None,
            use_parallel: self.config.use_parallel,
            chunk_size: self.config.chunk_size,
            path_capture: PathCaptureConfig::all().with_payoffs(),
            antithetic: false,
        };
        let engine = McEngine::new(engine_config);

        let rng = crate::instruments::common::mc::rng::philox::PhiloxRng::new(self.config.seed);
        let disc = ExactGbm::new();
        let initial_state = vec![initial_spot];

        // Process metadata
        let process_params = process.metadata();

        let full = engine.price_with_capture(
            &rng,
            process,
            &disc,
            &initial_state,
            payoff,
            currency,
            discount_factor,
            process_params,
        )?;

        // Extract estimate and paths
        let estimate = full.estimate.clone();
        let paths = match &full.paths {
            Some(ds) => &ds.paths,
            None => return Ok((estimate, None)),
        };

        if paths.is_empty()
            || discount_factor <= 0.0
            || time_to_maturity <= 0.0
            || volatility <= 0.0
        {
            return Ok((estimate, None));
        }

        // Build undiscounted payoffs and W_T
        let mut payoffs: Vec<f64> = Vec::with_capacity(paths.len());
        let mut w_terminals: Vec<f64> = Vec::with_capacity(paths.len());
        let mu = rate - dividend_yield - 0.5 * volatility * volatility;
        for p in paths {
            // Final discounted payoff value is stored; un-discount it
            let undisc = p.final_value / discount_factor;
            payoffs.push(undisc);

            // Terminal spot from last point's state var
            if let Some(last) = p.terminal_point() {
                if let Some(s_t) = last.spot() {
                    let w_t = ((s_t / initial_spot).ln() - mu * time_to_maturity)
                        / (volatility * time_to_maturity.sqrt());
                    w_terminals.push(w_t);
                }
            }
        }

        if payoffs.len() == w_terminals.len() && !payoffs.is_empty() {
            use super::super::greeks::lrm::{lrm_delta, lrm_vega};
            let (delta, _) = lrm_delta(
                &payoffs,
                &w_terminals,
                initial_spot,
                volatility,
                time_to_maturity,
                discount_factor,
            );
            let (vega, _) = lrm_vega(
                &payoffs,
                &w_terminals,
                volatility,
                time_to_maturity,
                discount_factor,
            );
            Ok((estimate, Some((delta, vega))))
        } else {
            Ok((estimate, None))
        }
    }

    /// Get configuration.
    pub fn config(&self) -> &PathDependentPricerConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::common::mc::process::gbm::GbmParams;
    use crate::instruments::common::models::monte_carlo::payoff::asian::{
        AsianCall, AveragingMethod,
    };
    use crate::instruments::common::models::monte_carlo::payoff::lookback::LookbackCall;

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
