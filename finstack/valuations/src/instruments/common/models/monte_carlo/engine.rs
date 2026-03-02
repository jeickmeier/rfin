//! Monte Carlo simulation engine.
//!
//! Provides the core execution harness for Monte Carlo simulation with:
//! - Structure of Arrays (SoA) layout for cache efficiency
//! - Rayon-based parallelism with deterministic reduction
//! - Online statistics via Welford's algorithm
//! - Variance reduction integration
//! - Auto-stopping on target confidence interval
//! - Optional path capture for visualization and diagnostics

use super::results::{MoneyEstimate, MonteCarloResult};
use super::traits::Payoff;
use crate::instruments::common_impl::models::monte_carlo::estimate::Estimate;
use crate::instruments::common_impl::models::monte_carlo::online_stats::OnlineStats;
use crate::instruments::common_impl::models::monte_carlo::paths::{
    PathDataset, PathPoint, PathSamplingMethod, ProcessParams, SimulatedPath,
};
use crate::instruments::common_impl::models::monte_carlo::time_grid::TimeGrid;
use crate::instruments::common_impl::models::monte_carlo::traits::{
    Discretization, PathState, RandomStream, StochasticProcess,
};
use finstack_core::currency::Currency;
use finstack_core::Result;
use smallvec::SmallVec;
use std::sync::Mutex;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Path capture mode for Monte Carlo simulation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathCaptureMode {
    /// Capture all paths
    All,
    /// Capture a random sample of paths
    Sample {
        /// Number of Monte Carlo paths
        count: usize,
        /// Random seed for reproducibility
        seed: u64,
    },
}

/// Configuration for path capture during Monte Carlo simulation.
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
    /// Create a new path capture config (disabled by default).
    pub fn new() -> Self {
        Self {
            enabled: false,
            capture_mode: PathCaptureMode::All,
            capture_payoffs: false,
        }
    }

    /// Enable path capture for all paths.
    pub fn all() -> Self {
        Self {
            enabled: true,
            capture_mode: PathCaptureMode::All,
            capture_payoffs: false,
        }
    }

    /// Enable path capture for a sample of paths.
    pub fn sample(count: usize, seed: u64) -> Self {
        Self {
            enabled: true,
            capture_mode: PathCaptureMode::Sample { count, seed },
            capture_payoffs: false,
        }
    }

    /// Enable payoff capture at each timestep.
    pub fn with_payoffs(mut self) -> Self {
        self.capture_payoffs = true;
        self
    }

    /// Disable path capture.
    pub fn disabled() -> Self {
        Self::new()
    }

    /// Check if a path should be captured based on path_id.
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

/// Monte Carlo engine configuration.
#[derive(Debug, Clone)]
pub struct McEngineConfig {
    /// Number of paths to simulate
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
    /// Create a new configuration with defaults.
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

    /// Set random seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Set target CI half-width for auto-stopping.
    pub fn with_target_ci(mut self, target: f64) -> Self {
        self.target_ci_half_width = Some(target);
        self
    }

    /// Enable/disable parallel execution.
    pub fn with_parallel(mut self, parallel: bool) -> Self {
        self.use_parallel = parallel && cfg!(feature = "parallel");
        self
    }

    /// Set parallel chunk size.
    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }

    /// Set path capture configuration.
    pub fn with_path_capture(mut self, config: PathCaptureConfig) -> Self {
        self.path_capture = config;
        self
    }

    /// Enable/disable antithetic variance reduction.
    pub fn with_antithetic(mut self, enabled: bool) -> Self {
        self.antithetic = enabled;
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
}

/// Monte Carlo engine builder.
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
    /// Create a new builder.
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

    /// Set number of paths.
    pub fn num_paths(mut self, n: usize) -> Self {
        self.num_paths = n;
        self
    }

    /// Set random seed.
    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Set time grid (required).
    pub fn time_grid(mut self, grid: TimeGrid) -> Self {
        self.time_grid = Some(grid);
        self
    }

    /// Set uniform time grid (convenience).
    pub fn uniform_grid(mut self, t_max: f64, num_steps: usize) -> Self {
        self.time_grid = TimeGrid::uniform(t_max, num_steps).ok();
        self
    }

    /// Set target CI half-width for auto-stopping.
    pub fn target_ci(mut self, target: f64) -> Self {
        self.target_ci = Some(target);
        self
    }

    /// Enable/disable parallel execution.
    pub fn parallel(mut self, enable: bool) -> Self {
        self.parallel = enable && cfg!(feature = "parallel");
        self
    }

    /// Set parallel chunk size.
    pub fn chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }

    /// Set path capture configuration.
    pub fn path_capture(mut self, config: PathCaptureConfig) -> Self {
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

    /// Enable/disable antithetic variance reduction.
    pub fn antithetic(mut self, enable: bool) -> Self {
        self.antithetic = enable;
        self
    }

    /// Build the engine.
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

/// Monte Carlo simulation engine.
///
/// The engine executes Monte Carlo simulation for a given process,
/// discretization, and payoff, returning statistical estimates.
pub struct McEngine {
    config: McEngineConfig,
}

/// Calculate adaptive chunk size for parallel MC execution.
///
/// Balances load distribution across cores with cache efficiency.
/// Target: 4 chunks per thread for good load balancing.
fn adaptive_chunk_size(num_paths: usize) -> usize {
    #[cfg(feature = "parallel")]
    {
        let num_cpus = rayon::current_num_threads();
        // Target 4 chunks per thread for load balancing
        // Min 100 paths per chunk to amortize overhead
        // Max 10_000 paths to avoid cache thrashing
        (num_paths / (num_cpus * 4)).clamp(100, 10_000)
    }
    #[cfg(not(feature = "parallel"))]
    {
        // Serial execution - use full batch
        num_paths
    }
}

impl McEngine {
    /// Create a builder for the engine.
    pub fn builder() -> McEngineBuilder {
        McEngineBuilder::new()
    }

    /// Create a new engine with the given configuration.
    pub fn new(config: McEngineConfig) -> Self {
        Self { config }
    }

    /// Get the engine configuration.
    pub fn config(&self) -> &McEngineConfig {
        &self.config
    }

    /// Price a payoff using Monte Carlo simulation.
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
    /// * `rng` - Random number generator
    /// * `process` - Stochastic process
    /// * `disc` - Discretization scheme
    /// * `initial_state` - Initial state vector
    /// * `payoff` - Payoff specification
    /// * `currency` - Currency for result
    /// * `discount_factor` - Optional discount factor (default 1.0)
    ///
    /// # Returns
    ///
    /// Statistical estimate with mean, stderr, and confidence interval.
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

    /// Price with path capture support.
    ///
    /// This method is identical to `price` but returns a `MonteCarloResult` which
    /// includes optional captured paths based on the engine configuration.
    ///
    /// # Arguments
    ///
    /// * `process_params` - Process parameters metadata for path dataset
    ///
    /// For other arguments, see `price`.
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

        for path_id in 0..self.config.num_paths {
            // Create independent RNG stream for this path
            let mut path_rng = rng.split(path_id as u64);

            // Clone payoff for this path (required since Payoff trait needs &mut self)
            let mut payoff_clone = payoff.clone();
            // Allow payoff to draw per-path state (e.g., default threshold)
            payoff_clone.on_path_start(&mut path_rng);

            // Simulate one path
            let payoff_value = if self.config.antithetic {
                self.simulate_antithetic_pair(
                    &mut path_rng,
                    process,
                    disc,
                    initial_state,
                    &mut payoff_clone,
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

            // Accumulate statistics
            let discounted_value = payoff_value * discount_factor;
            stats.update(discounted_value);

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

        let chunks: Vec<_> = (0..self.config.num_paths)
            .step_by(effective_chunk_size)
            .map(|start| {
                let end = (start + effective_chunk_size).min(self.config.num_paths);
                start..end
            })
            .collect();

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

                for path_id in range.clone() {
                    let mut path_rng = rng.split(path_id as u64);

                    // Clone payoff for this path
                    let mut payoff_clone = payoff.clone();
                    payoff_clone.on_path_start(&mut path_rng);

                    // Use ? operator to propagate errors instead of panicking
                    let payoff_value = if self.config.antithetic {
                        self.simulate_antithetic_pair(
                            &mut path_rng,
                            process,
                            disc,
                            initial_state,
                            &mut payoff_clone,
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
                    stats.update(discounted_value);
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

        // Path capture setup
        let capture_enabled = self.config.path_capture.enabled;
        let sampling_method = match &self.config.path_capture.capture_mode {
            PathCaptureMode::All => PathSamplingMethod::All,
            PathCaptureMode::Sample { count, seed } => PathSamplingMethod::RandomSample {
                count: *count,
                seed: *seed,
            },
        };

        let mut paths = if capture_enabled {
            Some(PathDataset::new(
                self.config.num_paths,
                sampling_method,
                process_params,
            ))
        } else {
            None
        };

        for path_id in 0..self.config.num_paths {
            // Create independent RNG stream for this path
            let mut path_rng = rng.split(path_id as u64);

            // Clone payoff for this path
            let mut payoff_clone = payoff.clone();
            payoff_clone.on_path_start(&mut path_rng);

            // Determine if we should capture this path
            let should_capture = capture_enabled
                && self
                    .config
                    .path_capture
                    .should_capture(path_id, self.config.num_paths);

            // Simulate path with optional capture
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

            // Accumulate statistics
            let discounted_value = payoff_value * discount_factor;
            stats.update(discounted_value);

            // Store captured path
            if let (Some(ref mut dataset), Some(path)) = (&mut paths, captured_path) {
                dataset.add_path(path);
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

        // If we have captured paths, calculate additional statistics
        if let Some(ref dataset) = paths {
            let mut values: Vec<f64> = dataset.paths.iter().map(|p| p.final_value).collect();

            if !values.is_empty() {
                values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let n = values.len();

                // Median
                let median = if n.is_multiple_of(2) {
                    (values[n / 2 - 1] + values[n / 2]) / 2.0
                } else {
                    values[n / 2]
                };

                // Percentiles
                let p25_idx = (n as f64 * 0.25).floor() as usize;
                let p75_idx = (n as f64 * 0.75).floor() as usize;
                let p25 = values[p25_idx.min(n - 1)];
                let p75 = values[p75_idx.min(n - 1)];

                // Min/Max
                let min = values[0];
                let max = values[n - 1];

                estimate = estimate
                    .with_median(median)
                    .with_percentiles(p25, p75)
                    .with_range(min, max);
            }
        }

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

        let chunks: Vec<_> = (0..self.config.num_paths)
            .step_by(effective_chunk_size)
            .map(|start| {
                let end = (start + effective_chunk_size).min(self.config.num_paths);
                start..end
            })
            .collect();

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
                let mut chunk_paths = Vec::new();

                for path_id in range.clone() {
                    let mut path_rng = rng.split(path_id as u64);
                    let mut payoff_clone = payoff.clone();
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
                    stats.update(discounted_value);

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
            let mut dataset =
                PathDataset::new(self.config.num_paths, sampling_method, process_params);
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

            // Compute additional statistics from captured paths
            let mut values: Vec<f64> = dataset.paths.iter().map(|p| p.final_value).collect();

            if !values.is_empty() {
                values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let n = values.len();

                // Median
                let median = if n.is_multiple_of(2) {
                    (values[n / 2 - 1] + values[n / 2]) / 2.0
                } else {
                    values[n / 2]
                };

                // Percentiles
                let p25_idx = (n as f64 * 0.25).floor() as usize;
                let p75_idx = (n as f64 * 0.75).floor() as usize;
                let p25 = values[p25_idx.min(n - 1)];
                let p75 = values[p75_idx.min(n - 1)];

                // Min/Max
                let min = values[0];
                let max = values[n - 1];

                estimate = estimate
                    .with_median(median)
                    .with_percentiles(p25, p75)
                    .with_range(min, max);
            }

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
        // Reset payoff for new path
        payoff.reset();

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

            path_state.step = step + 1;
            path_state.time = t + dt;
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
        // Reset payoff for new path
        payoff.reset();

        // Initialize state
        state.copy_from_slice(initial_state);

        // Initialize simulated path.
        //
        // Use with_capacity for the points Vec to avoid reallocations as we push,
        // but keep per-path allocation local to this function to preserve
        // thread-safety and deterministic ordering under parallel execution.
        let num_steps = self.config.time_grid.num_steps() + 1; // +1 for initial point
        let mut simulated_path = SimulatedPath::with_capacity(path_id, num_steps);

        // Capture initial point
        let initial_state_vec = SmallVec::from_slice(state);
        let mut initial_point = PathPoint::with_state(0, 0.0, initial_state_vec);
        if self.config.path_capture.capture_payoffs {
            // Initial payoff is typically zero, but capture it for completeness
            initial_point.set_payoff(0.0);
        }
        simulated_path.add_point(initial_point);

        // Create initial path state for payoff
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

            path_state.step = step + 1;
            path_state.time = t + dt;
            process.populate_path_state(state, &mut path_state);
            path_state.set_uniform_random(rng.next_u01());

            // Process payoff event (payoff may add cashflows to path_state)
            payoff.on_event(&mut path_state);

            // Capture this point with state vector
            let state_vec = SmallVec::from_slice(state);
            let mut point = PathPoint::with_state(step + 1, t + dt, state_vec);

            // Transfer cashflows from PathState to PathPoint
            let cashflows = path_state.take_cashflows();
            for (time, amount, cf_type) in cashflows {
                point.add_typed_cashflow(time, amount, cf_type);
            }

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
        let all_cashflows = simulated_path.extract_cashflows();
        if all_cashflows.len() >= 2 {
            // Use periodic IRR approximation (assumes roughly equal spacing)
            let cashflow_amounts: Vec<f64> = all_cashflows.iter().map(|(_, amt)| *amt).collect();

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
        currency: Currency,
    ) -> Result<f64>
    where
        R: RandomStream,
        P: StochasticProcess,
        D: Discretization<P>,
        F: Payoff,
    {
        // Primary path state and payoff
        let mut state_p = vec![0.0; process.dim()];
        state_p.copy_from_slice(initial_state);
        let mut payoff_p = payoff.clone();
        payoff_p.reset();
        let mut path_state_p = PathState::new(0, 0.0);
        process.populate_path_state(&state_p, &mut path_state_p);
        let u_init = rng.next_u01();
        path_state_p.set_uniform_random(u_init);
        payoff_p.on_event(&mut path_state_p);

        // Antithetic path state and payoff
        let mut state_a = vec![0.0; process.dim()];
        state_a.copy_from_slice(initial_state);
        let mut payoff_a = payoff.clone();
        payoff_a.reset();
        let mut path_state_a = PathState::new(0, 0.0);
        process.populate_path_state(&state_a, &mut path_state_a);
        // Use 1-u for antithetic path to preserve negative correlation in barrier sampling
        path_state_a.set_uniform_random(1.0 - u_init);
        payoff_a.on_event(&mut path_state_a);

        // Shared buffers
        let mut z = vec![0.0; process.num_factors()];
        let mut z_anti = vec![0.0; process.num_factors()];
        let mut work = vec![0.0; disc.work_size(process)];

        for step in 0..self.config.time_grid.num_steps() {
            let t = self.config.time_grid.time(step);
            let dt = self.config.time_grid.dt(step);

            rng.fill_std_normals(&mut z);
            for i in 0..z.len() {
                z_anti[i] = -z[i];
            }

            disc.step(process, t, dt, &mut state_p, &z, &mut work);
            disc.step(process, t, dt, &mut state_a, &z_anti, &mut work);

            let u_step = rng.next_u01();

            path_state_p.step = step + 1;
            path_state_p.time = t + dt;
            process.populate_path_state(&state_p, &mut path_state_p);
            path_state_p.set_uniform_random(u_step);
            payoff_p.on_event(&mut path_state_p);

            path_state_a.step = step + 1;
            path_state_a.time = t + dt;
            process.populate_path_state(&state_a, &mut path_state_a);
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
}
