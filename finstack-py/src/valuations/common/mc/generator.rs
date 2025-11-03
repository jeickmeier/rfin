//! Python bindings for standalone Monte Carlo path generation.

use crate::core::error::core_to_py;
use super::paths::PyPathDataset;
use finstack_valuations::instruments::common::mc::discretization::exact::ExactGbm;
use finstack_valuations::instruments::common::mc::path_data::{
    PathDataset, PathPoint, PathSamplingMethod, SimulatedPath,
};
use finstack_valuations::instruments::common::mc::process::gbm::GbmProcess;
use finstack_valuations::instruments::common::mc::process::metadata::ProcessMetadata;
use finstack_valuations::instruments::common::mc::rng::philox::PhiloxRng;
use finstack_valuations::instruments::common::mc::time_grid::TimeGrid;
use finstack_valuations::instruments::common::mc::traits::{
    Discretization, RandomStream, StochasticProcess,
};
use finstack_valuations::instruments::common::models::monte_carlo::engine::{
    McEngineConfig, PathCaptureConfig, PathCaptureMode,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::Bound;

/// Standalone Monte Carlo path generator.
///
/// This class generates Monte Carlo paths without pricing, useful for:
/// - Pure process visualization
/// - Process validation
/// - Understanding stochastic dynamics
/// - Educational purposes
#[pyclass(
    module = "finstack.valuations.common.mc",
    name = "MonteCarloPathGenerator"
)]
pub(crate) struct PyMonteCarloPathGenerator;

#[pymethods]
impl PyMonteCarloPathGenerator {
    #[new]
    fn new() -> Self {
        Self
    }

    /// Generate paths for a Geometric Brownian Motion process.
    ///
    /// Args:
    ///     initial_spot: Initial spot price
    ///     r: Risk-free rate (annual)
    ///     q: Dividend/foreign rate (annual)  
    ///     sigma: Volatility (annual)
    ///     time_to_maturity: Time horizon in years
    ///     num_steps: Number of time steps
    ///     num_paths: Total number of paths to simulate
    ///     capture_mode: 'all' to capture all paths, or 'sample' with count
    ///     sample_count: Number of paths to capture (if mode='sample')
    ///     seed: Random seed for reproducibility
    ///
    /// Returns:
    ///     PathDataset with generated paths
    #[pyo3(signature = (initial_spot, r, q, sigma, time_to_maturity, num_steps, num_paths, capture_mode="all", sample_count=None, seed=42))]
    fn generate_gbm_paths(
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
    ) -> PyResult<PyPathDataset> {
        // Create GBM process
        let gbm = GbmProcess::with_params(r, q, sigma);

        // Create time grid
        let time_grid = TimeGrid::uniform(time_to_maturity, num_steps).map_err(core_to_py)?;

        // Configure path capture
        let path_capture = match capture_mode {
            "all" => PathCaptureConfig::all(),
            "sample" => {
                let count = sample_count.ok_or_else(|| {
                    PyValueError::new_err("sample_count required when capture_mode='sample'")
                })?;
                PathCaptureConfig::sample(count, seed + 1)
            }
            _ => {
                return Err(PyValueError::new_err(format!(
                    "Invalid capture_mode '{}', must be 'all' or 'sample'",
                    capture_mode
                )))
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
        };

        // Generate paths (release GIL for compute-heavy path generation)
        let paths = Python::with_gil(|py| {
            py.allow_threads(|| {
                self.generate_paths_internal(&gbm, &ExactGbm::new(), &[initial_spot], &config)
            })
        })?;

        Ok(PyPathDataset { inner: paths })
    }

    /// Generate paths with custom parameters (advanced).
    ///
    /// This is a lower-level interface for advanced users who want full control.
    ///
    /// Args:
    ///     process_type: Currently only 'gbm' supported
    ///     process_params: Dictionary of process parameters
    ///     initial_state: List of initial state values
    ///     time_to_maturity: Time horizon in years
    ///     num_steps: Number of time steps
    ///     num_paths: Total number of paths to simulate
    ///     capture_mode: 'all' or 'sample'
    ///     sample_count: Number of paths to capture (if mode='sample')
    ///     seed: Random seed
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (process_type, process_params, initial_state, time_to_maturity, num_steps, num_paths, capture_mode="all", sample_count=None, seed=42))]
    fn generate_paths(
        &self,
        process_type: &str,
        process_params: &Bound<PyDict>,
        initial_state: Vec<f64>,
        time_to_maturity: f64,
        num_steps: usize,
        num_paths: usize,
        capture_mode: &str,
        sample_count: Option<usize>,
        seed: u64,
    ) -> PyResult<PyPathDataset> {
        // For now, only support GBM
        if process_type != "gbm" {
            return Err(PyValueError::new_err(format!(
                "Unsupported process_type '{}', currently only 'gbm' is supported",
                process_type
            )));
        }

        let r = process_params
            .get_item("r")?
            .ok_or_else(|| PyValueError::new_err("Missing parameter 'r'"))?
            .extract::<f64>()?;
        let q = process_params
            .get_item("q")?
            .ok_or_else(|| PyValueError::new_err("Missing parameter 'q'"))?
            .extract::<f64>()?;
        let sigma = process_params
            .get_item("sigma")?
            .ok_or_else(|| PyValueError::new_err("Missing parameter 'sigma'"))?
            .extract::<f64>()?;

        let initial_spot = *initial_state
            .first()
            .ok_or_else(|| PyValueError::new_err("initial_state must have at least one element"))?;

        self.generate_gbm_paths(
            initial_spot,
            r,
            q,
            sigma,
            time_to_maturity,
            num_steps,
            num_paths,
            capture_mode,
            sample_count,
            seed,
        )
    }
}

impl PyMonteCarloPathGenerator {
    /// Internal path generation method.
    fn generate_paths_internal<P, D>(
        &self,
        process: &P,
        disc: &D,
        initial_state: &[f64],
        config: &McEngineConfig,
    ) -> PyResult<PathDataset>
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
            let mut initial_point = PathPoint::new(0, 0.0);
            for (i, &val) in state.iter().enumerate() {
                let key = match i {
                    0 => "spot",
                    1 => "variance",
                    2 => "credit_spread",
                    _ => continue,
                };
                initial_point.add_var(key.to_string(), val);
            }
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
                let mut point = PathPoint::new(step + 1, t + dt);
                for (i, &val) in state.iter().enumerate() {
                    let key = match i {
                        0 => "spot",
                        1 => "variance",
                        2 => "credit_spread",
                        _ => continue,
                    };
                    point.add_var(key.to_string(), val);
                }
                simulated_path.add_point(point);
            }

            // Final value is just the terminal spot for visualization
            simulated_path.set_final_value(*state.first().unwrap_or(&0.0));

            dataset.add_path(simulated_path);
        }

        Ok(dataset)
    }
}

