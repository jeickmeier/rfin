//! Python bindings for Monte Carlo time grid.

use crate::errors::core_to_py;
use finstack_monte_carlo::time_grid::TimeGrid;
use pyo3::prelude::*;

/// Time grid for Monte Carlo simulation.
///
/// Defines the discretization points in time from t=0 to t=T.
/// Supports both uniform grids (equal spacing) and custom grids
/// (irregular time points for finer resolution near important dates).
#[pyclass(
    module = "finstack.valuations.common.monte_carlo",
    name = "TimeGrid",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyTimeGrid {
    pub(crate) inner: TimeGrid,
}

#[pymethods]
impl PyTimeGrid {
    /// Create a uniform time grid from 0 to t_max with num_steps steps.
    ///
    /// Args:
    ///     t_max: Final time in years (must be > 0)
    ///     num_steps: Number of time steps (must be > 0)
    ///
    /// Returns:
    ///     TimeGrid with equally spaced time points
    #[staticmethod]
    fn uniform(t_max: f64, num_steps: usize) -> PyResult<Self> {
        let inner = TimeGrid::uniform(t_max, num_steps).map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// Create a custom time grid from explicit time points.
    ///
    /// Args:
    ///     times: Monotonically increasing time points starting at 0.0
    ///
    /// Returns:
    ///     TimeGrid with the specified time points
    #[staticmethod]
    fn from_times(times: Vec<f64>) -> PyResult<Self> {
        let inner = TimeGrid::from_times(times).map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// Number of time steps.
    #[getter]
    fn num_steps(&self) -> usize {
        self.inner.num_steps()
    }

    /// Total time span (t_max).
    #[getter]
    fn t_max(&self) -> f64 {
        self.inner.t_max()
    }

    /// Get all time points as a list.
    #[getter]
    fn times(&self) -> Vec<f64> {
        self.inner.times().to_vec()
    }

    /// Get all time step sizes as a list.
    #[getter]
    fn dts(&self) -> Vec<f64> {
        self.inner.dts().to_vec()
    }

    /// Get time at a specific step index.
    fn time_at(&self, step: usize) -> f64 {
        self.inner.time(step)
    }

    /// Get time step size at a specific step index.
    fn dt_at(&self, step: usize) -> f64 {
        self.inner.dt(step)
    }

    /// Check if grid is uniform (all dts equal within tolerance).
    fn is_uniform(&self) -> bool {
        self.inner.is_uniform()
    }

    fn __repr__(&self) -> String {
        format!(
            "TimeGrid(num_steps={}, t_max={:.4}, uniform={})",
            self.inner.num_steps(),
            self.inner.t_max(),
            self.inner.is_uniform()
        )
    }

    fn __len__(&self) -> usize {
        self.inner.num_steps()
    }
}
