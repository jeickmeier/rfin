use super::config::{McEngineBuilder, McEngineConfig, MAX_NUM_PATHS};
use super::path_capture::PathCaptureMode;
use crate::estimate::Estimate;
use crate::online_stats::OnlineStats;
use crate::paths::ProcessParams;
use crate::results::{MoneyEstimate, MonteCarloResult};
use crate::traits::{Discretization, Payoff, RandomStream, StochasticProcess};
use finstack_core::currency::Currency;
use finstack_core::Result;

#[cfg(feature = "parallel")]
use std::ops::Range;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Generic Monte Carlo execution engine.
///
/// [`McEngine`] prices a payoff by simulating discounted path values under a
/// supplied process and discretization scheme. It can run serially or in
/// parallel and optionally capture paths for diagnostics.
pub struct McEngine {
    pub(super) config: McEngineConfig,
}

/// Calculate adaptive chunk size for parallel MC execution.
///
/// Balances load distribution across cores with cache efficiency.
/// Target: 4 chunks per thread for good load balancing.
#[cfg(feature = "parallel")]
pub(super) fn adaptive_chunk_size(num_paths: usize) -> usize {
    let num_cpus = rayon::current_num_threads();
    // Target 4 chunks per thread for load balancing
    // Min 100 paths per chunk to amortize overhead
    // Max 10_000 paths to avoid cache thrashing
    (num_paths / (num_cpus * 4)).clamp(100, 10_000)
}

/// Pre-sized chunk index ranges for parallel path loops (avoids `Vec` reallocations).
#[cfg(feature = "parallel")]
pub(super) fn parallel_path_chunks(num_paths: usize, chunk_size: usize) -> Vec<Range<usize>> {
    let num_chunks = num_paths.div_ceil(chunk_size);
    let mut chunks = Vec::with_capacity(num_chunks);
    for start in (0..num_paths).step_by(chunk_size) {
        let end = (start + chunk_size).min(num_paths);
        chunks.push(start..end);
    }
    chunks
}

impl McEngine {
    /// Create a builder with the crate's default engine settings.
    pub fn builder() -> McEngineBuilder {
        McEngineBuilder::new()
    }

    /// Create an engine from an explicit configuration.
    pub fn new(config: McEngineConfig) -> Self {
        Self { config }
    }

    /// Construct from config (used by builder).
    pub(super) fn from_config(config: McEngineConfig) -> Self {
        Self { config }
    }

    /// Borrow the engine configuration.
    pub fn config(&self) -> &McEngineConfig {
        &self.config
    }

    pub(super) fn validate_runtime<R, P>(
        &self,
        rng: &R,
        process: &P,
        initial_state: &[f64],
        discount_factor: f64,
        process_params: Option<&ProcessParams>,
    ) -> Result<()>
    where
        R: RandomStream,
        P: StochasticProcess,
    {
        if self.config.num_paths == 0 {
            return Err(finstack_core::Error::Validation(
                "Monte Carlo num_paths must be greater than zero".to_string(),
            ));
        }

        if self.config.num_paths > MAX_NUM_PATHS {
            return Err(finstack_core::Error::Validation(format!(
                "Monte Carlo num_paths ({}) exceeds maximum ({})",
                self.config.num_paths, MAX_NUM_PATHS
            )));
        }

        if self.config.chunk_size == 0 {
            return Err(finstack_core::Error::Validation(
                "Monte Carlo chunk_size must be greater than zero".to_string(),
            ));
        }

        if initial_state.len() != process.dim() {
            return Err(finstack_core::Error::Validation(format!(
                "initial_state length {} does not match process dimension {}",
                initial_state.len(),
                process.dim()
            )));
        }

        if !discount_factor.is_finite() || discount_factor < 0.0 {
            return Err(finstack_core::Error::Validation(
                "discount_factor must be finite and non-negative".to_string(),
            ));
        }

        if let Some(target) = self.config.target_ci_half_width {
            if !target.is_finite() || target <= 0.0 {
                return Err(finstack_core::Error::Validation(
                    "target_ci_half_width must be finite and positive".to_string(),
                ));
            }

            if self.config.use_parallel {
                return Err(finstack_core::Error::Validation(
                    "target_ci_half_width is currently unsupported with use_parallel=true"
                        .to_string(),
                ));
            }
        }

        if self.config.use_parallel && !rng.supports_splitting() {
            return Err(finstack_core::Error::Validation(
                "Parallel Monte Carlo requires a splittable RNG (e.g., PhiloxRng). \
                 SobolRng does not support stream splitting — use serial mode (use_parallel: false) \
                 or switch to PhiloxRng for parallel execution."
                    .to_string(),
            ));
        }

        if self.config.path_capture.enabled {
            if self.config.antithetic {
                return Err(finstack_core::Error::Validation(
                    "Path capture is currently unsupported with antithetic=true".to_string(),
                ));
            }

            if let PathCaptureMode::Sample { count, .. } = self.config.path_capture.capture_mode {
                if count == 0 || count > self.config.num_paths {
                    return Err(finstack_core::Error::Validation(format!(
                        "Path capture sample count must be between 1 and num_paths (got {count})"
                    )));
                }
            }
        }

        if let Some(params) = process_params {
            params.validate()?;
        }

        Ok(())
    }

    /// Price a payoff via discounted Monte Carlo simulation.
    ///
    /// # Type Parameters
    ///
    /// * `R` - Random stream type
    /// * `P` - Stochastic process type
    /// * `D` - Discretization scheme type
    /// * `F` - Payoff type
    ///
    /// # Arguments
    ///
    /// * `rng` - Random stream used to generate per-path shocks.
    /// * `process` - Stochastic process defining drift, diffusion, and state layout.
    /// * `disc` - Time-stepping scheme compatible with `process`.
    /// * `initial_state` - Initial process state. Its length must equal `process.dim()`.
    /// * `payoff` - Payoff accumulator evaluated on each simulated path.
    /// * `currency` - Currency tag for the returned [`MoneyEstimate`].
    /// * `discount_factor` - Present-value multiplier for the payoff horizon.
    ///
    /// # Returns
    ///
    /// A discounted Monte Carlo estimate in `currency`, including the mean,
    /// standard error, and 95% confidence interval of discounted path values.
    ///
    /// # Errors
    ///
    /// Returns an error when:
    ///
    /// * `num_paths == 0`
    /// * `chunk_size == 0`
    /// * `initial_state.len() != process.dim()`
    /// * `discount_factor` is not finite or is negative
    /// * `target_ci_half_width` is non-positive or combined with parallel mode
    /// * parallel mode is requested with an RNG that does not support splitting
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use finstack_core::currency::Currency;
    /// use finstack_monte_carlo::prelude::*;
    ///
    /// let engine = McEngine::builder()
    ///     .num_paths(25_000)
    ///     .seed(11)
    ///     .uniform_grid(1.0, 252)
    ///     .build()
    ///     .expect("valid Monte Carlo configuration");
    ///
    /// let rng = PhiloxRng::new(11);
    /// let process = GbmProcess::with_params(0.03, 0.01, 0.20);
    /// let disc = ExactGbm::new();
    /// let payoff = EuropeanCall::new(100.0, 1.0, 252);
    /// let discount_factor = (-0.03_f64).exp();
    ///
    /// let result = engine
    ///     .price(
    ///         &rng,
    ///         &process,
    ///         &disc,
    ///         &[100.0],
    ///         &payoff,
    ///         Currency::USD,
    ///         discount_factor,
    ///     )
    ///     .expect("pricing should succeed");
    ///
    /// assert!(result.mean.amount() >= 0.0);
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn price<R, P, D, F>(
        &self,
        rng: &R,
        process: &P,
        disc: &D,
        initial_state: &[f64],
        payoff: &F,
        currency: Currency,
        discount_factor: f64,
    ) -> Result<MoneyEstimate>
    where
        R: RandomStream,
        P: StochasticProcess,
        D: Discretization<P>,
        F: Payoff,
    {
        self.validate_runtime(rng, process, initial_state, discount_factor, None)?;

        let estimate = if self.config.use_parallel {
            self.price_parallel(
                rng,
                process,
                disc,
                initial_state,
                payoff,
                currency,
                discount_factor,
            )?
        } else {
            self.price_serial(
                rng,
                process,
                disc,
                initial_state,
                payoff,
                currency,
                discount_factor,
            )?
        };

        Ok(MoneyEstimate::from_estimate(estimate, currency))
    }

    /// Price a payoff and optionally return captured paths.
    ///
    /// This method extends [`Self::price`] by validating and attaching
    /// [`ProcessParams`] metadata and by returning a [`MonteCarloResult`] that
    /// may include a captured [`crate::paths::PathDataset`].
    ///
    /// # Arguments
    ///
    /// * `process_params` - Metadata describing the captured state layout,
    ///   process parameters, and optional correlation matrix.
    ///
    /// For the other arguments, see [`Self::price`].
    ///
    /// # Returns
    ///
    /// A Monte Carlo result containing the discounted estimate and, when path
    /// capture is enabled, a captured-path dataset.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::price`], plus any validation error
    /// raised by [`ProcessParams::validate`](crate::paths::ProcessParams::validate).
    /// The engine also rejects `antithetic = true` together with path capture.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use finstack_core::currency::Currency;
    /// use finstack_monte_carlo::prelude::*;
    ///
    /// let engine = McEngine::builder()
    ///     .num_paths(2_000)
    ///     .seed(5)
    ///     .uniform_grid(1.0, 12)
    ///     .path_capture(PathCaptureConfig::sample(100, 17).with_payoffs())
    ///     .build()
    ///     .expect("valid Monte Carlo configuration");
    ///
    /// let rng = PhiloxRng::new(5);
    /// let process = GbmProcess::with_params(0.03, 0.01, 0.20);
    /// let disc = ExactGbm::new();
    /// let payoff = EuropeanCall::new(100.0, 1.0, 12);
    /// let discount_factor = (-0.03_f64).exp();
    /// let process_params = ProcessParams::new("GBM").with_factors(vec!["spot".to_string()]);
    ///
    /// let result = engine
    ///     .price_with_capture(
    ///         &rng,
    ///         &process,
    ///         &disc,
    ///         &[100.0],
    ///         &payoff,
    ///         Currency::USD,
    ///         discount_factor,
    ///         process_params,
    ///     )
    ///     .expect("pricing with capture should succeed");
    ///
    /// assert!(result.estimate.mean.amount().is_finite());
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn price_with_capture<R, P, D, F>(
        &self,
        rng: &R,
        process: &P,
        disc: &D,
        initial_state: &[f64],
        payoff: &F,
        currency: Currency,
        discount_factor: f64,
        process_params: ProcessParams,
    ) -> Result<MonteCarloResult>
    where
        R: RandomStream,
        P: StochasticProcess,
        D: Discretization<P>,
        F: Payoff,
    {
        self.validate_runtime(
            rng,
            process,
            initial_state,
            discount_factor,
            Some(&process_params),
        )?;

        let (estimate, paths) = if self.config.use_parallel {
            self.price_parallel_with_capture(
                rng,
                process,
                disc,
                initial_state,
                payoff,
                currency,
                discount_factor,
                process_params,
            )?
        } else {
            self.price_serial_with_capture(
                rng,
                process,
                disc,
                initial_state,
                payoff,
                currency,
                discount_factor,
                process_params,
            )?
        };

        let money_estimate = MoneyEstimate::from_estimate(estimate, currency);

        if let Some(paths) = paths {
            Ok(MonteCarloResult::with_paths(money_estimate, paths))
        } else {
            Ok(MonteCarloResult::new(money_estimate))
        }
    }

    /// Serial pricing implementation.
    #[allow(clippy::too_many_arguments)]
    fn price_serial<R, P, D, F>(
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
        let mut stats = OnlineStats::new();
        let dim = process.dim();
        let num_factors = process.num_factors();
        let work_size = disc.work_size(process);

        // Pre-allocate buffers (reused across paths)
        let mut state = vec![0.0; dim];
        let mut z = vec![0.0; num_factors];
        let mut work = vec![0.0; work_size];
        let mut state_a = vec![0.0; dim];
        let mut z_anti = vec![0.0; num_factors];

        // Single clone reused across all paths (reset between iterations)
        let mut payoff_local = payoff.clone();
        let mut num_skipped: usize = 0;

        for path_id in 0..self.config.num_paths {
            let mut path_rng = rng.split(path_id as u64);

            payoff_local.reset();
            payoff_local.on_path_start(&mut path_rng);

            let payoff_value = if self.config.antithetic {
                self.simulate_antithetic_pair(
                    &mut path_rng,
                    process,
                    disc,
                    initial_state,
                    &mut payoff_local,
                    &mut state,
                    &mut state_a,
                    &mut z,
                    &mut z_anti,
                    &mut work,
                    currency,
                )?
            } else {
                self.simulate_path(
                    &mut path_rng,
                    process,
                    disc,
                    initial_state,
                    &mut payoff_local,
                    &mut state,
                    &mut z,
                    &mut work,
                    currency,
                )?
            };

            // Accumulate statistics (skip non-finite values to prevent NaN poisoning)
            let discounted_value = payoff_value * discount_factor;
            if discounted_value.is_finite() {
                stats.update(discounted_value);
            } else {
                num_skipped += 1;
                tracing::warn!(
                    path_id,
                    payoff_value,
                    discount_factor,
                    "Skipping non-finite payoff value in MC statistics"
                );
            }

            // Check auto-stop condition
            if let Some(target) = self.config.target_ci_half_width {
                if stats.count() > 1000 && stats.ci_half_width() < target {
                    break;
                }
            }
        }

        Ok(Estimate::new(
            stats.mean(),
            stats.stderr(),
            stats.confidence_interval(0.05),
            stats.count(),
        )
        .with_std_dev(stats.std_dev())
        .with_num_skipped(num_skipped))
    }

    /// Parallel pricing implementation.
    #[cfg(feature = "parallel")]
    #[allow(clippy::too_many_arguments)]
    fn price_parallel<R, P, D, F>(
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
        // Split paths into chunks for parallel processing
        // Use adaptive chunk size if default (1000), otherwise use configured size
        let effective_chunk_size = if self.config.chunk_size == 1000 {
            adaptive_chunk_size(self.config.num_paths)
        } else {
            self.config.chunk_size
        };

        let chunks = parallel_path_chunks(self.config.num_paths, effective_chunk_size);

        // Process chunks in parallel
        let chunk_results: Vec<Result<(OnlineStats, usize)>> = chunks
            .par_iter()
            .map(|range| {
                let mut stats = OnlineStats::new();
                let mut chunk_skipped: usize = 0;
                let dim = process.dim();
                let num_factors = process.num_factors();
                let work_size = disc.work_size(process);

                let mut state = vec![0.0; dim];
                let mut z = vec![0.0; num_factors];
                let mut work = vec![0.0; work_size];
                let mut state_a = vec![0.0; dim];
                let mut z_anti = vec![0.0; num_factors];
                let mut payoff_clone = payoff.clone();

                for path_id in range.clone() {
                    let mut path_rng = rng.split(path_id as u64);

                    payoff_clone.reset();
                    payoff_clone.on_path_start(&mut path_rng);

                    let payoff_value = if self.config.antithetic {
                        self.simulate_antithetic_pair(
                            &mut path_rng,
                            process,
                            disc,
                            initial_state,
                            &mut payoff_clone,
                            &mut state,
                            &mut state_a,
                            &mut z,
                            &mut z_anti,
                            &mut work,
                            currency,
                        )?
                    } else {
                        self.simulate_path(
                            &mut path_rng,
                            process,
                            disc,
                            initial_state,
                            &mut payoff_clone,
                            &mut state,
                            &mut z,
                            &mut work,
                            currency,
                        )?
                    };

                    let discounted_value = payoff_value * discount_factor;
                    if discounted_value.is_finite() {
                        stats.update(discounted_value);
                    } else {
                        chunk_skipped += 1;
                        tracing::warn!(
                            path_id,
                            payoff_value,
                            discount_factor,
                            "Skipping non-finite payoff value in MC statistics"
                        );
                    }
                }

                Ok((stats, chunk_skipped))
            })
            .collect();

        // Collect and handle errors (fail-fast on first error)
        let chunk_stats: Vec<(OnlineStats, usize)> =
            chunk_results.into_iter().collect::<Result<Vec<_>>>()?;

        // Deterministically reduce chunk statistics
        let mut combined = OnlineStats::new();
        let mut num_skipped: usize = 0;
        for (chunk_stat, chunk_skipped) in chunk_stats {
            combined.merge(&chunk_stat);
            num_skipped += chunk_skipped;
        }

        Ok(Estimate::new(
            combined.mean(),
            combined.stderr(),
            combined.confidence_interval(0.05),
            combined.count(),
        )
        .with_std_dev(combined.std_dev())
        .with_num_skipped(num_skipped))
    }

    /// Parallel pricing (fallback when parallel feature disabled).
    #[cfg(not(feature = "parallel"))]
    fn price_parallel<R, P, D, F>(
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
        // Fall back to serial when parallel feature is disabled
        self.price_serial(
            rng,
            process,
            disc,
            initial_state,
            payoff,
            currency,
            discount_factor,
        )
    }
}
