//! Result types for Monte Carlo simulations.

use crate::bindings::core::money::PyMoney;
use finstack_monte_carlo::results::MoneyEstimate;
use pyo3::prelude::*;

/// Monte Carlo pricing result with discounted statistics.
#[pyclass(name = "MonteCarloResult", module = "finstack.monte_carlo", frozen)]
pub struct PyMonteCarloResult {
    inner: MoneyEstimate,
}

impl PyMonteCarloResult {
    pub(super) fn from_inner(inner: MoneyEstimate) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMonteCarloResult {
    /// Discounted mean present value.
    #[getter]
    fn mean(&self) -> PyMoney {
        PyMoney::from_inner(self.inner.mean)
    }

    /// Standard error of the discounted mean.
    #[getter]
    fn stderr(&self) -> f64 {
        self.inner.stderr
    }

    /// Sample standard deviation (if available).
    #[getter]
    fn std_dev(&self) -> Option<f64> {
        self.inner.std_dev
    }

    /// Lower bound of the 95% CI.
    #[getter]
    fn ci_lower(&self) -> PyMoney {
        PyMoney::from_inner(self.inner.ci_95.0)
    }

    /// Upper bound of the 95% CI.
    #[getter]
    fn ci_upper(&self) -> PyMoney {
        PyMoney::from_inner(self.inner.ci_95.1)
    }

    /// Number of simulated paths.
    #[getter]
    fn num_paths(&self) -> usize {
        self.inner.num_paths
    }

    /// Relative standard error (stderr / |mean|).
    fn relative_stderr(&self) -> f64 {
        self.inner.relative_stderr()
    }

    fn __repr__(&self) -> String {
        format!(
            "MonteCarloResult(mean={}, stderr={:.6}, n={})",
            self.inner.mean, self.inner.stderr, self.inner.num_paths,
        )
    }
}

/// Raw numerical estimate (non-currency).
#[pyclass(name = "Estimate", module = "finstack.monte_carlo", frozen)]
pub struct PyEstimate {
    inner: finstack_monte_carlo::estimate::Estimate,
}

impl PyEstimate {
    #[allow(dead_code)]
    pub(super) fn from_inner(inner: finstack_monte_carlo::estimate::Estimate) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyEstimate {
    /// Point estimate (mean).
    #[getter]
    fn mean(&self) -> f64 {
        self.inner.mean
    }

    /// Standard error.
    #[getter]
    fn stderr(&self) -> f64 {
        self.inner.stderr
    }

    /// Sample standard deviation (if available).
    #[getter]
    fn std_dev(&self) -> Option<f64> {
        self.inner.std_dev
    }

    /// Lower 95% CI bound.
    #[getter]
    fn ci_lower(&self) -> f64 {
        self.inner.ci_95.0
    }

    /// Upper 95% CI bound.
    #[getter]
    fn ci_upper(&self) -> f64 {
        self.inner.ci_95.1
    }

    /// Number of paths/samples.
    #[getter]
    fn num_paths(&self) -> usize {
        self.inner.num_paths
    }

    fn __repr__(&self) -> String {
        format!(
            "Estimate(mean={:.6}, stderr={:.6}, n={})",
            self.inner.mean, self.inner.stderr, self.inner.num_paths,
        )
    }
}

pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyMonteCarloResult>()?;
    m.add_class::<PyEstimate>()?;
    Ok(())
}
