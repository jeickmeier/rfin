//! Python bindings for Monte Carlo process parameters.

use finstack_valuations::instruments::common::mc::path_data::ProcessParams;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};
use pyo3::Bound;

/// Process parameters and metadata for Monte Carlo simulation.
#[pyclass(module = "finstack.valuations.mc_params", name = "ProcessParams")]
#[derive(Clone)]
pub struct PyProcessParams {
    pub(crate) inner: ProcessParams,
}

#[pymethods]
impl PyProcessParams {
    /// Get the process type name.
    #[getter]
    fn process_type(&self) -> String {
        self.inner.process_type.clone()
    }

    /// Get parameters as a dictionary.
    #[getter]
    fn parameters(&self, py: Python) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        for (key, value) in &self.inner.parameters {
            dict.set_item(key, value)?;
        }
        Ok(dict.into())
    }

    /// Get correlation matrix as a flat list (row-major).
    #[getter]
    fn correlation(&self) -> Option<Vec<f64>> {
        self.inner.correlation.clone()
    }

    /// Get factor names.
    #[getter]
    fn factor_names(&self) -> Vec<String> {
        self.inner.factor_names.clone()
    }

    /// Get a specific parameter by name.
    fn get_param(&self, key: &str) -> Option<f64> {
        self.inner.parameters.get(key).copied()
    }

    /// Get the dimension (number of factors) from correlation matrix.
    fn dim(&self) -> Option<usize> {
        self.inner.dim()
    }

    /// Get correlation matrix as a 2D list (nested lists).
    ///
    /// Returns None if no correlation matrix is present.
    fn correlation_matrix(&self, py: Python) -> PyResult<Option<Py<PyList>>> {
        if let Some(ref corr_flat) = self.inner.correlation {
            let dim = (corr_flat.len() as f64).sqrt() as usize;
            let rows = PyList::empty(py);

            for i in 0..dim {
                let row = PyList::empty(py);
                for j in 0..dim {
                    row.append(corr_flat[i * dim + j])?;
                }
                rows.append(row)?;
            }

            Ok(Some(rows.into()))
        } else {
            Ok(None)
        }
    }

    /// Get correlation matrix as a flat numpy-compatible list with shape info.
    ///
    /// Returns a tuple of (flat_data, shape) suitable for numpy.array(data).reshape(shape).
    fn correlation_array(&self) -> Option<(Vec<f64>, (usize, usize))> {
        self.inner.correlation.as_ref().map(|corr| {
            let dim = (corr.len() as f64).sqrt() as usize;
            (corr.clone(), (dim, dim))
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "ProcessParams(type='{}', params={}, factors={})",
            self.inner.process_type,
            self.inner.parameters.len(),
            self.inner.factor_names.len()
        )
    }
}

impl PyProcessParams {
    pub fn new(inner: ProcessParams) -> Self {
        Self { inner }
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "mc_params")?;
    module.setattr("__doc__", "Monte Carlo process parameters and metadata.")?;

    module.add_class::<PyProcessParams>()?;

    let exports = vec!["ProcessParams"];

    module.setattr("__all__", PyList::new(py, &exports)?)?;

    parent.add_submodule(&module)?;
    parent.setattr("mc_params", &module)?;

    Ok(exports)
}
