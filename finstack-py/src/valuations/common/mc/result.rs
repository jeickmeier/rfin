//! Python bindings for Monte Carlo result wrapper.

use super::paths::PyPathDataset;
use crate::core::money::PyMoney;
use finstack_core::math::special_functions::standard_normal_inv_cdf;
use finstack_core::money::Money;
use finstack_valuations::instruments::common::models::monte_carlo::results::MonteCarloResult;
use pyo3::prelude::*;

/// Monte Carlo result with optional path data.
#[pyclass(module = "finstack.valuations.common.mc", name = "MonteCarloResult", from_py_object)]
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

    /// Get the confidence interval for the given significance level.
    fn confidence_interval(&self, alpha: f64) -> (PyMoney, PyMoney) {
        let z = standard_normal_inv_cdf(1.0 - alpha / 2.0);
        let margin = z * self.inner.estimate.stderr;
        let mean = self.inner.estimate.mean.amount();
        let currency = self.inner.estimate.mean.currency();
        (
            PyMoney::new(Money::new(mean - margin, currency)),
            PyMoney::new(Money::new(mean + margin, currency)),
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
