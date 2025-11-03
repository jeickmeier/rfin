//! Python bindings for Monte Carlo result wrapper.

use crate::core::money::PyMoney;
use super::paths::PyPathDataset;
use finstack_valuations::instruments::common::models::monte_carlo::results::MonteCarloResult;
use pyo3::prelude::*;

/// Monte Carlo result with optional path data.
#[pyclass(module = "finstack.valuations.common.mc", name = "MonteCarloResult")]
#[derive(Clone)]
pub struct PyMonteCarloResult {
    pub(crate) inner: MonteCarloResult,
}

#[pymethods]
impl PyMonteCarloResult {
    /// Get the statistical estimate.
    #[getter]
    fn estimate(&self) -> PyMoney {
        PyMoney::new(self.inner.estimate.mean)
    }

    /// Get the standard error.
    #[getter]
    fn stderr(&self) -> f64 {
        self.inner.estimate.stderr
    }

    /// Get the 95% confidence interval as a tuple (lower, upper).
    #[getter]
    fn ci_95(&self) -> (PyMoney, PyMoney) {
        (
            PyMoney::new(self.inner.estimate.ci_95.0),
            PyMoney::new(self.inner.estimate.ci_95.1),
        )
    }

    /// Get the number of paths used for the estimate.
    #[getter]
    fn num_paths(&self) -> usize {
        self.inner.estimate.num_paths
    }

    /// Get the captured paths dataset (if available).
    #[getter]
    fn paths(&self) -> Option<PyPathDataset> {
        self.inner
            .paths
            .as_ref()
            .map(|p| PyPathDataset { inner: p.clone() })
    }

    /// Check if paths were captured.
    fn has_paths(&self) -> bool {
        self.inner.has_paths()
    }

    /// Get the number of captured paths.
    fn num_captured_paths(&self) -> usize {
        self.inner.num_captured_paths()
    }

    /// Get just the mean estimate as Money.
    fn mean(&self) -> PyMoney {
        PyMoney::new(self.inner.estimate.mean)
    }

    /// Get the relative standard error (stderr / mean).
    fn relative_stderr(&self) -> f64 {
        self.inner.estimate.relative_stderr()
    }

    fn __repr__(&self) -> String {
        if self.inner.has_paths() {
            format!(
                "MonteCarloResult(estimate={}, stderr={:.4}, paths={}/{})",
                self.inner.estimate.mean,
                self.inner.estimate.stderr,
                self.inner.num_captured_paths(),
                self.inner.estimate.num_paths
            )
        } else {
            format!(
                "MonteCarloResult(estimate={}, stderr={:.4}, num_paths={})",
                self.inner.estimate.mean, self.inner.estimate.stderr, self.inner.estimate.num_paths
            )
        }
    }
}

impl PyMonteCarloResult {
    pub fn new(inner: MonteCarloResult) -> Self {
        Self { inner }
    }
}

