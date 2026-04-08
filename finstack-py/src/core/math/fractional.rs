//! Python bindings for fractional Brownian motion primitives.
//!
//! Provides `HurstExponent` and free functions for fBM covariance/variance
//! computations. The Mittag-Leffler function is omitted because it uses
//! `Complex64` which requires non-trivial wrapping.

use crate::errors::core_to_py;
use finstack_core::math::fractional::{self, HurstExponent};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

// ---------------------------------------------------------------------------
// PyHurstExponent
// ---------------------------------------------------------------------------

/// Validated Hurst exponent H in (0, 1).
///
/// The Hurst exponent determines the roughness of fractional Brownian motion:
///
/// - H < 0.5: rough (anti-persistent increments)
/// - H = 0.5: standard Brownian motion
/// - H > 0.5: smooth (persistent increments)
///
/// Parameters
/// ----------
/// h : float
///     The Hurst parameter value, must be in the open interval (0, 1).
///
/// Raises
/// ------
/// ValueError
///     If ``h`` is not in (0, 1) or is not finite.
#[pyclass(
    module = "finstack.core.math.fractional",
    name = "HurstExponent",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyHurstExponent {
    pub(crate) inner: HurstExponent,
}

#[pymethods]
impl PyHurstExponent {
    /// Create a new Hurst exponent, validating that H is in (0, 1) and is finite.
    ///
    /// Parameters
    /// ----------
    /// h : float
    ///     The Hurst parameter value.
    #[new]
    #[pyo3(text_signature = "(h)")]
    fn new(h: f64) -> PyResult<Self> {
        let inner = HurstExponent::new(h).map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// The raw Hurst parameter value.
    #[getter]
    fn value(&self) -> f64 {
        self.inner.value()
    }

    /// The fractional index alpha = H + 0.5 used in Volterra-type representations.
    #[getter]
    fn alpha(&self) -> f64 {
        self.inner.alpha()
    }

    /// Whether the exponent describes a rough process (H < 0.5).
    #[getter]
    fn is_rough(&self) -> bool {
        self.inner.is_rough()
    }

    fn __repr__(&self) -> String {
        format!("HurstExponent(h={})", self.inner.value())
    }

    fn __float__(&self) -> f64 {
        self.inner.value()
    }
}

// ---------------------------------------------------------------------------
// Free functions
// ---------------------------------------------------------------------------

/// Covariance of fractional Brownian motion.
///
/// Cov(B_H(t), B_H(s)) = 0.5 * (|t|^{2H} + |s|^{2H} - |t-s|^{2H})
///
/// Parameters
/// ----------
/// t : float
///     First time point.
/// s : float
///     Second time point.
/// h : float
///     Hurst exponent.
///
/// Returns
/// -------
/// float
#[pyfunction(name = "fbm_covariance")]
#[pyo3(text_signature = "(t, s, h)")]
fn fbm_covariance_py(t: f64, s: f64, h: f64) -> f64 {
    fractional::fbm_covariance(t, s, h)
}

/// Variance of fractional Brownian motion at time t.
///
/// Var(B_H(t)) = |t|^{2H}
///
/// Parameters
/// ----------
/// t : float
///     Time point.
/// h : float
///     Hurst exponent.
///
/// Returns
/// -------
/// float
#[pyfunction(name = "fbm_variance")]
#[pyo3(text_signature = "(t, h)")]
fn fbm_variance_py(t: f64, h: f64) -> f64 {
    fractional::fbm_variance(t, h)
}

/// Covariance of fBM increments on arbitrary intervals.
///
/// Cov(B_H(ti1) - B_H(ti), B_H(tj1) - B_H(tj))
///
/// Parameters
/// ----------
/// ti : float
///     Start of the first interval.
/// ti1 : float
///     End of the first interval.
/// tj : float
///     Start of the second interval.
/// tj1 : float
///     End of the second interval.
/// h : float
///     Hurst exponent.
///
/// Returns
/// -------
/// float
#[pyfunction(name = "fbm_increment_covariance")]
#[pyo3(text_signature = "(ti, ti1, tj, tj1, h)")]
fn fbm_increment_covariance_py(ti: f64, ti1: f64, tj: f64, tj1: f64, h: f64) -> f64 {
    fractional::fbm_increment_covariance(ti, ti1, tj, tj1, h)
}

/// Full n x n covariance matrix of fBM at times t_1, ..., t_n.
///
/// Entry (i, j) = Cov(B_H(t_i), B_H(t_j)).
///
/// Parameters
/// ----------
/// times : list[float]
///     Time points.
/// h : float
///     Hurst exponent.
///
/// Returns
/// -------
/// list[list[float]]
///     Symmetric covariance matrix.
#[pyfunction(name = "fbm_covariance_matrix")]
#[pyo3(text_signature = "(times, h)")]
fn fbm_covariance_matrix_py(times: Vec<f64>, h: f64) -> Vec<Vec<f64>> {
    let matrix = fractional::fbm_covariance_matrix(&times, h);
    let nrows = matrix.nrows();
    let ncols = matrix.ncols();
    (0..nrows)
        .map(|i| (0..ncols).map(|j| matrix[(i, j)]).collect())
        .collect()
}

/// Covariance matrix of fBM increments on a time grid.
///
/// Given times t_0, t_1, ..., t_n the matrix is n x n with entry
/// (i, j) = Cov(B_H(t_{i+1}) - B_H(t_i), B_H(t_{j+1}) - B_H(t_j)).
///
/// Requires at least two time points. Returns an empty list when fewer than
/// two points are supplied.
///
/// Parameters
/// ----------
/// times : list[float]
///     Time grid points (at least 2).
/// h : float
///     Hurst exponent.
///
/// Returns
/// -------
/// list[list[float]]
///     Symmetric covariance matrix of increments.
#[pyfunction(name = "fbm_increment_covariance_matrix")]
#[pyo3(text_signature = "(times, h)")]
fn fbm_increment_covariance_matrix_py(times: Vec<f64>, h: f64) -> Vec<Vec<f64>> {
    let matrix = fractional::fbm_increment_covariance_matrix(&times, h);
    let nrows = matrix.nrows();
    let ncols = matrix.ncols();
    (0..nrows)
        .map(|i| (0..ncols).map(|j| matrix[(i, j)]).collect())
        .collect()
}

// NOTE: mittag_leffler is not exposed because it uses Complex64 (num_complex)
// which would require non-trivial wrapping (e.g., accepting/returning tuples
// of (re, im) or a dedicated PyComplex wrapper).

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "fractional")?;
    module.setattr(
        "__doc__",
        concat!(
            "Fractional Brownian motion primitives and kernel functions.\n\n",
            "Provides HurstExponent and covariance functions for fBM-based\n",
            "rough volatility models."
        ),
    )?;
    module.add_class::<PyHurstExponent>()?;
    module.add_function(wrap_pyfunction!(fbm_covariance_py, &module)?)?;
    module.add_function(wrap_pyfunction!(fbm_variance_py, &module)?)?;
    module.add_function(wrap_pyfunction!(fbm_increment_covariance_py, &module)?)?;
    module.add_function(wrap_pyfunction!(fbm_covariance_matrix_py, &module)?)?;
    module.add_function(wrap_pyfunction!(
        fbm_increment_covariance_matrix_py,
        &module
    )?)?;

    let exports = [
        "HurstExponent",
        "fbm_covariance",
        "fbm_variance",
        "fbm_increment_covariance",
        "fbm_covariance_matrix",
        "fbm_increment_covariance_matrix",
    ];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    let _ = py;
    Ok(exports.to_vec())
}
