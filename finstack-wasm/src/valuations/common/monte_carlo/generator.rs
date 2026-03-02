//! WASM bindings for standalone Monte Carlo path generation.

use super::paths::JsPathDataset;
use crate::core::error::js_error;
use finstack_valuations::instruments::common::models::monte_carlo::discretization::exact::ExactGbm;
use finstack_valuations::instruments::common::models::monte_carlo::paths::{
    PathDataset, PathPoint, PathSamplingMethod, SimulatedPath,
};
use finstack_valuations::instruments::common::models::monte_carlo::process::gbm::GbmProcess;
use finstack_valuations::instruments::common::models::monte_carlo::process::metadata::ProcessMetadata;
use finstack_valuations::instruments::common::models::monte_carlo::rng::philox::PhiloxRng;
use finstack_valuations::instruments::common::models::monte_carlo::time_grid::TimeGrid;
use finstack_valuations::instruments::common::models::monte_carlo::{
    Discretization, RandomStream, StochasticProcess,
};
use finstack_valuations::instruments::common::models::monte_carlo::engine::{
    McEngineConfig, PathCaptureConfig, PathCaptureMode,
};
use wasm_bindgen::prelude::*;

/// Standalone Monte Carlo path generator.
///
/// This class generates Monte Carlo paths without pricing, useful for:
/// - Pure process visualization
/// - Process validation
/// - Understanding stochastic dynamics
/// - Educational purposes
#[wasm_bindgen(js_name = MonteCarloPathGenerator)]
#[derive(Default)]
pub struct JsMonteCarloPathGenerator;

#[wasm_bindgen(js_class = MonteCarloPathGenerator)]
impl JsMonteCarloPathGenerator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self
    }

    /// Generate paths for a Geometric Brownian Motion process.
    ///
    /// @param {number} initialSpot - Initial spot price
    /// @param {number} r - Risk-free rate (annual)
    /// @param {number} q - Dividend/foreign rate (annual)
    /// @param {number} sigma - Volatility (annual)
    /// @param {number} timeToMaturity - Time horizon in years
    /// @param {number} numSteps - Number of time steps
    /// @param {number} numPaths - Total number of paths to simulate
    /// @param {string} captureMode - 'all' to capture all paths, or 'sample' with count
    /// @param {number} sampleCount - Number of paths to capture (if mode='sample')
    /// @param {number} seed - Random seed for reproducibility
    ///
    /// @returns {PathDataset} PathDataset with generated paths
    #[allow(clippy::too_many_arguments)]
    #[wasm_bindgen(js_name = generateGbmPaths)]
    pub fn generate_gbm_paths(
        &self,
        initial_spot: f64,
        r: f64,
        q: f64,
        sigma: f64,
        time_to_maturity: f64,
        num_steps: usize,
        num_paths: usize,
        capture_mode: &str,
        sample_count: Option<usize>,
        seed: u64,
    ) -> Result<JsPathDataset, JsValue> {
        // Create GBM process
        let gbm = GbmProcess::with_params(r, q, sigma);

        // Create time grid
        let time_grid =
            TimeGrid::uniform(time_to_maturity, num_steps).map_err(|e| js_error(e.to_string()))?;

        // Configure path capture
        let path_capture = match capture_mode {
            "all" => PathCaptureConfig::all(),
            "sample" => {
                let count = sample_count
                    .ok_or_else(|| js_error("sample_count required when capture_mode='sample'"))?;
                PathCaptureConfig::sample(count, seed + 1)
            }
            _ => {
                return Err(js_error(format!(
                    "Invalid capture_mode '{}', must be 'all' or 'sample'",
                    capture_mode
                )));
            }
        };

        // Create engine config
        let config = McEngineConfig {
            num_paths,
            seed,
            time_grid,
            target_ci_half_width: None,
            use_parallel: false,
            chunk_size: 1000,
            path_capture,
            antithetic: false,
        };

        // Generate paths
        let paths =
            self.generate_paths_internal(&gbm, &ExactGbm::new(), &[initial_spot], &config)?;

        Ok(JsPathDataset::from_inner(paths))
    }
}

impl JsMonteCarloPathGenerator {
    /// Internal path generation method.
    fn generate_paths_internal<P, D>(
        &self,
        process: &P,
        disc: &D,
        initial_state: &[f64],
        config: &McEngineConfig,
    ) -> Result<PathDataset, JsValue>
    where
        P: StochasticProcess + ProcessMetadata,
        D: Discretization<P>,
    {
        let rng = PhiloxRng::new(config.seed);
        let dim = process.dim();
        let num_factors = process.num_factors();
        let work_size = disc.work_size(process);

        // Determine sampling method
        let sampling_method = match &config.path_capture.capture_mode {
            PathCaptureMode::All => PathSamplingMethod::All,
            PathCaptureMode::Sample { count, seed } => PathSamplingMethod::RandomSample {
                count: *count,
                seed: *seed,
            },
        };

        // Get process metadata
        let process_params = process.metadata();

        // Create path dataset
        let mut dataset = PathDataset::new(config.num_paths, sampling_method, process_params);

        // Pre-allocate buffers
        let mut state = vec![0.0; dim];
        let mut z = vec![0.0; num_factors];
        let mut work = vec![0.0; work_size];

        // Generate paths
        for path_id in 0..config.num_paths {
            // Check if we should capture this path
            if !config
                .path_capture
                .should_capture(path_id, config.num_paths)
            {
                continue;
            }

            let mut path_rng = rng.split(path_id as u64);

            // Initialize state
            state.copy_from_slice(initial_state);

            // Create simulated path
            let num_steps = config.time_grid.num_steps() + 1;
            let mut simulated_path = SimulatedPath::with_capacity(path_id, num_steps);

            // Capture initial point
            let initial_point = PathPoint::with_state(0, 0.0, state.clone().into());
            simulated_path.add_point(initial_point);

            // Simulate path
            for step in 0..config.time_grid.num_steps() {
                let t = config.time_grid.time(step);
                let dt = config.time_grid.dt(step);

                // Generate random shocks
                path_rng.fill_std_normals(&mut z);

                // Advance state
                disc.step(process, t, dt, &mut state, &z, &mut work);

                // Capture this point
                let point = PathPoint::with_state(step + 1, t + dt, state.clone().into());
                simulated_path.add_point(point);
            }

            // Final value is just the terminal spot for visualization
            simulated_path.set_final_value(*state.first().unwrap_or(&0.0));

            dataset.add_path(simulated_path);
        }

        Ok(dataset)
    }
}
