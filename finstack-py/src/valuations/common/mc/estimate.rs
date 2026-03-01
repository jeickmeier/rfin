//! Python bindings for Monte Carlo estimation types.

use finstack_valuations::instruments::common::mc::estimate::{ConvergenceDiagnostics, Estimate};
use pyo3::prelude::*;

/// Monte Carlo estimation result.
///
/// Contains point estimate, uncertainty quantification, and metadata
/// about the simulation run.
#[pyclass(
    module = "finstack.valuations.common.mc",
    name = "Estimate",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyEstimate {
    pub(crate) inner: Estimate,
}

#[pymethods]
impl PyEstimate {
    /// Create a new estimate.
    ///
    /// Args:
    ///     mean: Point estimate (mean)
    ///     stderr: Standard error of the mean
    ///     ci_95_lower: Lower bound of 95% confidence interval
    ///     ci_95_upper: Upper bound of 95% confidence interval
    ///     num_paths: Number of paths simulated
    #[new]
    fn new(mean: f64, stderr: f64, ci_95_lower: f64, ci_95_upper: f64, num_paths: usize) -> Self {
        Self {
            inner: Estimate::new(mean, stderr, (ci_95_lower, ci_95_upper), num_paths),
        }
    }

    /// Point estimate (mean).
    #[getter]
    fn mean(&self) -> f64 {
        self.inner.mean
    }

    /// Standard error of the mean.
    #[getter]
    fn stderr(&self) -> f64 {
        self.inner.stderr
    }

    /// 95% confidence interval as (lower, upper).
    #[getter]
    fn ci_95(&self) -> (f64, f64) {
        self.inner.ci_95
    }

    /// Number of paths simulated.
    #[getter]
    fn num_paths(&self) -> usize {
        self.inner.num_paths
    }

    /// Sample standard deviation (if available).
    #[getter]
    fn std_dev(&self) -> Option<f64> {
        self.inner.std_dev
    }

    /// Median value (if available).
    #[getter]
    fn median(&self) -> Option<f64> {
        self.inner.median
    }

    /// 25th percentile (if available).
    #[getter]
    fn percentile_25(&self) -> Option<f64> {
        self.inner.percentile_25
    }

    /// 75th percentile (if available).
    #[getter]
    fn percentile_75(&self) -> Option<f64> {
        self.inner.percentile_75
    }

    /// Minimum value (if available).
    #[getter]
    fn min(&self) -> Option<f64> {
        self.inner.min
    }

    /// Maximum value (if available).
    #[getter]
    fn max(&self) -> Option<f64> {
        self.inner.max
    }

    /// Relative standard error (stderr / |mean|).
    fn relative_stderr(&self) -> f64 {
        self.inner.relative_stderr()
    }

    /// Coefficient of variation (std_dev / |mean|).
    fn cv(&self) -> Option<f64> {
        self.inner.cv()
    }

    /// Half-width of the 95% confidence interval.
    fn ci_half_width(&self) -> f64 {
        self.inner.ci_half_width()
    }

    /// Interquartile range (IQR) if percentiles are available.
    fn iqr(&self) -> Option<f64> {
        self.inner.iqr()
    }

    /// Range (max - min) if available.
    fn range(&self) -> Option<f64> {
        self.inner.range()
    }

    fn __repr__(&self) -> String {
        format!(
            "Estimate(mean={:.6}, stderr={:.6}, ci_95=({:.6}, {:.6}), n={})",
            self.inner.mean,
            self.inner.stderr,
            self.inner.ci_95.0,
            self.inner.ci_95.1,
            self.inner.num_paths
        )
    }
}

impl PyEstimate {
    pub fn from_inner(inner: Estimate) -> Self {
        Self { inner }
    }
}

/// Convergence diagnostics for Monte Carlo simulation.
#[pyclass(
    module = "finstack.valuations.common.mc",
    name = "ConvergenceDiagnostics",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyConvergenceDiagnostics {
    pub(crate) inner: ConvergenceDiagnostics,
}

#[pymethods]
impl PyConvergenceDiagnostics {
    /// Create empty diagnostics.
    #[new]
    fn new() -> Self {
        Self {
            inner: ConvergenceDiagnostics::new(),
        }
    }

    /// Stderr decay rate (should be ~-0.5 for standard MC).
    #[getter]
    fn stderr_decay_rate(&self) -> Option<f64> {
        self.inner.stderr_decay_rate
    }

    /// Effective sample size (for weighted samples).
    #[getter]
    fn effective_sample_size(&self) -> Option<usize> {
        self.inner.effective_sample_size
    }

    /// Variance reduction factor (vs. baseline).
    #[getter]
    fn variance_reduction_factor(&self) -> Option<f64> {
        self.inner.variance_reduction_factor
    }

    fn __repr__(&self) -> String {
        format!(
            "ConvergenceDiagnostics(stderr_decay={:?}, ess={:?}, vr_factor={:?})",
            self.inner.stderr_decay_rate,
            self.inner.effective_sample_size,
            self.inner.variance_reduction_factor
        )
    }
}

impl PyConvergenceDiagnostics {
    pub fn from_inner(inner: ConvergenceDiagnostics) -> Self {
        Self { inner }
    }
}
