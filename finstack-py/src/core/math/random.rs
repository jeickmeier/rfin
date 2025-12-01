use finstack_core::math::random::{
    box_muller_transform as core_box_muller_transform, RandomNumberGenerator, TestRng,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

#[pyclass(name = "SimpleRng", module = "finstack.core.math.random")]
/// Deterministic pseudo‑random number generator for testing and simple simulations.
///
/// This wraps `finstack-core`'s `SimpleRng`, exposing:
/// - `uniform()`: U(0, 1) variates
/// - `normal(mean, std_dev)`: Normal variates
/// - `bernoulli(p)`: Bernoulli trials
#[derive(Clone, Debug)]
pub struct PySimpleRng {
    inner: TestRng,
}

#[pymethods]
impl PySimpleRng {
    #[new]
    #[pyo3(text_signature = "(seed)")]
    /// Create a new RNG with the given integer seed.
    ///
    /// Parameters
    /// ----------
    /// seed : int
    ///     Seed for the underlying generator. The same seed yields the same sequence.
    pub fn new(seed: u64) -> Self {
        Self {
            inner: TestRng::new(seed),
        }
    }

    #[pyo3(text_signature = "($self)")]
    /// Draw a uniform random number in ``[0, 1)``.
    ///
    /// Returns
    /// -------
    /// float
    ///     Uniform variate in ``[0, 1)``.
    pub fn uniform(&mut self) -> f64 {
        self.inner.uniform()
    }

    #[pyo3(text_signature = "($self, mean=0.0, std_dev=1.0)")]
    /// Draw a normally distributed random number.
    ///
    /// Parameters
    /// ----------
    /// mean : float, optional
    ///     Mean of the distribution (default ``0.0``).
    /// std_dev : float, optional
    ///     Standard deviation (must be positive, default ``1.0``).
    ///
    /// Returns
    /// -------
    /// float
    ///     Normal variate with the requested parameters.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``std_dev`` is not positive.
    pub fn normal(&mut self, mean: Option<f64>, std_dev: Option<f64>) -> PyResult<f64> {
        let m = mean.unwrap_or(0.0);
        let s = std_dev.unwrap_or(1.0);
        if s <= 0.0 {
            return Err(PyValueError::new_err("std_dev must be positive"));
        }
        Ok(self.inner.normal(m, s))
    }

    #[pyo3(text_signature = "($self, p)")]
    /// Draw a Bernoulli trial with success probability ``p``.
    ///
    /// Parameters
    /// ----------
    /// p : float
    ///     Probability of success in ``[0, 1]``.
    ///
    /// Returns
    /// -------
    /// bool
    ///     ``True`` with probability ``p``.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``p`` is outside ``[0, 1]``.
    pub fn bernoulli(&mut self, p: f64) -> PyResult<bool> {
        if !(0.0..=1.0).contains(&p) {
            return Err(PyValueError::new_err("p must be in the range [0, 1]"));
        }
        Ok(self.inner.bernoulli(p))
    }

    /// String representation summarising the RNG type.
    pub fn __repr__(&self) -> String {
        "SimpleRng(seed=<internal>)".to_string()
    }
}

#[pyfunction(name = "box_muller_transform")]
#[pyo3(text_signature = "(u1, u2)")]
/// Box‑Muller transform for generating a pair of standard normal variables.
///
/// Parameters
/// ----------
/// u1 : float
///     First uniform variate in ``(0, 1)`` (extremes are safely clamped).
/// u2 : float
///     Second uniform variate in ``(0, 1)``.
///
/// Returns
/// -------
/// tuple[float, float]
///     Pair ``(z1, z2)`` of independent ``N(0, 1)`` samples.
pub fn box_muller_transform_py(u1: f64, u2: f64) -> (f64, f64) {
    core_box_muller_transform(u1, u2)
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "random")?;
    module.setattr(
        "__doc__",
        "Random number utilities from finstack-core (SimpleRng, Box-Muller transforms).",
    )?;

    module.add_class::<PySimpleRng>()?;
    module.add_function(wrap_pyfunction!(box_muller_transform_py, &module)?)?;

    let exports = ["SimpleRng", "box_muller_transform"];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
