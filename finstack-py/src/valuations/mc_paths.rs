//! Python bindings for Monte Carlo path data structures.

use finstack_valuations::instruments::common::mc::path_data::{
    PathDataset, PathPoint, SimulatedPath,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};
use pyo3::Bound;
use std::collections::HashMap;

/// A single point along a Monte Carlo path.
#[pyclass(module = "finstack.valuations.mc_paths", name = "PathPoint")]
#[derive(Clone)]
pub struct PyPathPoint {
    pub(crate) inner: PathPoint,
}

#[pymethods]
impl PyPathPoint {
    /// Get the step index.
    #[getter]
    fn step(&self) -> usize {
        self.inner.step
    }

    /// Get the time in years.
    #[getter]
    fn time(&self) -> f64 {
        self.inner.time
    }

    /// Get state variables as a dictionary.
    #[getter]
    fn state_vars(&self, py: Python) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        for (key, value) in &self.inner.state_vars {
            dict.set_item(key, value)?;
        }
        Ok(dict.into())
    }

    /// Get the payoff value at this point (if captured).
    #[getter]
    fn payoff_value(&self) -> Option<f64> {
        self.inner.payoff_value
    }

    /// Get a specific state variable by name.
    fn get_var(&self, key: &str) -> Option<f64> {
        self.inner.get_var(key)
    }

    /// Get the spot price (convenience method).
    fn spot(&self) -> Option<f64> {
        self.inner.spot()
    }

    /// Get the variance (convenience method).
    fn variance(&self) -> Option<f64> {
        self.inner.variance()
    }

    /// Get the short rate (convenience method).
    fn short_rate(&self) -> Option<f64> {
        self.inner.short_rate()
    }

    fn __repr__(&self) -> String {
        format!(
            "PathPoint(step={}, time={:.4}, vars={})",
            self.inner.step,
            self.inner.time,
            self.inner.state_vars.len()
        )
    }
}

/// A complete simulated Monte Carlo path.
#[pyclass(module = "finstack.valuations.mc_paths", name = "SimulatedPath")]
#[derive(Clone)]
pub struct PySimulatedPath {
    pub(crate) inner: SimulatedPath,
}

#[pymethods]
impl PySimulatedPath {
    /// Get the path ID.
    #[getter]
    fn path_id(&self) -> usize {
        self.inner.path_id
    }

    /// Get all points along the path.
    #[getter]
    fn points(&self) -> Vec<PyPathPoint> {
        self.inner
            .points
            .iter()
            .map(|p| PyPathPoint { inner: p.clone() })
            .collect()
    }

    /// Get the final discounted payoff value.
    #[getter]
    fn final_value(&self) -> f64 {
        self.inner.final_value
    }

    /// Get the number of time steps.
    fn num_steps(&self) -> usize {
        self.inner.num_steps()
    }

    /// Get a specific point by step index.
    fn point(&self, step: usize) -> Option<PyPathPoint> {
        self.inner.point(step).map(|p| PyPathPoint {
            inner: p.clone(),
        })
    }

    /// Get the initial point.
    fn initial_point(&self) -> Option<PyPathPoint> {
        self.inner.initial_point().map(|p| PyPathPoint {
            inner: p.clone(),
        })
    }

    /// Get the terminal point.
    fn terminal_point(&self) -> Option<PyPathPoint> {
        self.inner.terminal_point().map(|p| PyPathPoint {
            inner: p.clone(),
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "SimulatedPath(id={}, steps={}, final_value={:.4})",
            self.inner.path_id,
            self.inner.num_steps(),
            self.inner.final_value
        )
    }

    fn __len__(&self) -> usize {
        self.inner.num_steps()
    }
}

/// Collection of simulated paths with metadata.
#[pyclass(module = "finstack.valuations.mc_paths", name = "PathDataset")]
#[derive(Clone)]
pub struct PyPathDataset {
    pub(crate) inner: PathDataset,
}

#[pymethods]
impl PyPathDataset {
    /// Get all captured paths.
    #[getter]
    fn paths(&self) -> Vec<PySimulatedPath> {
        self.inner
            .paths
            .iter()
            .map(|p| PySimulatedPath {
                inner: p.clone(),
            })
            .collect()
    }

    /// Get the total number of paths in the simulation.
    #[getter]
    fn num_paths_total(&self) -> usize {
        self.inner.num_paths_total
    }

    /// Get the sampling method used.
    #[getter]
    fn sampling_method(&self) -> String {
        self.inner.sampling_method.to_string()
    }

    /// Get the number of captured paths.
    fn num_captured(&self) -> usize {
        self.inner.num_captured()
    }

    /// Get a specific path by index.
    fn path(&self, index: usize) -> Option<PySimulatedPath> {
        self.inner.path(index).map(|p| PySimulatedPath {
            inner: p.clone(),
        })
    }

    /// Check if all paths were captured.
    fn is_complete(&self) -> bool {
        self.inner.is_complete()
    }

    /// Get the sampling ratio (captured / total).
    fn sampling_ratio(&self) -> f64 {
        self.inner.sampling_ratio()
    }

    /// Get all state variable keys present in the dataset.
    fn state_var_keys(&self) -> Vec<String> {
        self.inner.state_var_keys()
    }

    /// Convert to a long-format dictionary suitable for pandas DataFrame.
    ///
    /// Returns a dictionary with columns:
    /// - path_id: Path identifier
    /// - step: Time step index
    /// - time: Time in years
    /// - final_value: Final discounted payoff for this path
    /// - One column per state variable (e.g., 'spot', 'variance')
    /// - payoff_value: Optional payoff at each step (if captured)
    fn to_dict(&self, py: Python) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);

        // Pre-allocate vectors
        let total_points: usize = self.inner.paths.iter().map(|p| p.points.len()).sum();

        let mut path_ids = Vec::with_capacity(total_points);
        let mut steps = Vec::with_capacity(total_points);
        let mut times = Vec::with_capacity(total_points);
        let mut final_values = Vec::with_capacity(total_points);
        let mut payoff_values = Vec::with_capacity(total_points);

        // Collect all state variable keys
        let state_keys = self.inner.state_var_keys();
        let mut state_columns: HashMap<String, Vec<Option<f64>>> = state_keys
            .iter()
            .map(|k| (k.clone(), Vec::with_capacity(total_points)))
            .collect();

        // Iterate through all paths and points
        for path in &self.inner.paths {
            for point in &path.points {
                path_ids.push(path.path_id);
                steps.push(point.step);
                times.push(point.time);
                final_values.push(path.final_value);
                payoff_values.push(point.payoff_value);

                // Add state variables
                for key in &state_keys {
                    if let Some(col) = state_columns.get_mut(key) {
                        col.push(point.state_vars.get(key.as_str()).copied());
                    }
                }
            }
        }

        // Add to dictionary
        dict.set_item("path_id", path_ids)?;
        dict.set_item("step", steps)?;
        dict.set_item("time", times)?;
        dict.set_item("final_value", final_values)?;
        dict.set_item("payoff_value", payoff_values)?;

        for (key, values) in state_columns {
            dict.set_item(key, values)?;
        }

        Ok(dict.into())
    }

    /// Convert to a wide-format dictionary (paths as columns).
    ///
    /// Returns a dictionary with:
    /// - time: Time points (shared across all paths)
    /// - step: Step indices
    /// - path_0, path_1, ...: State variable values for each path
    ///
    /// Args:
    ///     state_var: Name of the state variable to extract (e.g., 'spot')
    fn to_wide_dict(&self, py: Python, state_var: &str) -> PyResult<Py<PyDict>> {
        if self.inner.paths.is_empty() {
            return Err(PyValueError::new_err("No paths in dataset"));
        }

        let dict = PyDict::new(py);

        // Use first path to get time points
        let first_path = &self.inner.paths[0];
        let times: Vec<f64> = first_path.points.iter().map(|p| p.time).collect();
        let steps: Vec<usize> = first_path.points.iter().map(|p| p.step).collect();

        dict.set_item("time", times)?;
        dict.set_item("step", steps)?;

        // Add each path as a column
        for (idx, path) in self.inner.paths.iter().enumerate() {
            let values: Vec<Option<f64>> = path
                .points
                .iter()
                .map(|p| p.state_vars.get(state_var).copied())
                .collect();
            dict.set_item(format!("path_{}", idx), values)?;
        }

        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "PathDataset(captured={}, total={}, sampling={})",
            self.inner.num_captured(),
            self.inner.num_paths_total,
            self.inner.sampling_method
        )
    }

    fn __len__(&self) -> usize {
        self.inner.num_captured()
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "mc_paths")?;
    module.setattr(
        "__doc__",
        "Monte Carlo path data structures for visualization and analysis.",
    )?;

    module.add_class::<PyPathPoint>()?;
    module.add_class::<PySimulatedPath>()?;
    module.add_class::<PyPathDataset>()?;

    let exports = vec!["PathPoint", "SimulatedPath", "PathDataset"];

    module.setattr("__all__", PyList::new(py, &exports)?)?;

    parent.add_submodule(&module)?;
    parent.setattr("mc_paths", &module)?;

    Ok(exports)
}

