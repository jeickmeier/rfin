//! Python bindings for Monte Carlo time grids.

use crate::errors::map_error;
use finstack_core::math::time_grid::TimeGrid;
use pyo3::exceptions::PyIndexError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

/// Time grid for Monte Carlo simulation.
#[pyclass(
    name = "TimeGrid",
    module = "finstack.core.math",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyTimeGrid {
    pub(crate) inner: TimeGrid,
}

impl PyTimeGrid {
    pub(crate) fn from_inner(inner: TimeGrid) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyTimeGrid {
    /// Create a uniform time grid.
    ///
    /// Args:
    ///     t_max: Maximum time (year fraction).
    ///     num_steps: Number of time steps.
    #[staticmethod]
    fn uniform(t_max: f64, num_steps: usize) -> PyResult<Self> {
        TimeGrid::uniform(t_max, num_steps)
            .map(Self::from_inner)
            .map_err(map_error)
    }

    /// Create a uniform time grid with required intermediate times.
    ///
    /// Args:
    ///     t_max: Horizon in years (> 0).
    ///     steps_per_year: Target density for uniform spacing (> 0).
    ///     min_steps: Minimum number of uniform steps before merging events.
    ///     required_times: Extra knot times (e.g. barrier monitoring, cashflow dates).
    #[staticmethod]
    fn uniform_with_required_times(
        t_max: f64,
        steps_per_year: f64,
        min_steps: usize,
        required_times: Vec<f64>,
    ) -> PyResult<Self> {
        TimeGrid::uniform_with_required_times(t_max, steps_per_year, min_steps, &required_times)
            .map(Self::from_inner)
            .map_err(map_error)
    }

    /// Create a time grid from explicit time points.
    ///
    /// Args:
    ///     times: Monotonically increasing time points starting at 0.
    #[staticmethod]
    fn from_times(times: Vec<f64>) -> PyResult<Self> {
        TimeGrid::from_times(times)
            .map(Self::from_inner)
            .map_err(map_error)
    }

    /// Number of time steps in the grid.
    #[getter]
    fn num_steps(&self) -> usize {
        self.inner.num_steps()
    }

    /// Maximum time in the grid.
    #[getter]
    fn t_max(&self) -> f64 {
        self.inner.t_max()
    }

    /// Get the time at a given step index.
    fn time(&self, step: usize) -> PyResult<f64> {
        if step > self.inner.num_steps() {
            return Err(PyIndexError::new_err(format!(
                "step {} out of range [0, {}]",
                step,
                self.inner.num_steps()
            )));
        }
        Ok(self.inner.time(step))
    }

    /// Get the time increment at a given step index.
    fn dt(&self, step: usize) -> PyResult<f64> {
        if step >= self.inner.num_steps() {
            return Err(PyIndexError::new_err(format!(
                "step {} out of range [0, {})",
                step,
                self.inner.num_steps()
            )));
        }
        Ok(self.inner.dt(step))
    }

    /// All time points as a list.
    #[getter]
    fn times(&self) -> Vec<f64> {
        self.inner.times().to_vec()
    }

    /// All time increments as a list.
    #[getter]
    fn dts(&self) -> Vec<f64> {
        self.inner.dts().to_vec()
    }

    /// Whether the grid has uniform spacing.
    #[getter]
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
        self.inner.num_steps() + 1
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "time_grid")?;
    module.setattr(
        "__doc__",
        "Time grid utilities for Monte Carlo simulation.\n\n\
         Classes:\n\
         - TimeGrid: Discretization points for MC simulation",
    )?;

    module.add_class::<PyTimeGrid>()?;

    let exports = ["TimeGrid"];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    let _ = py;
    Ok(exports.to_vec())
}
