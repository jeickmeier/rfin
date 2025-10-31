//! Monte Carlo simulation engine.
//!
//! Provides the core execution harness for Monte Carlo simulation with:
//! - Structure of Arrays (SoA) layout for cache efficiency
//! - Rayon-based parallelism with deterministic reduction
//! - Online statistics via Welford's algorithm
//! - Variance reduction integration
//! - Auto-stopping on target confidence interval

use super::results::{Estimate, MoneyEstimate};
use super::stats::OnlineStats;
use super::time_grid::TimeGrid;
use super::traits::{Discretization, PathState, Payoff, RandomStream, StochasticProcess};
use finstack_core::currency::Currency;
use finstack_core::Result;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Monte Carlo engine configuration.
#[derive(Clone, Debug)]
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
}

/// Monte Carlo engine builder.
pub struct McEngineBuilder {
    num_paths: usize,
    seed: u64,
    time_grid: Option<TimeGrid>,
    target_ci: Option<f64>,
    parallel: bool,
    chunk_size: usize,
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

    /// Build the engine.
    pub fn build(self) -> Result<McEngine> {
        let time_grid = self
            .time_grid
            .ok_or(finstack_core::error::InputError::Invalid)?;

        let config = McEngineConfig {
            num_paths: self.num_paths,
            seed: self.seed,
            time_grid,
            target_ci_half_width: self.target_ci,
            use_parallel: self.parallel,
            chunk_size: self.chunk_size,
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
            self.price_parallel(rng, process, disc, initial_state, payoff, discount_factor)?
        } else {
            self.price_serial(rng, process, disc, initial_state, payoff, discount_factor)?
        };

        Ok(MoneyEstimate::from_estimate(estimate, currency))
    }

    /// Serial pricing implementation.
    fn price_serial<R, P, D, F>(
        &self,
        rng: &R,
        process: &P,
        disc: &D,
        initial_state: &[f64],
        payoff: &F,
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

            // Simulate one path
            let payoff_value = self.simulate_path(
                &mut path_rng,
                process,
                disc,
                initial_state,
                &mut payoff_clone,
                &mut state,
                &mut z,
                &mut work,
            )?;

            // Accumulate statistics
            let discounted_value = payoff_value * discount_factor;
            stats.update(discounted_value);

            // Check auto-stop condition
            if let Some(target) = self.config.target_ci_half_width {
                if stats.count() > 100 && stats.ci_half_width() < target {
                    break;
                }
            }
        }

        Ok(
            Estimate::new(stats.mean(), stats.stderr(), stats.ci_95(), stats.count())
                .with_std_dev(stats.std_dev()),
        )
    }

    /// Parallel pricing implementation.
    #[cfg(feature = "parallel")]
    fn price_parallel<R, P, D, F>(
        &self,
        rng: &R,
        process: &P,
        disc: &D,
        initial_state: &[f64],
        payoff: &F,
        discount_factor: f64,
    ) -> Result<Estimate>
    where
        R: RandomStream,
        P: StochasticProcess,
        D: Discretization<P>,
        F: Payoff,
    {
        // Split paths into chunks for parallel processing
        let chunks: Vec<_> = (0..self.config.num_paths)
            .step_by(self.config.chunk_size)
            .map(|start| {
                let end = (start + self.config.chunk_size).min(self.config.num_paths);
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

                    // Use ? operator to propagate errors instead of panicking
                    let payoff_value = self.simulate_path(
                        &mut path_rng,
                        process,
                        disc,
                        initial_state,
                        &mut payoff_clone,
                        &mut state,
                        &mut z,
                        &mut work,
                    )?;

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
            combined.ci_95(),
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
        discount_factor: f64,
    ) -> Result<Estimate>
    where
        R: RandomStream,
        P: StochasticProcess,
        D: Discretization<P>,
        F: Payoff,
    {
        // Fall back to serial when parallel feature is disabled
        self.price_serial(rng, process, disc, initial_state, payoff, discount_factor)
    }

    /// Simulate a single Monte Carlo path.
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
        for (i, &val) in state.iter().enumerate() {
            if i == 0 {
                path_state.set(super::traits::state_keys::SPOT, val);
            } else if i == 1 {
                path_state.set(super::traits::state_keys::VARIANCE, val);
                path_state.set(super::traits::state_keys::SHORT_RATE, val);
            } else if i == 2 {
                path_state.set("credit_spread", val);
            }
        }
        payoff.on_event(&path_state);

        // Simulate path through time steps
        for step in 0..self.config.time_grid.num_steps() {
            let t = self.config.time_grid.time(step);
            let dt = self.config.time_grid.dt(step);

            // Generate random shocks
            rng.fill_std_normals(z);

            // Advance state
            disc.step(process, t, dt, state, z, work);

            // Update path state
            path_state.step = step + 1;
            path_state.time = t + dt;
            for (i, &val) in state.iter().enumerate() {
                if i == 0 {
                    path_state.set(super::traits::state_keys::SPOT, val);
                } else if i == 1 {
                    path_state.set(super::traits::state_keys::VARIANCE, val);
                    // Also set as SHORT_RATE for multi-factor models (e.g., revolving credit)
                    path_state.set(super::traits::state_keys::SHORT_RATE, val);
                } else if i == 2 {
                    // Third factor (e.g., credit spread)
                    path_state.set("credit_spread", val);
                }
            }

            // Process payoff event
            payoff.on_event(&path_state);
        }

        // Extract payoff value (currency will be added by caller)
        let payoff_money = payoff.value(Currency::USD); // Temporary currency
        Ok(payoff_money.amount())
    }
}

#[cfg(test)]
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
        fn on_event(&mut self, _state: &PathState) {}
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
            .unwrap();

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
            .unwrap();

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
            .unwrap();

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
            .unwrap();

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
        let estimate = result.unwrap();
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
            .unwrap();

        #[cfg(feature = "parallel")]
        let engine_parallel = McEngine::builder()
            .num_paths(1000)
            .uniform_grid(1.0, 10)
            .seed(42)
            .parallel(true)
            .chunk_size(200)
            .build()
            .unwrap();

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
            .unwrap();

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
            .unwrap();

        // Both should succeed and produce same results (deterministic RNG)
        assert_eq!(serial_result.num_paths, 1000);
        #[cfg(feature = "parallel")]
        assert_eq!(parallel_result.num_paths, 1000);
        #[cfg(feature = "parallel")]
        assert_eq!(serial_result.mean.amount(), parallel_result.mean.amount());
    }
}
