//! Monte Carlo execution engine and runtime configuration.
//!
//! This module contains the generic simulation loop used throughout the crate.
//! [`McEngine`] combines a [`RandomStream`], a [`StochasticProcess`], a
//! compatible [`Discretization`], and a [`Payoff`] into discounted Monte Carlo
//! estimates.
//!
//! The engine provides:
//!
//! - reusable serial and parallel path loops
//! - deterministic path-to-stream mapping for reproducibility
//! - online mean / variance estimation via Welford's algorithm
//! - optional early stopping on a target 95% confidence-interval half-width
//! - optional path capture for visualization and diagnostics
//! - built-in antithetic pairing in the generic engine loop
//!
//! # Important Runtime Constraints
//!
//! Several configuration combinations are intentionally rejected at runtime:
//!
//! - parallel execution requires an RNG with deterministic stream splitting
//! - auto-stopping via `target_ci_half_width` is currently serial-only
//! - path capture cannot be combined with antithetic pairing
//! - sampled path capture uses deterministic Bernoulli sampling, so the number
//!   of captured paths is an expected count, not an exact count
//!
//! # Conventions
//!
//! - `discount_factor` is the present-value multiplier for the payoff horizon
//!   and must be finite and non-negative.
//! - Payoffs produce undiscounted [`finstack_core::money::Money`] amounts in the
//!   requested currency. The engine applies `discount_factor` after each path is
//!   simulated.
//! - Confidence intervals are reported on discounted path values.
//!
//! # References
//!
//! - Online variance accumulation follows
//!   [`docs/REFERENCES.md#welford-1962`](docs/REFERENCES.md#welford-1962).
//! - Discounting conventions should be consistent with
//!   [`docs/REFERENCES.md#hull-options-futures`](docs/REFERENCES.md#hull-options-futures).

use super::results::{MoneyEstimate, MonteCarloResult};
use super::traits::Payoff;
use crate::captured_path_stats::apply_captured_path_statistics;
use crate::estimate::Estimate;
use crate::online_stats::OnlineStats;
use crate::paths::{PathDataset, PathPoint, PathSamplingMethod, ProcessParams, SimulatedPath};
use crate::time_grid::TimeGrid;
use crate::traits::{Discretization, PathState, RandomStream, StochasticProcess};
use finstack_core::currency::Currency;
use finstack_core::Result;
use smallvec::SmallVec;

#[cfg(feature = "parallel")]
use std::ops::Range;

#[cfg(feature = "parallel")]
use rayon::prelude::*;
#[cfg(feature = "parallel")]
use std::sync::Mutex;

/// Selects how simulated paths are captured for diagnostics.
///
/// Use [`PathCaptureMode::All`] when every path should be retained. Use
/// [`PathCaptureMode::Sample`] when you only need a representative subset for
/// plotting or debugging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathCaptureMode {
    /// Capture every simulated path.
    All,
    /// Capture a deterministic sample of paths.
    ///
    /// The sample is selected by hashing `path_id` together with `seed` and
    /// comparing the result against the implied sampling probability
    /// `count / num_paths`. This makes the capture decision reproducible across
    /// serial and parallel runs, but the realized number of captured paths is
    /// generally close to `count`, not guaranteed to equal it exactly.
    Sample {
        /// Target number of paths to capture on average.
        count: usize,
        /// Seed controlling the deterministic sampling decision.
        seed: u64,
    },
}

/// Configures optional path capture during Monte Carlo pricing.
///
/// Captured paths can include state vectors, cashflows, and optionally payoff
/// snapshots at each time step. The engine validates that sampled capture counts
/// are between `1` and `num_paths`, and that capture is not combined with
/// antithetic pricing.
#[derive(Debug, Clone)]
pub struct PathCaptureConfig {
    /// Whether path capture is enabled
    pub enabled: bool,
    /// Capture mode (all paths or sample)
    pub capture_mode: PathCaptureMode,
    /// Whether to capture payoff values at each timestep
    pub capture_payoffs: bool,
}

impl PathCaptureConfig {
    /// Create a disabled path-capture configuration.
    ///
    /// # Returns
    ///
    /// A configuration with capture disabled and no payoff snapshots.
    pub fn new() -> Self {
        Self {
            enabled: false,
            capture_mode: PathCaptureMode::All,
            capture_payoffs: false,
        }
    }

    /// Enable capture for every simulated path.
    ///
    /// # Returns
    ///
    /// A configuration that records all paths but does not capture intermediate
    /// payoff values unless [`Self::with_payoffs`] is called.
    pub fn all() -> Self {
        Self {
            enabled: true,
            capture_mode: PathCaptureMode::All,
            capture_payoffs: false,
        }
    }

    /// Enable capture for a deterministic sample of paths.
    ///
    /// # Arguments
    ///
    /// * `count` - Target number of captured paths on average. Runtime
    ///   validation requires `1 <= count <= num_paths`.
    /// * `seed` - Sampling seed used in the hash-based selection rule.
    ///
    /// # Returns
    ///
    /// A configuration that records an expected sample of paths. The realized
    /// number of captured paths can differ from `count`.
    pub fn sample(count: usize, seed: u64) -> Self {
        Self {
            enabled: true,
            capture_mode: PathCaptureMode::Sample { count, seed },
            capture_payoffs: false,
        }
    }

    /// Record payoff snapshots at each captured time step.
    ///
    /// # Returns
    ///
    /// The same configuration with `capture_payoffs` enabled.
    pub fn with_payoffs(mut self) -> Self {
        self.capture_payoffs = true;
        self
    }

    /// Disable path capture explicitly.
    ///
    /// # Returns
    ///
    /// A disabled path-capture configuration.
    pub fn disabled() -> Self {
        Self::new()
    }

    /// Decide whether a particular path should be captured.
    ///
    /// # Arguments
    ///
    /// * `path_id` - Zero-based Monte Carlo path identifier.
    /// * `num_paths` - Total number of simulated paths in the run.
    ///
    /// # Returns
    ///
    /// `true` if the path should be recorded under the configured capture mode.
    /// For sampled capture this uses deterministic Bernoulli sampling, so the
    /// total number of `true` results is approximate.
    pub fn should_capture(&self, path_id: usize, num_paths: usize) -> bool {
        if !self.enabled {
            return false;
        }

        match self.capture_mode {
            PathCaptureMode::All => true,
            PathCaptureMode::Sample { count, seed } => {
                // Use hash-based sampling for determinism
                // This ensures same paths are selected across runs
                // Use a proper hash function that provides good distribution
                let mut hash = path_id as u64;
                hash = hash.wrapping_mul(0x9e3779b97f4a7c15); // Multiplicative hash constant
                hash ^= seed;
                hash = hash.wrapping_mul(0x9e3779b97f4a7c15);
                hash ^= hash >> 16; // Mix bits
                hash = hash.wrapping_mul(0x85ebca6b);
                hash ^= hash >> 13;
                hash = hash.wrapping_mul(0xc2b2ae35);
                hash ^= hash >> 16;

                let sample_prob = count as f64 / num_paths as f64;
                let threshold = (u64::MAX as f64 * sample_prob) as u64;
                hash < threshold
            }
        }
    }
}

impl Default for PathCaptureConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Maximum number of Monte Carlo paths allowed per simulation run.
pub const MAX_NUM_PATHS: usize = 10_000_000;

/// Stores the runtime configuration for a Monte Carlo pricing run.
///
/// This configuration is consumed by [`McEngine`] and can either be built
/// manually or via [`McEngineBuilder`]. All time values are year fractions.
#[derive(Debug, Clone)]
pub struct McEngineConfig {
    /// Number of paths to simulate (capped at [`MAX_NUM_PATHS`] at runtime)
    pub num_paths: usize,
    /// Random number generator seed
    pub seed: u64,
    /// Time grid for discretization
    pub time_grid: TimeGrid,
    /// Optional target CI half-width for auto-stopping
    pub target_ci_half_width: Option<f64>,
    /// Use parallel execution (requires parallel feature)
    pub use_parallel: bool,
    /// Chunk size for parallel execution
    pub chunk_size: usize,
    /// Path capture configuration
    pub path_capture: PathCaptureConfig,
    /// Use antithetic variance reduction (pair z and -z per step)
    pub antithetic: bool,
}

impl McEngineConfig {
    /// Create a configuration with default runtime options.
    ///
    /// # Arguments
    ///
    /// * `num_paths` - Requested number of Monte Carlo paths. Runtime validation
    ///   requires this to be greater than zero.
    /// * `time_grid` - Simulation grid in year fractions.
    ///
    /// # Returns
    ///
    /// A configuration using seed `42`, adaptive parallel defaults, disabled
    /// path capture, and no antithetic pairing.
    pub fn new(num_paths: usize, time_grid: TimeGrid) -> Self {
        Self {
            num_paths,
            seed: 42,
            time_grid,
            target_ci_half_width: None,
            use_parallel: cfg!(feature = "parallel"),
            chunk_size: 1000,
            path_capture: PathCaptureConfig::default(),
            antithetic: false,
        }
    }

    /// Set the root RNG seed used by the engine.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Set the target 95% CI half-width for serial auto-stopping.
    ///
    /// The engine currently rejects this option when `use_parallel` is `true`.
    pub fn with_target_ci(mut self, target: f64) -> Self {
        self.target_ci_half_width = Some(target);
        self
    }

    /// Enable or disable parallel execution.
    ///
    /// If the crate is built without the `parallel` feature this setter leaves
    /// `use_parallel` as `false` even when `parallel` is `true`.
    pub fn with_parallel(mut self, parallel: bool) -> Self {
        self.use_parallel = parallel && cfg!(feature = "parallel");
        self
    }

    /// Set the parallel chunk size.
    ///
    /// A value of `1000` keeps the engine's adaptive chunking behavior. Runtime
    /// validation rejects `0`.
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }

    /// Install a path-capture configuration for the run.
    pub fn with_path_capture(mut self, config: PathCaptureConfig) -> Self {
        self.path_capture = config;
        self
    }

    /// Enable or disable antithetic path pairing.
    ///
    /// Path capture and antithetic pricing are currently mutually exclusive.
    pub fn with_antithetic(mut self, enabled: bool) -> Self {
        self.antithetic = enabled;
        self
    }

    /// Convenience helper equivalent to `with_path_capture(PathCaptureConfig::all())`.
    pub fn capture_all_paths(mut self) -> Self {
        self.path_capture = PathCaptureConfig::all();
        self
    }

    /// Convenience helper equivalent to `with_path_capture(PathCaptureConfig::sample(count, seed))`.
    pub fn capture_sample_paths(mut self, count: usize, seed: u64) -> Self {
        self.path_capture = PathCaptureConfig::sample(count, seed);
        self
    }
}

/// Builder for [`McEngine`] with ergonomic defaults.
pub struct McEngineBuilder {
    num_paths: usize,
    seed: u64,
    time_grid: Option<TimeGrid>,
    target_ci: Option<f64>,
    parallel: bool,
    chunk_size: usize,
    path_capture: PathCaptureConfig,
    antithetic: bool,
}

impl McEngineBuilder {
    /// Create a builder with default settings.
    ///
    /// The builder defaults to `100_000` paths, seed `42`, the crate's parallel
    /// default, and no time grid. You must provide a valid grid via
    /// [`Self::time_grid`] or [`Self::uniform_grid`] before calling [`Self::build`].
    pub fn new() -> Self {
        Self {
            num_paths: 100_000,
            seed: 42,
            time_grid: None,
            target_ci: None,
            parallel: cfg!(feature = "parallel"),
            chunk_size: 1000,
            path_capture: PathCaptureConfig::default(),
            antithetic: false,
        }
    }

    /// Set the requested number of paths.
    pub fn num_paths(mut self, n: usize) -> Self {
        self.num_paths = n;
        self
    }

    /// Set the root RNG seed.
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Set the simulation time grid.
    pub fn time_grid(mut self, grid: TimeGrid) -> Self {
        self.time_grid = Some(grid);
        self
    }

    /// Set a uniform time grid in year fractions.
    ///
    /// # Arguments
    ///
    /// * `t_max` - Final simulation time in years.
    /// * `num_steps` - Number of time steps between `0` and `t_max`.
    ///
    /// # Returns
    ///
    /// The builder with `time_grid` populated if `TimeGrid::uniform` succeeds.
    /// Invalid inputs leave the builder without a grid, causing
    /// [`Self::build`] to return an error later.
    pub fn uniform_grid(mut self, t_max: f64, num_steps: usize) -> Self {
        self.time_grid = TimeGrid::uniform(t_max, num_steps).ok();
        self
    }

    /// Set the target 95% CI half-width for serial auto-stopping.
    pub fn target_ci(mut self, target: f64) -> Self {
        self.target_ci = Some(target);
        self
    }

    /// Enable or disable parallel execution.
    pub fn parallel(mut self, enable: bool) -> Self {
        self.parallel = enable && cfg!(feature = "parallel");
        self
    }

    /// Set the parallel chunk size.
    pub fn chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }

    /// Install a path-capture configuration.
    pub fn path_capture(mut self, config: PathCaptureConfig) -> Self {
        self.path_capture = config;
        self
    }

    /// Capture every path in the run.
    pub fn capture_all_paths(mut self) -> Self {
        self.path_capture = PathCaptureConfig::all();
        self
    }

    /// Capture a deterministic sample of paths.
    pub fn capture_sample_paths(mut self, count: usize, seed: u64) -> Self {
        self.path_capture = PathCaptureConfig::sample(count, seed);
        self
    }

    /// Enable or disable antithetic path pairing.
    pub fn antithetic(mut self, enable: bool) -> Self {
        self.antithetic = enable;
        self
    }

    /// Build an [`McEngine`] from the accumulated settings.
    ///
    /// # Errors
    ///
    /// Returns an error when no valid time grid has been configured. This
    /// commonly happens when neither [`Self::time_grid`] nor
    /// [`Self::uniform_grid`] was called, or when `uniform_grid` was called with
    /// invalid inputs.
    pub fn build(self) -> Result<McEngine> {
        let time_grid = self.time_grid.ok_or(finstack_core::InputError::Invalid)?;

        let config = McEngineConfig {
            num_paths: self.num_paths,
            seed: self.seed,
            time_grid,
            target_ci_half_width: self.target_ci,
            use_parallel: self.parallel,
            chunk_size: self.chunk_size,
            path_capture: self.path_capture,
            antithetic: self.antithetic,
        };

        Ok(McEngine { config })
    }
}

impl Default for McEngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Generic Monte Carlo execution engine.
///
/// [`McEngine`] prices a payoff by simulating discounted path values under a
/// supplied process and discretization scheme. It can run serially or in
/// parallel and optionally capture paths for diagnostics.
pub struct McEngine {
    config: McEngineConfig,
}

/// Calculate adaptive chunk size for parallel MC execution.
///
/// Balances load distribution across cores with cache efficiency.
/// Target: 4 chunks per thread for good load balancing.
#[cfg(feature = "parallel")]
fn adaptive_chunk_size(num_paths: usize) -> usize {
    let num_cpus = rayon::current_num_threads();
    // Target 4 chunks per thread for load balancing
    // Min 100 paths per chunk to amortize overhead
    // Max 10_000 paths to avoid cache thrashing
    (num_paths / (num_cpus * 4)).clamp(100, 10_000)
}

/// Pre-sized chunk index ranges for parallel path loops (avoids `Vec` reallocations).
#[cfg(feature = "parallel")]
fn parallel_path_chunks(num_paths: usize, chunk_size: usize) -> Vec<Range<usize>> {
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

    /// Borrow the engine configuration.
    pub fn config(&self) -> &McEngineConfig {
        &self.config
    }

    fn validate_runtime<R, P>(
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
    /// may include a [`PathDataset`].
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
        .with_std_dev(stats.std_dev()))
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
        let chunk_results: Vec<Result<OnlineStats>> = chunks
            .par_iter()
            .map(|range| {
                let mut stats = OnlineStats::new();
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
                        tracing::warn!(
                            path_id,
                            payoff_value,
                            discount_factor,
                            "Skipping non-finite payoff value in MC statistics"
                        );
                    }
                }

                Ok(stats)
            })
            .collect();

        // Collect and handle errors (fail-fast on first error)
        let chunk_stats: Vec<OnlineStats> =
            chunk_results.into_iter().collect::<Result<Vec<_>>>()?;

        // Deterministically reduce chunk statistics
        let mut combined = OnlineStats::new();
        for chunk_stat in chunk_stats {
            combined.merge(&chunk_stat);
        }

        Ok(Estimate::new(
            combined.mean(),
            combined.stderr(),
            combined.confidence_interval(0.05),
            combined.count(),
        )
        .with_std_dev(combined.std_dev()))
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

    /// Serial pricing with path capture.
    #[allow(clippy::too_many_arguments)]
    fn price_serial_with_capture<R, P, D, F>(
        &self,
        rng: &R,
        process: &P,
        disc: &D,
        initial_state: &[f64],
        payoff: &F,
        currency: Currency,
        discount_factor: f64,
        process_params: ProcessParams,
    ) -> Result<(Estimate, Option<PathDataset>)>
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

        // Path capture setup
        let capture_enabled = self.config.path_capture.enabled;
        let sampling_method = match &self.config.path_capture.capture_mode {
            PathCaptureMode::All => PathSamplingMethod::All,
            PathCaptureMode::Sample { count, seed } => PathSamplingMethod::RandomSample {
                count: *count,
                seed: *seed,
            },
        };

        let mut captured_paths = if capture_enabled {
            let estimated_capacity = match self.config.path_capture.capture_mode {
                PathCaptureMode::All => self.config.num_paths,
                PathCaptureMode::Sample { count, .. } => count,
            };
            Vec::with_capacity(estimated_capacity)
        } else {
            Vec::new()
        };

        let mut payoff_local = payoff.clone();

        for path_id in 0..self.config.num_paths {
            let mut path_rng = rng.split(path_id as u64);

            payoff_local.reset();
            payoff_local.on_path_start(&mut path_rng);

            let should_capture = capture_enabled
                && self
                    .config
                    .path_capture
                    .should_capture(path_id, self.config.num_paths);

            let (payoff_value, captured_path) = if should_capture {
                self.simulate_path_with_capture(
                    &mut path_rng,
                    process,
                    disc,
                    initial_state,
                    &mut payoff_local,
                    &mut state,
                    &mut z,
                    &mut work,
                    path_id,
                    discount_factor,
                    currency,
                )?
            } else {
                let val = if self.config.antithetic {
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
                (val, None)
            };

            // Accumulate statistics (skip non-finite values to prevent NaN poisoning)
            let discounted_value = payoff_value * discount_factor;
            if discounted_value.is_finite() {
                stats.update(discounted_value);
            } else {
                tracing::warn!(
                    path_id,
                    payoff_value,
                    discount_factor,
                    "Skipping non-finite payoff value in MC statistics"
                );
            }

            // Store captured path
            if let Some(path) = captured_path {
                captured_paths.push(path);
            }

            // Check auto-stop condition
            if let Some(target) = self.config.target_ci_half_width {
                if stats.count() > 1000 && stats.ci_half_width() < target {
                    break;
                }
            }
        }

        // Compute median and percentiles if paths were captured
        let mut estimate = Estimate::new(
            stats.mean(),
            stats.stderr(),
            stats.confidence_interval(0.05),
            stats.count(),
        )
        .with_std_dev(stats.std_dev());

        let paths = if capture_enabled {
            let mut dataset = PathDataset::new(stats.count(), sampling_method, process_params);
            for path in captured_paths {
                dataset.add_path(path);
            }
            estimate = apply_captured_path_statistics(estimate, &dataset.paths);
            Some(dataset)
        } else {
            None
        };

        Ok((estimate, paths))
    }

    /// Parallel pricing with path capture.
    #[cfg(feature = "parallel")]
    #[allow(clippy::too_many_arguments)]
    fn price_parallel_with_capture<R, P, D, F>(
        &self,
        rng: &R,
        process: &P,
        disc: &D,
        initial_state: &[f64],
        payoff: &F,
        currency: Currency,
        discount_factor: f64,
        process_params: ProcessParams,
    ) -> Result<(Estimate, Option<PathDataset>)>
    where
        R: RandomStream,
        P: StochasticProcess,
        D: Discretization<P>,
        F: Payoff,
    {
        // For parallel execution with path capture, we need thread-safe collection
        let capture_enabled = self.config.path_capture.enabled;
        let sampling_method = match &self.config.path_capture.capture_mode {
            PathCaptureMode::All => PathSamplingMethod::All,
            PathCaptureMode::Sample { count, seed } => PathSamplingMethod::RandomSample {
                count: *count,
                seed: *seed,
            },
        };

        let captured_paths: Mutex<Vec<SimulatedPath>> = Mutex::new(Vec::new());

        // Split paths into chunks for parallel processing
        // Use adaptive chunk size if default (1000), otherwise use configured size
        let effective_chunk_size = if self.config.chunk_size == 1000 {
            adaptive_chunk_size(self.config.num_paths)
        } else {
            self.config.chunk_size
        };

        let chunks = parallel_path_chunks(self.config.num_paths, effective_chunk_size);

        // Process chunks in parallel
        let chunk_results: Vec<Result<OnlineStats>> = chunks
            .par_iter()
            .map(|range| {
                let mut stats = OnlineStats::new();
                let dim = process.dim();
                let num_factors = process.num_factors();
                let work_size = disc.work_size(process);

                let mut state = vec![0.0; dim];
                let mut z = vec![0.0; num_factors];
                let mut work = vec![0.0; work_size];
                let mut state_a = vec![0.0; dim];
                let mut z_anti = vec![0.0; num_factors];
                let mut chunk_paths = if capture_enabled {
                    Vec::with_capacity(range.len() / 10 + 1)
                } else {
                    Vec::new()
                };
                let mut payoff_clone = payoff.clone();

                for path_id in range.clone() {
                    let mut path_rng = rng.split(path_id as u64);
                    payoff_clone.reset();
                    payoff_clone.on_path_start(&mut path_rng);

                    let should_capture = capture_enabled
                        && self
                            .config
                            .path_capture
                            .should_capture(path_id, self.config.num_paths);

                    let (payoff_value, captured_path) = if should_capture {
                        self.simulate_path_with_capture(
                            &mut path_rng,
                            process,
                            disc,
                            initial_state,
                            &mut payoff_clone,
                            &mut state,
                            &mut z,
                            &mut work,
                            path_id,
                            discount_factor,
                            currency,
                        )?
                    } else {
                        let val = if self.config.antithetic {
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
                        (val, None)
                    };

                    let discounted_value = payoff_value * discount_factor;
                    if discounted_value.is_finite() {
                        stats.update(discounted_value);
                    } else {
                        tracing::warn!(
                            path_id,
                            payoff_value,
                            discount_factor,
                            "Skipping non-finite payoff value in MC statistics"
                        );
                    }

                    if let Some(path) = captured_path {
                        chunk_paths.push(path);
                    }
                }

                // Store paths from this chunk
                if !chunk_paths.is_empty() {
                    // SAFETY: A poisoned mutex indicates a prior panic in another thread.
                    // Re-panicking here propagates that failure rather than silently continuing
                    // with potentially corrupted state.
                    #[allow(clippy::expect_used)]
                    captured_paths
                        .lock()
                        .expect("Mutex should not be poisoned")
                        .extend(chunk_paths);
                }

                Ok(stats)
            })
            .collect();

        // Collect and handle errors
        let chunk_stats: Vec<OnlineStats> =
            chunk_results.into_iter().collect::<Result<Vec<_>>>()?;

        // Deterministically reduce chunk statistics
        let mut combined = OnlineStats::new();
        for chunk_stat in chunk_stats {
            combined.merge(&chunk_stat);
        }

        let mut estimate = Estimate::new(
            combined.mean(),
            combined.stderr(),
            combined.confidence_interval(0.05),
            combined.count(),
        )
        .with_std_dev(combined.std_dev());

        let paths = if capture_enabled {
            let mut dataset = PathDataset::new(combined.count(), sampling_method, process_params);
            // SAFETY: A poisoned mutex indicates a prior panic in another thread.
            // Re-panicking here propagates that failure rather than silently continuing
            // with potentially corrupted state.
            #[allow(clippy::expect_used)]
            let mut collected_paths = captured_paths
                .into_inner()
                .expect("Mutex should not be poisoned");
            // Sort by path_id for deterministic ordering across parallel runs
            collected_paths.sort_by_key(|p| p.path_id);
            for path in collected_paths {
                dataset.add_path(path);
            }
            estimate = apply_captured_path_statistics(estimate, &dataset.paths);

            Some(dataset)
        } else {
            None
        };

        Ok((estimate, paths))
    }

    /// Parallel pricing with path capture (fallback).
    #[cfg(not(feature = "parallel"))]
    #[allow(clippy::too_many_arguments)]
    fn price_parallel_with_capture<R, P, D, F>(
        &self,
        rng: &R,
        process: &P,
        disc: &D,
        initial_state: &[f64],
        payoff: &F,
        currency: Currency,
        discount_factor: f64,
        process_params: ProcessParams,
    ) -> Result<(Estimate, Option<PathDataset>)>
    where
        R: RandomStream,
        P: StochasticProcess,
        D: Discretization<P>,
        F: Payoff,
    {
        // Fall back to serial
        self.price_serial_with_capture(
            rng,
            process,
            disc,
            initial_state,
            payoff,
            currency,
            discount_factor,
            process_params,
        )
    }

    /// Simulate a single Monte Carlo path.
    #[allow(clippy::too_many_arguments)]
    fn simulate_path<R, P, D, F>(
        &self,
        rng: &mut R,
        process: &P,
        disc: &D,
        initial_state: &[f64],
        payoff: &mut F,
        state: &mut [f64],
        z: &mut [f64],
        work: &mut [f64],
        currency: Currency,
    ) -> Result<f64>
    where
        R: RandomStream,
        P: StochasticProcess,
        D: Discretization<P>,
        F: Payoff,
    {
        // Initialize state
        state.copy_from_slice(initial_state);

        // Create initial path state
        let mut path_state = PathState::new(0, 0.0);
        process.populate_path_state(state, &mut path_state);
        path_state.set_uniform_random(rng.next_u01());
        payoff.on_event(&mut path_state);

        // Simulate path through time steps
        for step in 0..self.config.time_grid.num_steps() {
            let t = self.config.time_grid.time(step);
            let dt = self.config.time_grid.dt(step);

            rng.fill_std_normals(z);
            disc.step(process, t, dt, state, z, work);

            path_state.set_step_time(step + 1, t + dt);
            process.populate_path_state(state, &mut path_state);
            path_state.set_uniform_random(rng.next_u01());

            // Process payoff event
            payoff.on_event(&mut path_state);
        }

        // Extract payoff value (currency will be added by caller)
        let payoff_money = payoff.value(currency);
        Ok(payoff_money.amount())
    }

    /// Simulate a single Monte Carlo path with full capture.
    ///
    /// Returns the payoff value and optionally the captured path data.
    #[allow(clippy::too_many_arguments)]
    fn simulate_path_with_capture<R, P, D, F>(
        &self,
        rng: &mut R,
        process: &P,
        disc: &D,
        initial_state: &[f64],
        payoff: &mut F,
        state: &mut [f64],
        z: &mut [f64],
        work: &mut [f64],
        path_id: usize,
        discount_factor: f64,
        currency: Currency,
    ) -> Result<(f64, Option<SimulatedPath>)>
    where
        R: RandomStream,
        P: StochasticProcess,
        D: Discretization<P>,
        F: Payoff,
    {
        // Initialize state
        state.copy_from_slice(initial_state);

        // Create initial path state for payoff
        let mut path_state = PathState::new(0, 0.0);
        process.populate_path_state(state, &mut path_state);
        path_state.set_uniform_random(rng.next_u01());
        payoff.on_event(&mut path_state);

        // Initialize simulated path after the initial event so step-0 payoff and
        // cashflow state are captured consistently.
        let num_steps = self.config.time_grid.num_steps() + 1; // +1 for initial point
        let mut simulated_path = SimulatedPath::with_capacity(path_id, num_steps);
        let initial_state_vec = SmallVec::from_slice(state);
        let mut initial_point = PathPoint::with_state(0, 0.0, initial_state_vec);
        path_state.drain_cashflows(|time, amount, cf_type| {
            initial_point.add_typed_cashflow(time, amount, cf_type);
        });
        if self.config.path_capture.capture_payoffs {
            let payoff_money = payoff.value(currency);
            initial_point.set_payoff(payoff_money.amount());
        }
        simulated_path.add_point(initial_point);

        // Simulate path through time steps
        for step in 0..self.config.time_grid.num_steps() {
            let t = self.config.time_grid.time(step);
            let dt = self.config.time_grid.dt(step);

            rng.fill_std_normals(z);
            disc.step(process, t, dt, state, z, work);

            path_state.set_step_time(step + 1, t + dt);
            process.populate_path_state(state, &mut path_state);
            path_state.set_uniform_random(rng.next_u01());

            // Process payoff event (payoff may add cashflows to path_state)
            payoff.on_event(&mut path_state);

            // Capture this point with state vector
            let state_vec = SmallVec::from_slice(state);
            let mut point = PathPoint::with_state(step + 1, t + dt, state_vec);

            // Transfer cashflows from PathState to PathPoint
            path_state.drain_cashflows(|time, amount, cf_type| {
                point.add_typed_cashflow(time, amount, cf_type);
            });

            if self.config.path_capture.capture_payoffs {
                // Capture intermediate payoff value (undiscounted)
                let payoff_money = payoff.value(currency);
                point.set_payoff(payoff_money.amount());
            }
            simulated_path.add_point(point);
        }

        // Extract final payoff value
        let payoff_money = payoff.value(currency);
        let payoff_value = payoff_money.amount();

        // Set final discounted value
        simulated_path.set_final_value(payoff_value * discount_factor);

        // Calculate IRR from cashflows (if available)
        let cashflow_amounts = simulated_path.extract_cashflow_amounts();
        if cashflow_amounts.len() >= 2 {
            // Use periodic IRR approximation (assumes roughly equal spacing)
            // Use finstack_core IRR calculation
            use finstack_core::cashflow::InternalRateOfReturn;
            if let Ok(irr) = cashflow_amounts.irr(None) {
                simulated_path.set_irr(irr);
            }
        }

        Ok((payoff_value, Some(simulated_path)))
    }

    /// Simulate one antithetic pair and return the average payoff (in amount).
    #[allow(clippy::too_many_arguments)]
    fn simulate_antithetic_pair<R, P, D, F>(
        &self,
        rng: &mut R,
        process: &P,
        disc: &D,
        initial_state: &[f64],
        payoff: &mut F,
        state_p: &mut [f64],
        state_a: &mut [f64],
        z: &mut [f64],
        z_anti: &mut [f64],
        work: &mut [f64],
        currency: Currency,
    ) -> Result<f64>
    where
        R: RandomStream,
        P: StochasticProcess,
        D: Discretization<P>,
        F: Payoff,
    {
        // Primary path state and payoff
        state_p.copy_from_slice(initial_state);
        let mut payoff_p = payoff.clone();
        let mut path_state_p = PathState::new(0, 0.0);
        process.populate_path_state(state_p, &mut path_state_p);
        let u_init = rng.next_u01();
        path_state_p.set_uniform_random(u_init);
        payoff_p.on_event(&mut path_state_p);

        // Antithetic path state and payoff
        state_a.copy_from_slice(initial_state);
        let mut payoff_a = payoff.clone();
        let mut path_state_a = PathState::new(0, 0.0);
        process.populate_path_state(state_a, &mut path_state_a);
        path_state_a.set_uniform_random(1.0 - u_init);
        payoff_a.on_event(&mut path_state_a);

        for step in 0..self.config.time_grid.num_steps() {
            let t = self.config.time_grid.time(step);
            let dt = self.config.time_grid.dt(step);

            rng.fill_std_normals(z);
            for i in 0..z.len() {
                z_anti[i] = -z[i];
            }

            disc.step(process, t, dt, state_p, z, work);
            disc.step(process, t, dt, state_a, z_anti, work);

            let u_step = rng.next_u01();

            path_state_p.set_step_time(step + 1, t + dt);
            process.populate_path_state(state_p, &mut path_state_p);
            path_state_p.set_uniform_random(u_step);
            payoff_p.on_event(&mut path_state_p);

            path_state_a.set_step_time(step + 1, t + dt);
            process.populate_path_state(state_a, &mut path_state_a);
            path_state_a.set_uniform_random(1.0 - u_step);
            payoff_a.on_event(&mut path_state_a);
        }

        let v_p = payoff_p.value(currency).amount();
        let v_a = payoff_a.value(currency).amount();
        Ok(0.5 * (v_p + v_a))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::paths::CashflowType;
    use finstack_core::money::Money;

    // Dummy implementations for testing
    #[derive(Clone)]
    struct DummyRng;
    impl RandomStream for DummyRng {
        fn split(&self, _id: u64) -> Self {
            DummyRng
        }
        fn fill_u01(&mut self, out: &mut [f64]) {
            for x in out {
                *x = 0.5;
            }
        }
        fn fill_std_normals(&mut self, out: &mut [f64]) {
            for x in out {
                *x = 0.0;
            }
        }
    }

    #[derive(Clone)]
    struct PathIndexedRng {
        path_id: u64,
    }

    impl PathIndexedRng {
        fn root() -> Self {
            Self { path_id: 0 }
        }
    }

    impl RandomStream for PathIndexedRng {
        fn split(&self, stream_id: u64) -> Self {
            Self { path_id: stream_id }
        }

        fn fill_u01(&mut self, out: &mut [f64]) {
            let value = (self.path_id + 1) as f64 / 8.0;
            for x in out {
                *x = value;
            }
        }

        fn fill_std_normals(&mut self, out: &mut [f64]) {
            for x in out {
                *x = 0.0;
            }
        }
    }

    struct DummyProcess;
    impl StochasticProcess for DummyProcess {
        fn dim(&self) -> usize {
            1
        }
        fn drift(&self, _t: f64, _x: &[f64], out: &mut [f64]) {
            out[0] = 0.0;
        }
        fn diffusion(&self, _t: f64, _x: &[f64], out: &mut [f64]) {
            out[0] = 0.1;
        }
    }

    struct DummyDisc;
    impl Discretization<DummyProcess> for DummyDisc {
        fn step(
            &self,
            _process: &DummyProcess,
            _t: f64,
            _dt: f64,
            _x: &mut [f64],
            _z: &[f64],
            _work: &mut [f64],
        ) {
            // Just keep state constant
        }
    }

    #[derive(Clone)]
    struct DummyPayoff;
    impl Payoff for DummyPayoff {
        fn on_event(&mut self, _state: &mut PathState) {}
        fn value(&self, currency: Currency) -> Money {
            Money::new(100.0, currency)
        }
        fn reset(&mut self) {}
    }

    #[derive(Clone, Default)]
    struct PathStartPayoff {
        start_uniform: Option<f64>,
    }

    impl Payoff for PathStartPayoff {
        fn on_path_start<R: RandomStream>(&mut self, rng: &mut R) {
            self.start_uniform = Some(rng.next_u01());
        }

        fn on_event(&mut self, _state: &mut PathState) {}

        fn value(&self, currency: Currency) -> Money {
            Money::new(self.start_uniform.unwrap_or(-1.0), currency)
        }

        fn reset(&mut self) {
            self.start_uniform = None;
        }
    }

    #[derive(Clone, Default)]
    struct CapturedValuePayoff {
        value: Option<f64>,
    }

    impl Payoff for CapturedValuePayoff {
        fn on_path_start<R: RandomStream>(&mut self, rng: &mut R) {
            self.value = Some(rng.next_u01());
        }

        fn on_event(&mut self, _state: &mut PathState) {}

        fn value(&self, currency: Currency) -> Money {
            Money::new(self.value.unwrap_or_default(), currency)
        }

        fn reset(&mut self) {
            self.value = None;
        }
    }

    #[derive(Clone)]
    struct InitialCashflowPayoff {
        value: f64,
    }

    impl Payoff for InitialCashflowPayoff {
        fn on_event(&mut self, state: &mut PathState) {
            if state.step == 0 {
                state.add_cashflow(state.time, self.value);
            }
        }

        fn value(&self, currency: Currency) -> Money {
            Money::new(self.value, currency)
        }

        fn reset(&mut self) {}
    }

    #[derive(Clone, Default)]
    struct RecurringCashflowPayoff;

    impl Payoff for RecurringCashflowPayoff {
        fn on_event(&mut self, state: &mut PathState) {
            state.add_typed_cashflow(state.time, state.step as f64 + 1.0, CashflowType::Interest);
        }

        fn value(&self, currency: Currency) -> Money {
            Money::new(0.0, currency)
        }

        fn reset(&mut self) {}
    }

    #[test]
    fn test_engine_builder() {
        let engine = McEngine::builder()
            .num_paths(1000)
            .seed(42)
            .uniform_grid(1.0, 100)
            .build()
            .expect("McEngine builder should succeed with valid test data");

        assert_eq!(engine.config.num_paths, 1000);
        assert_eq!(engine.config.seed, 42);
    }

    #[test]
    fn test_basic_pricing() {
        let engine = McEngine::builder()
            .num_paths(100)
            .uniform_grid(1.0, 10)
            .parallel(false)
            .build()
            .expect("McEngine builder should succeed with valid test data");

        let rng = DummyRng;
        let process = DummyProcess;
        let disc = DummyDisc;
        let initial_state = vec![100.0];
        let payoff = DummyPayoff;

        let result = engine
            .price(
                &rng,
                &process,
                &disc,
                &initial_state,
                &payoff,
                Currency::USD,
                1.0,
            )
            .expect("should succeed");

        assert_eq!(result.mean.amount(), 100.0);
        assert_eq!(result.num_paths, 100);
    }

    #[test]
    #[cfg(feature = "parallel")]
    fn test_parallel_execution_error_propagation() {
        // Test that parallel execution properly propagates errors instead of panicking.
        // The key change is that we replaced .expect() with ? operator, which ensures
        // errors are propagated via Result rather than panicking.
        //
        // This test verifies that:
        // 1. Parallel execution works correctly for valid inputs
        // 2. Error handling mechanism is in place (verified by compilation - ? operator
        //    requires Result return type)

        let engine = McEngine::builder()
            .num_paths(100)
            .uniform_grid(1.0, 10)
            .parallel(true)
            .chunk_size(50)
            .build()
            .expect("McEngine builder should succeed with valid test data");

        let rng = DummyRng;
        let process = DummyProcess;
        let disc = DummyDisc;
        let initial_state = vec![100.0];
        let payoff = DummyPayoff;

        // Valid input should work
        let result = engine.price(
            &rng,
            &process,
            &disc,
            &initial_state,
            &payoff,
            Currency::USD,
            1.0,
        );

        assert!(result.is_ok());
        let estimate = result.expect("MC pricing should succeed in test");
        assert_eq!(estimate.num_paths, 100);

        // Note: Testing actual error scenarios would require extensive mocking
        // of simulate_path. The important change is that errors are now propagated
        // via Result instead of panicking (verified by ? operator usage).
    }

    #[test]
    fn test_serial_vs_parallel_consistency() {
        // Test that serial and parallel paths produce consistent results
        let engine_serial = McEngine::builder()
            .num_paths(1000)
            .uniform_grid(1.0, 10)
            .seed(42)
            .parallel(false)
            .build()
            .expect("McEngine builder should succeed with valid test data");

        #[cfg(feature = "parallel")]
        let engine_parallel = McEngine::builder()
            .num_paths(1000)
            .uniform_grid(1.0, 10)
            .seed(42)
            .parallel(true)
            .chunk_size(200)
            .build()
            .expect("McEngine builder should succeed with valid test data");

        let rng_serial = DummyRng;
        #[cfg(feature = "parallel")]
        let rng_parallel = DummyRng;
        let process = DummyProcess;
        let disc = DummyDisc;
        let initial_state = vec![100.0];
        let payoff = DummyPayoff;

        let serial_result = engine_serial
            .price(
                &rng_serial,
                &process,
                &disc,
                &initial_state,
                &payoff,
                Currency::USD,
                1.0,
            )
            .expect("should succeed");

        #[cfg(feature = "parallel")]
        let parallel_result = engine_parallel
            .price(
                &rng_parallel,
                &process,
                &disc,
                &initial_state,
                &payoff,
                Currency::USD,
                1.0,
            )
            .expect("should succeed");

        // Both should succeed and produce same results (deterministic RNG)
        assert_eq!(serial_result.num_paths, 1000);
        #[cfg(feature = "parallel")]
        assert_eq!(parallel_result.num_paths, 1000);
        #[cfg(feature = "parallel")]
        assert_eq!(serial_result.mean.amount(), parallel_result.mean.amount());
    }

    /// A minimal RNG that declares it does not support splitting (mimicking SobolRng).
    #[derive(Clone)]
    struct NonSplittableRng;
    impl RandomStream for NonSplittableRng {
        fn split(&self, _id: u64) -> Self {
            NonSplittableRng
        }
        fn fill_u01(&mut self, out: &mut [f64]) {
            for x in out {
                *x = 0.5;
            }
        }
        fn fill_std_normals(&mut self, out: &mut [f64]) {
            for x in out {
                *x = 0.0;
            }
        }
        fn supports_splitting(&self) -> bool {
            false
        }
    }

    #[test]
    fn test_parallel_with_non_splittable_rng_returns_error() {
        // Guard: McEngine::price() must return Err when use_parallel=true and
        // rng.supports_splitting() == false.
        let engine = McEngine::builder()
            .num_paths(100)
            .uniform_grid(1.0, 10)
            .parallel(true)
            .build()
            .expect("McEngine builder should succeed with valid test data");

        let rng = NonSplittableRng;
        let process = DummyProcess;
        let disc = DummyDisc;
        let initial_state = vec![100.0];
        let payoff = DummyPayoff;

        let result = engine.price(
            &rng,
            &process,
            &disc,
            &initial_state,
            &payoff,
            Currency::USD,
            1.0,
        );

        // When the parallel feature is enabled this must be an Err; when it is
        // disabled the engine falls back to serial, so the guard is never
        // reached and the call succeeds.
        #[cfg(feature = "parallel")]
        {
            assert!(
                result.is_err(),
                "Expected Err for parallel + non-splittable RNG, got Ok"
            );
            let err = result.expect_err("parallel + non-splittable RNG should return an error");
            let err_str = err.to_string();
            assert!(
                err_str.contains("splittable RNG"),
                "Error message should mention splittable RNG, got: {err_str}"
            );
        }
        #[cfg(not(feature = "parallel"))]
        {
            // Serial fallback — guard never fires
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_price_with_capture_parallel_non_splittable_returns_error() {
        // Same guard must fire in price_with_capture().
        let engine = McEngine::builder()
            .num_paths(100)
            .uniform_grid(1.0, 10)
            .parallel(true)
            .build()
            .expect("McEngine builder should succeed with valid test data");

        let rng = NonSplittableRng;
        let process = DummyProcess;
        let disc = DummyDisc;
        let initial_state = vec![100.0];
        let payoff = DummyPayoff;
        let params = ProcessParams::new("test");

        let result = engine.price_with_capture(
            &rng,
            &process,
            &disc,
            &initial_state,
            &payoff,
            Currency::USD,
            1.0,
            params,
        );

        #[cfg(feature = "parallel")]
        {
            assert!(
                result.is_err(),
                "Expected Err for parallel + non-splittable RNG in price_with_capture, got Ok"
            );
        }
        #[cfg(not(feature = "parallel"))]
        {
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_on_path_start_state_survives_into_simulation() {
        let engine = McEngine::builder()
            .num_paths(1)
            .uniform_grid(1.0, 1)
            .parallel(false)
            .build()
            .expect("McEngine builder should succeed with valid test data");

        let result = engine
            .price(
                &DummyRng,
                &DummyProcess,
                &DummyDisc,
                &[100.0],
                &PathStartPayoff::default(),
                Currency::USD,
                1.0,
            )
            .expect("pricing should succeed");

        assert_eq!(result.mean.amount(), 0.5);
    }

    #[test]
    fn test_price_rejects_zero_paths() {
        let time_grid = TimeGrid::uniform(1.0, 1).expect("grid should build");
        let engine = McEngine::new(McEngineConfig {
            num_paths: 0,
            seed: 42,
            time_grid,
            target_ci_half_width: None,
            use_parallel: false,
            chunk_size: 1,
            path_capture: PathCaptureConfig::disabled(),
            antithetic: false,
        });

        let err = engine
            .price(
                &DummyRng,
                &DummyProcess,
                &DummyDisc,
                &[100.0],
                &DummyPayoff,
                Currency::USD,
                1.0,
            )
            .expect_err("zero-path configuration should be rejected");

        assert!(err.to_string().contains("num_paths"));
    }

    #[test]
    fn test_price_rejects_zero_chunk_size() {
        let time_grid = TimeGrid::uniform(1.0, 1).expect("grid should build");
        let engine = McEngine::new(McEngineConfig {
            num_paths: 10,
            seed: 42,
            time_grid,
            target_ci_half_width: None,
            use_parallel: false,
            chunk_size: 0,
            path_capture: PathCaptureConfig::disabled(),
            antithetic: false,
        });

        let err = engine
            .price(
                &DummyRng,
                &DummyProcess,
                &DummyDisc,
                &[100.0],
                &DummyPayoff,
                Currency::USD,
                1.0,
            )
            .expect_err("zero chunk size should be rejected");

        assert!(err.to_string().contains("chunk_size"));
    }

    #[test]
    fn test_price_rejects_initial_state_dimension_mismatch() {
        let engine = McEngine::builder()
            .num_paths(10)
            .uniform_grid(1.0, 1)
            .parallel(false)
            .build()
            .expect("McEngine builder should succeed with valid test data");

        let err = engine
            .price(
                &DummyRng,
                &DummyProcess,
                &DummyDisc,
                &[],
                &DummyPayoff,
                Currency::USD,
                1.0,
            )
            .expect_err("state dimension mismatch should be rejected");

        assert!(err.to_string().contains("initial_state"));
    }

    #[test]
    fn test_price_with_capture_rejects_invalid_sample_count() {
        let engine = McEngine::new(McEngineConfig {
            num_paths: 10,
            seed: 42,
            time_grid: TimeGrid::uniform(1.0, 1).expect("grid should build"),
            target_ci_half_width: None,
            use_parallel: false,
            chunk_size: 1,
            path_capture: PathCaptureConfig::sample(0, 99),
            antithetic: false,
        });

        let err = engine
            .price_with_capture(
                &DummyRng,
                &DummyProcess,
                &DummyDisc,
                &[100.0],
                &DummyPayoff,
                Currency::USD,
                1.0,
                ProcessParams::new("test"),
            )
            .expect_err("zero sample count should be rejected");

        assert!(err.to_string().contains("sample"));
    }

    #[test]
    fn test_price_with_capture_rejects_antithetic_capture_combination() {
        let engine = McEngine::new(McEngineConfig {
            num_paths: 10,
            seed: 42,
            time_grid: TimeGrid::uniform(1.0, 1).expect("grid should build"),
            target_ci_half_width: None,
            use_parallel: false,
            chunk_size: 1,
            path_capture: PathCaptureConfig::all(),
            antithetic: true,
        });

        let err = engine
            .price_with_capture(
                &DummyRng,
                &DummyProcess,
                &DummyDisc,
                &[100.0],
                &DummyPayoff,
                Currency::USD,
                1.0,
                ProcessParams::new("test"),
            )
            .expect_err("antithetic + path capture should be rejected");

        assert!(err.to_string().contains("antithetic"));
    }

    #[test]
    fn test_price_rejects_parallel_auto_stop_configuration() {
        let time_grid = TimeGrid::uniform(1.0, 1).expect("grid should build");
        let engine = McEngine::new(McEngineConfig {
            num_paths: 10,
            seed: 42,
            time_grid,
            target_ci_half_width: Some(0.01),
            use_parallel: true,
            chunk_size: 2,
            path_capture: PathCaptureConfig::disabled(),
            antithetic: false,
        });

        let result = engine.price(
            &DummyRng,
            &DummyProcess,
            &DummyDisc,
            &[100.0],
            &DummyPayoff,
            Currency::USD,
            1.0,
        );

        #[cfg(feature = "parallel")]
        {
            let err = result.expect_err("parallel auto-stop should be rejected");
            assert!(err.to_string().contains("target_ci_half_width"));
        }
        #[cfg(not(feature = "parallel"))]
        {
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_price_with_capture_captures_initial_event_cashflows_and_payoff() {
        let engine = McEngine::new(McEngineConfig {
            num_paths: 1,
            seed: 42,
            time_grid: TimeGrid::uniform(1.0, 1).expect("grid should build"),
            target_ci_half_width: None,
            use_parallel: false,
            chunk_size: 1,
            path_capture: PathCaptureConfig::all().with_payoffs(),
            antithetic: false,
        });

        let result = engine
            .price_with_capture(
                &DummyRng,
                &DummyProcess,
                &DummyDisc,
                &[100.0],
                &InitialCashflowPayoff { value: 7.0 },
                Currency::USD,
                1.0,
                ProcessParams::new("test"),
            )
            .expect("capture should succeed");

        let path = result
            .paths()
            .and_then(|dataset| dataset.path(0))
            .expect("captured path should exist");
        let initial_point = path.initial_point().expect("initial point should exist");
        assert_eq!(initial_point.payoff_value, Some(7.0));
        assert_eq!(
            initial_point.cashflows,
            vec![(0.0, 7.0, CashflowType::Other)]
        );
    }

    #[test]
    fn test_price_with_capture_preserves_cashflows_across_multiple_timesteps() {
        let engine = McEngine::new(McEngineConfig {
            num_paths: 1,
            seed: 42,
            time_grid: TimeGrid::uniform(1.0, 2).expect("grid should build"),
            target_ci_half_width: None,
            use_parallel: false,
            chunk_size: 1000,
            path_capture: PathCaptureConfig::all(),
            antithetic: false,
        });

        let result = engine
            .price_with_capture(
                &DummyRng,
                &DummyProcess,
                &DummyDisc,
                &[100.0],
                &RecurringCashflowPayoff,
                Currency::USD,
                1.0,
                ProcessParams::new("test"),
            )
            .expect("captured pricing should succeed");

        let path = result
            .paths
            .as_ref()
            .and_then(|dataset| dataset.paths.first())
            .expect("captured path should exist");
        assert_eq!(path.points.len(), 3);
        assert_eq!(
            path.points[0].cashflows,
            vec![(0.0, 1.0, CashflowType::Interest)]
        );
        assert_eq!(
            path.points[1].cashflows,
            vec![(0.5, 2.0, CashflowType::Interest)]
        );
        assert_eq!(
            path.points[2].cashflows,
            vec![(1.0, 3.0, CashflowType::Interest)]
        );
    }

    #[test]
    fn test_price_with_capture_uses_actual_path_count_after_auto_stop() {
        let engine = McEngine::new(McEngineConfig {
            num_paths: 5_000,
            seed: 42,
            time_grid: TimeGrid::uniform(1.0, 1).expect("grid should build"),
            target_ci_half_width: Some(0.01),
            use_parallel: false,
            chunk_size: 100,
            path_capture: PathCaptureConfig::all(),
            antithetic: false,
        });

        let result = engine
            .price_with_capture(
                &DummyRng,
                &DummyProcess,
                &DummyDisc,
                &[100.0],
                &DummyPayoff,
                Currency::USD,
                1.0,
                ProcessParams::new("test"),
            )
            .expect("pricing should succeed");

        let captured = result.paths().expect("captured paths should exist");
        assert_eq!(result.estimate.num_paths, 1001);
        assert_eq!(captured.num_paths_total, 1001);
        assert_eq!(captured.num_captured(), 1001);
    }

    fn assert_captured_path_statistics(result: &MonteCarloResult) {
        assert_eq!(result.estimate.median, Some(0.375));
        assert_eq!(result.estimate.percentile_25, Some(0.25));
        assert_eq!(result.estimate.percentile_75, Some(0.5));
        assert_eq!(result.estimate.min, Some(0.125));
        assert_eq!(result.estimate.max, Some(0.625));
    }

    #[test]
    fn test_price_with_capture_serial_populates_captured_path_statistics() {
        let engine = McEngine::builder()
            .num_paths(5)
            .uniform_grid(1.0, 1)
            .parallel(false)
            .capture_all_paths()
            .build()
            .expect("McEngine builder should succeed with valid test data");

        let result = engine
            .price_with_capture(
                &PathIndexedRng::root(),
                &DummyProcess,
                &DummyDisc,
                &[100.0],
                &CapturedValuePayoff::default(),
                Currency::USD,
                1.0,
                ProcessParams::new("test"),
            )
            .expect("captured pricing should succeed");

        assert_captured_path_statistics(&result);
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn test_price_with_capture_parallel_populates_captured_path_statistics() {
        let engine = McEngine::builder()
            .num_paths(5)
            .uniform_grid(1.0, 1)
            .parallel(true)
            .chunk_size(2)
            .capture_all_paths()
            .build()
            .expect("McEngine builder should succeed with valid test data");

        let result = engine
            .price_with_capture(
                &PathIndexedRng::root(),
                &DummyProcess,
                &DummyDisc,
                &[100.0],
                &CapturedValuePayoff::default(),
                Currency::USD,
                1.0,
                ProcessParams::new("test"),
            )
            .expect("captured pricing should succeed");

        assert_captured_path_statistics(&result);
    }
}
