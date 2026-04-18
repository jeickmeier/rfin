//! Generic path-dependent option pricer with event scheduling.
//!
//! Handles payoffs that depend on the entire price path (Asians, barriers, lookbacks)
//! with flexible event scheduling.

use super::super::engine::{McEngine, McEngineConfig, PathCaptureConfig};
use super::super::results::{MoneyEstimate, MonteCarloResult};
use super::super::traits::Payoff;
use crate::captured_path_stats::apply_captured_path_statistics;
use crate::discretization::exact::ExactGbm;
use crate::estimate::Estimate;
use crate::online_stats::OnlineStats;
use crate::paths::{PathDataset, PathPoint, PathSamplingMethod, SimulatedPath};
use crate::process::gbm::GbmProcess;
use crate::process::metadata::ProcessMetadata;
#[cfg(feature = "mc")]
use crate::rng::brownian_bridge::BrownianBridge;
use crate::rng::philox::PhiloxRng;
#[cfg(feature = "mc")]
use crate::rng::sobol::{SobolRng, MAX_SOBOL_DIMENSION};
use crate::time_grid::TimeGrid;
use crate::traits::{Discretization, PathState, RandomStream, StochasticProcess};
use finstack_core::currency::Currency;
use finstack_core::{Error, Result};
use smallvec::SmallVec;

/// Configuration for path-dependent option pricing.
#[derive(Debug, Clone)]
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
            use_parallel: true,
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
        if parallel && self.use_sobol {
            tracing::warn!(
                "PathDependentPricer: Sobol sequences do not support stream splitting; \
                 automatically disabling parallel mode. Set use_parallel=false explicitly to suppress this warning."
            );
            self.use_parallel = false;
        }
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
            if self.use_parallel {
                tracing::warn!(
                    "PathDependentPricer: Sobol sequences do not support stream splitting; \
                     automatically disabling parallel mode. Set use_parallel=false explicitly to suppress this warning."
                );
                self.use_parallel = false;
            }
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

    /// Build a time grid from the configuration's step density and required event times.
    pub fn build_time_grid(
        &self,
        time_to_maturity: f64,
        required_times: &[f64],
    ) -> Result<TimeGrid> {
        TimeGrid::uniform_with_required_times(
            time_to_maturity,
            self.steps_per_year,
            self.min_steps,
            required_times,
        )
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

    #[cfg(feature = "mc")]
    fn validate_sobol_configuration(
        &self,
        time_grid: &TimeGrid,
        num_factors: usize,
    ) -> Result<usize> {
        if self.config.use_parallel {
            return Err(finstack_core::Error::Validation(
                "Sobol pricing requires serial execution in PathDependentPricer".to_string(),
            ));
        }

        if self.config.antithetic {
            return Err(finstack_core::Error::Validation(
                "Sobol pricing currently does not support antithetic variates".to_string(),
            ));
        }

        if let crate::engine::PathCaptureMode::Sample { count, .. } =
            self.config.path_capture.capture_mode
        {
            if self.config.path_capture.enabled && (count == 0 || count > self.config.num_paths) {
                return Err(finstack_core::Error::Validation(format!(
                    "Sobol path capture sample count must be between 1 and num_paths (got {count})"
                )));
            }
        }

        // Brownian bridge path construction allocates the leading Sobol
        // dimensions to terminal/midpoint increments of a single scalar
        // Brownian motion. With multi-factor processes the bridge would need
        // to be applied per-factor using the increment-covariance Cholesky,
        // which is not yet implemented. Reject the combination to prevent a
        // silently biased result.
        if self.config.use_brownian_bridge && num_factors != 1 {
            return Err(finstack_core::Error::Validation(format!(
                "Brownian-bridge path construction is only supported for single-factor \
                 processes, but the supplied process reports num_factors={num_factors}. \
                 Disable `use_brownian_bridge` or use a single-factor process."
            )));
        }

        let sobol_dimension = if self.config.use_brownian_bridge {
            time_grid.num_steps()
        } else {
            time_grid.num_steps() * num_factors
        };

        if sobol_dimension == 0 || sobol_dimension > MAX_SOBOL_DIMENSION {
            return Err(finstack_core::Error::Validation(format!(
                "Sobol dimension {} is unsupported (maximum {})",
                sobol_dimension, MAX_SOBOL_DIMENSION
            )));
        }

        Ok(sobol_dimension)
    }

    #[cfg(feature = "mc")]
    #[allow(clippy::too_many_arguments)]
    fn price_with_sobol<P>(
        &self,
        process: &GbmProcess,
        initial_spot: f64,
        time_grid: TimeGrid,
        payoff: &P,
        currency: Currency,
        discount_factor: f64,
    ) -> Result<MonteCarloResult>
    where
        P: Payoff,
    {
        let sobol_dimension =
            self.validate_sobol_configuration(&time_grid, process.num_factors())?;
        let mut sobol = SobolRng::try_new(sobol_dimension, self.config.seed)
            .map_err(|err| Error::Validation(err.to_string()))?;
        let disc = ExactGbm::new();
        let initial_state = vec![initial_spot];
        let capture_enabled = self.config.path_capture.enabled;
        let sampling_method = match &self.config.path_capture.capture_mode {
            crate::engine::PathCaptureMode::All => PathSamplingMethod::All,
            crate::engine::PathCaptureMode::Sample { count, seed } => {
                PathSamplingMethod::RandomSample {
                    count: *count,
                    seed: *seed,
                }
            }
        };

        let mut stats = OnlineStats::new();
        let mut state = vec![0.0; process.dim()];
        let mut work = vec![0.0; disc.work_size(process)];
        let mut z_path = vec![0.0; sobol_dimension];
        let mut z_step = vec![0.0; process.num_factors()];
        let mut w_path = vec![0.0; time_grid.num_steps() + 1];
        let bridge = self
            .config
            .use_brownian_bridge
            .then(|| BrownianBridge::new(time_grid.num_steps()));
        let mut captured_paths = if capture_enabled {
            let estimated_capacity = match self.config.path_capture.capture_mode {
                crate::engine::PathCaptureMode::All => self.config.num_paths,
                crate::engine::PathCaptureMode::Sample { count, .. } => count,
            };
            Vec::with_capacity(estimated_capacity)
        } else {
            Vec::new()
        };

        for path_id in 0..self.config.num_paths {
            sobol.fill_std_normals(&mut z_path);

            let mut payoff_local = payoff.clone();
            payoff_local.reset();

            // Keep auxiliary uniforms off the Sobol stream so asset dimensions remain stable.
            let mut aux_rng = PhiloxRng::new(self.config.seed ^ ((path_id as u64) << 1));
            payoff_local.on_path_start(&mut aux_rng);

            state.copy_from_slice(&initial_state);
            let should_capture = capture_enabled
                && self
                    .config
                    .path_capture
                    .should_capture(path_id, self.config.num_paths);
            let mut path_state = PathState::new(0, 0.0);
            process.populate_path_state(&state, &mut path_state);
            path_state.set_uniform_random(aux_rng.next_u01());
            payoff_local.on_event(&mut path_state);

            let mut simulated_path = if should_capture {
                let mut simulated =
                    SimulatedPath::with_capacity(path_id, time_grid.num_steps() + 1);
                let initial_state_vec = SmallVec::from_slice(&state);
                let mut initial_point = PathPoint::with_state(0, 0.0, initial_state_vec);
                path_state.drain_cashflows(|time, amount, cf_type| {
                    initial_point.add_typed_cashflow(time, amount, cf_type);
                });
                if self.config.path_capture.capture_payoffs {
                    initial_point.set_payoff(payoff_local.value(currency).amount());
                }
                simulated.add_point(initial_point);
                Some(simulated)
            } else {
                None
            };

            if let Some(bridge) = &bridge {
                if time_grid.is_uniform() {
                    bridge.construct_path(&z_path, &mut w_path, time_grid.dt(0));
                } else {
                    bridge.construct_path_irregular(&z_path, &mut w_path, time_grid.times());
                }
            }

            for step in 0..time_grid.num_steps() {
                let t = time_grid.time(step);
                let dt = time_grid.dt(step);

                if bridge.is_some() {
                    z_step[0] = (w_path[step + 1] - w_path[step]) / dt.sqrt();
                } else {
                    let offset = step * process.num_factors();
                    z_step.copy_from_slice(&z_path[offset..offset + process.num_factors()]);
                }

                disc.step(process, t, dt, &mut state, &z_step, &mut work);
                path_state.set_step_time(step + 1, t + dt);
                process.populate_path_state(&state, &mut path_state);
                path_state.set_uniform_random(aux_rng.next_u01());
                payoff_local.on_event(&mut path_state);

                if let Some(simulated_path) = &mut simulated_path {
                    let state_vec = SmallVec::from_slice(&state);
                    let mut point = PathPoint::with_state(step + 1, t + dt, state_vec);
                    path_state.drain_cashflows(|time, amount, cf_type| {
                        point.add_typed_cashflow(time, amount, cf_type);
                    });
                    if self.config.path_capture.capture_payoffs {
                        point.set_payoff(payoff_local.value(currency).amount());
                    }
                    simulated_path.add_point(point);
                }
            }

            let payoff_value = payoff_local.value(currency).amount();
            stats.update(payoff_value * discount_factor);

            if let Some(mut simulated_path) = simulated_path {
                simulated_path.set_final_value(payoff_value * discount_factor);
                let cashflow_amounts = simulated_path.extract_cashflow_amounts();
                if cashflow_amounts.len() >= 2 {
                    use finstack_core::cashflow::InternalRateOfReturn;
                    if let Ok(irr) = cashflow_amounts.irr(None) {
                        simulated_path.set_irr(irr);
                    }
                }
                captured_paths.push(simulated_path);
            }
        }

        let estimate = Estimate::new(
            stats.mean(),
            stats.stderr(),
            stats.confidence_interval(0.05),
            stats.count(),
        )
        .with_std_dev(stats.std_dev());
        let process_params = process.metadata();

        if capture_enabled {
            let mut dataset = PathDataset::new(stats.count(), sampling_method, process_params);
            for path in &captured_paths {
                dataset.add_path(path.clone());
            }
            let estimate = apply_captured_path_statistics(estimate, &captured_paths);
            Ok(MonteCarloResult::with_paths(
                MoneyEstimate::from_estimate(estimate, currency),
                dataset,
            ))
        } else {
            Ok(MonteCarloResult::new(MoneyEstimate::from_estimate(
                estimate, currency,
            )))
        }
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
        self.price_with_grid(
            process,
            initial_spot,
            time_grid,
            payoff,
            currency,
            discount_factor,
        )
    }

    /// Price a path-dependent option with a custom time grid.
    #[allow(clippy::too_many_arguments)]
    pub fn price_with_grid<P>(
        &self,
        process: &GbmProcess,
        initial_spot: f64,
        time_grid: TimeGrid,
        payoff: &P,
        currency: Currency,
        discount_factor: f64,
    ) -> Result<MoneyEstimate>
    where
        P: Payoff,
    {
        #[cfg(feature = "mc")]
        if self.config.use_sobol {
            return self
                .price_with_sobol(
                    process,
                    initial_spot,
                    time_grid,
                    payoff,
                    currency,
                    discount_factor,
                )
                .map(|result| result.estimate);
        }

        // Create MC engine. Antithetic pairing is handled inline by the engine
        // (see McEngine::simulate_antithetic_pair); path-capture + antithetic
        // is rejected at validate_runtime.
        let engine_config = McEngineConfig {
            num_paths: self.config.num_paths,
            seed: self.config.seed,
            time_grid,
            target_ci_half_width: None,
            use_parallel: self.config.use_parallel,
            chunk_size: self.config.chunk_size,
            path_capture: self.config.path_capture.clone(),
            antithetic: self.config.antithetic,
        };

        let engine = McEngine::new(engine_config);
        let disc = ExactGbm::new();
        let initial_state = vec![initial_spot];
        let rng = PhiloxRng::new(self.config.seed);

        if engine.config().path_capture.enabled {
            let process_params = process.metadata();
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
            Ok(result.estimate)
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
        #[cfg(feature = "mc")]
        if self.config.use_sobol {
            let time_grid = TimeGrid::uniform(time_to_maturity, num_steps)?;
            return self.price_with_sobol(
                process,
                initial_spot,
                time_grid,
                payoff,
                currency,
                discount_factor,
            );
        }

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

        let disc = ExactGbm::new();
        let initial_state = vec![initial_spot];
        let process_params = process.metadata();

        let rng = PhiloxRng::new(self.config.seed);
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
    /// This uses the Likelihood Ratio Method, deriving the standardized terminal
    /// shock `Z = W_T / √T` from terminal spots under GBM:
    /// `Z = (ln(S_T/S_0) - (r - q - 0.5 σ^2) T) / (σ √T)`.
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

        let rng = crate::rng::philox::PhiloxRng::new(self.config.seed);
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

        // Build undiscounted payoffs and standardized terminal shocks.
        let mut payoffs: Vec<f64> = Vec::with_capacity(paths.len());
        let mut terminal_shocks: Vec<f64> = Vec::with_capacity(paths.len());
        let mu = rate - dividend_yield - 0.5 * volatility * volatility;
        for p in paths {
            // Final discounted payoff value is stored; un-discount it
            let undisc = p.final_value / discount_factor;
            payoffs.push(undisc);

            // Terminal spot from last point's state var
            if let Some(last) = p.terminal_point() {
                if let Some(s_t) = last.spot() {
                    let z_t = ((s_t / initial_spot).ln() - mu * time_to_maturity)
                        / (volatility * time_to_maturity.sqrt());
                    terminal_shocks.push(z_t);
                }
            }
        }

        if payoffs.len() == terminal_shocks.len() && !payoffs.is_empty() {
            use super::super::greeks::lrm::{lrm_delta, lrm_vega};
            let (delta, _) = lrm_delta(
                &payoffs,
                &terminal_shocks,
                initial_spot,
                volatility,
                time_to_maturity,
                discount_factor,
            );
            let (vega, _) = lrm_vega(
                &payoffs,
                &terminal_shocks,
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
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::{PathDependentPricer, PathDependentPricerConfig};
    use crate::payoff::asian::{AsianCall, AveragingMethod};
    use crate::process::gbm::{GbmParams, GbmProcess};
    use crate::rng::sobol::MAX_SOBOL_DIMENSION;
    use crate::time_grid::TimeGrid;
    use finstack_core::currency::Currency;

    use crate::payoff::lookback::{Lookback, LookbackDirection};

    #[ignore = "slow"]
    #[test]
    fn test_path_dependent_pricer_asian() {
        let config = PathDependentPricerConfig::new(10_000)
            .with_seed(42)
            .with_parallel(false);
        let pricer = PathDependentPricer::new(config);

        let gbm = GbmProcess::new(GbmParams::new(0.05, 0.02, 0.2).unwrap());

        // Monthly fixings
        let fixing_steps: Vec<usize> = (0..=12).map(|i| i * 21).collect();
        let asian = AsianCall::new(100.0, 1.0, AveragingMethod::Arithmetic, fixing_steps);

        let result = pricer
            .price(&gbm, 100.0, 1.0, 252, &asian, Currency::USD, 1.0)
            .expect("should succeed");

        // Should get reasonable Asian option value
        assert!(result.mean.amount() > 0.0);
        assert!(result.mean.amount() < 20.0);
    }

    #[ignore = "slow"]
    #[test]
    fn test_path_dependent_pricer_lookback() {
        let config = PathDependentPricerConfig::new(10_000)
            .with_seed(42)
            .with_parallel(false);
        let pricer = PathDependentPricer::new(config);

        let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 0.3).unwrap());
        let lookback = Lookback::new(LookbackDirection::Call, 100.0, 1.0, 252);

        let result = pricer
            .price(&gbm, 100.0, 1.0, 252, &lookback, Currency::USD, 1.0)
            .expect("should succeed");

        // Lookback should have positive value
        assert!(result.mean.amount() > 0.0);
    }

    #[cfg(feature = "mc")]
    #[test]
    fn test_sobol_price_with_grid_multiple_paths() {
        let config = PathDependentPricerConfig::new(8)
            .with_seed(7)
            .with_parallel(false)
            .with_sobol(true)
            .with_brownian_bridge(false);
        let pricer = PathDependentPricer::new(config);
        let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 0.2).unwrap());
        let time_grid = TimeGrid::uniform(1.0, 4).expect("grid should build");
        let fixing_steps = vec![1, 2, 3, 4];
        let asian = AsianCall::new(100.0, 1.0, AveragingMethod::Arithmetic, fixing_steps);

        let result = pricer
            .price_with_grid(&gbm, 100.0, time_grid, &asian, Currency::USD, 1.0)
            .expect("Sobol pricing should succeed for multiple paths");

        assert_eq!(result.num_paths, 8);
    }

    #[cfg(feature = "mc")]
    #[test]
    fn test_sobol_price_with_paths_multiple_paths() {
        fn interpolated_percentile(sorted_values: &[f64], percentile: f64) -> f64 {
            let rank = percentile * (sorted_values.len() - 1) as f64;
            let lower = rank.floor() as usize;
            let upper = rank.ceil() as usize;
            if lower == upper {
                sorted_values[lower]
            } else {
                let weight = rank - lower as f64;
                sorted_values[lower] * (1.0 - weight) + sorted_values[upper] * weight
            }
        }

        let config = PathDependentPricerConfig::new(8)
            .with_seed(11)
            .with_parallel(false)
            .with_sobol(true)
            .capture_all_paths();
        let pricer = PathDependentPricer::new(config);
        let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 0.2).unwrap());
        let fixing_steps = vec![1, 2, 3, 4];
        let asian = AsianCall::new(100.0, 1.0, AveragingMethod::Arithmetic, fixing_steps);

        let result = pricer
            .price_with_paths(&gbm, 100.0, 1.0, 4, &asian, Currency::USD, 1.0)
            .expect("Sobol path capture should succeed for multiple paths");

        assert_eq!(result.estimate.num_paths, 8);
        assert_eq!(result.num_captured_paths(), 8);

        let captured = result.paths.as_ref().expect("paths should be captured");
        let mut final_values: Vec<f64> =
            captured.paths.iter().map(|path| path.final_value).collect();
        final_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let len = final_values.len();
        let expected_median = if len.is_multiple_of(2) {
            (final_values[len / 2 - 1] + final_values[len / 2]) / 2.0
        } else {
            final_values[len / 2]
        };
        let expected_p25 = interpolated_percentile(&final_values, 0.25);
        let expected_p75 = interpolated_percentile(&final_values, 0.75);

        assert_eq!(result.estimate.median, Some(expected_median));
        assert_eq!(result.estimate.percentile_25, Some(expected_p25));
        assert_eq!(result.estimate.percentile_75, Some(expected_p75));
        assert_eq!(result.estimate.min, Some(final_values[0]));
        assert_eq!(result.estimate.max, Some(final_values[len - 1]));
    }

    #[cfg(feature = "mc")]
    #[test]
    fn test_sobol_brownian_bridge_supports_irregular_grid() {
        let config = PathDependentPricerConfig::new(8)
            .with_seed(13)
            .with_parallel(false)
            .with_sobol(true)
            .with_brownian_bridge(true);
        let pricer = PathDependentPricer::new(config);
        let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 0.2).unwrap());
        let time_grid = TimeGrid::from_times(vec![0.0, 0.2, 0.55, 1.0]).expect("grid should build");
        let fixing_steps = vec![1, 2, 3];
        let asian = AsianCall::new(100.0, 1.0, AveragingMethod::Arithmetic, fixing_steps);

        let result = pricer
            .price_with_grid(&gbm, 100.0, time_grid, &asian, Currency::USD, 1.0)
            .expect("Irregular-grid Sobol Brownian bridge pricing should succeed");

        assert_eq!(result.num_paths, 8);
    }

    #[cfg(feature = "mc")]
    #[test]
    fn test_sobol_rejects_excessive_dimension() {
        let config = PathDependentPricerConfig::new(1)
            .with_seed(17)
            .with_parallel(false)
            .with_sobol(true)
            .with_brownian_bridge(false);
        let pricer = PathDependentPricer::new(config);
        let gbm = GbmProcess::new(GbmParams::new(0.05, 0.0, 0.2).unwrap());
        let fixing_steps = vec![MAX_SOBOL_DIMENSION + 1];
        let asian = AsianCall::new(100.0, 1.0, AveragingMethod::Arithmetic, fixing_steps);

        let err = pricer
            .price(
                &gbm,
                100.0,
                1.0,
                MAX_SOBOL_DIMENSION + 1,
                &asian,
                Currency::USD,
                1.0,
            )
            .expect_err("excessive Sobol dimension should be rejected");

        assert!(err.to_string().contains("Sobol"));
    }
}
