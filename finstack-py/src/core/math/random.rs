use crate::errors::core_to_py;
use finstack_core::math::random::{
    box_muller_transform as core_box_muller_transform,
    brownian_bridge::BrownianBridge,
    poisson::{
        poisson_from_normal as core_poisson_from_normal,
        poisson_inverse_cdf as core_poisson_inverse_cdf,
    },
    sobol::{SobolRng, MAX_SOBOL_DIMENSION},
    Pcg64Rng, RandomNumberGenerator,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

#[pyclass(name = "Rng", module = "finstack.core.math.random", from_py_object)]
/// Production-grade pseudo-random number generator backed by PCG64.
///
/// PCG64 (Permuted Congruential Generator) provides excellent statistical
/// properties for Monte Carlo simulations:
///
/// - **Period**: 2^128 (very long, no overlap in practice)
/// - **Quality**: Passes all TestU01 and PractRand statistical tests
/// - **Speed**: ~2ns per sample on modern hardware
/// - **Deterministic**: Same seed always produces same sequence
///
/// Methods
/// -------
/// uniform()
///     Draw U(0, 1) variates.
/// normal(mean, std_dev)
///     Draw Normal variates.
/// bernoulli(p)
///     Draw Bernoulli trials.
///
/// Examples
/// --------
/// >>> from finstack.core.math.random import Rng
/// >>> rng = Rng(42)
/// >>> rng.uniform()  # U(0, 1)
/// >>> rng.normal(0.0, 1.0)  # N(0, 1)
/// >>> rng.bernoulli(0.5)  # Coin flip
#[derive(Clone, Debug)]
pub struct PyRng {
    inner: Pcg64Rng,
}

#[pymethods]
impl PyRng {
    #[new]
    #[pyo3(text_signature = "(seed)")]
    /// Create a new RNG with the given integer seed.
    ///
    /// Parameters
    /// ----------
    /// seed : int
    ///     Seed for the underlying generator. The same seed yields the same sequence.
    ///
    /// Examples
    /// --------
    /// >>> rng1 = Rng(42)
    /// >>> rng2 = Rng(42)
    /// >>> rng1.uniform() == rng2.uniform()  # Same seed, same sequence
    /// True
    pub fn new(seed: u64) -> Self {
        Self {
            inner: Pcg64Rng::new(seed),
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
        "Rng(seed=<internal>, type=PCG64)".to_string()
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

// ============================================================================
// Sobol quasi-random number generator
// ============================================================================

#[pyclass(name = "SobolRng", module = "finstack.core.math.random")]
/// Sobol quasi-random sequence generator with Owen scrambling.
///
/// Sobol sequences are low-discrepancy quasi-random sequences that provide
/// better convergence than pseudo-random for smooth payoffs in Monte Carlo.
///
/// Parameters
/// ----------
/// dimension : int
///     Number of dimensions (1 to ``MAX_DIMENSION``).
/// scramble_seed : int, optional
///     Seed for Owen scrambling (default ``0`` = no scrambling).
///
/// Attributes
/// ----------
/// MAX_DIMENSION : int
///     Maximum supported dimension (40).
///
/// Methods
/// -------
/// next_point()
///     Return the next quasi-random point as a list of floats.
/// fill_point(buf_len)
///     Return the next point filling a buffer of the given length.
/// reset()
///     Reset the sequence to the beginning.
///
/// Examples
/// --------
/// >>> from finstack.core.math.random import SobolRng
/// >>> sobol = SobolRng(3, scramble_seed=12345)
/// >>> point = sobol.next_point()
/// >>> len(point)
/// 3
pub struct PySobolRng {
    inner: SobolRng,
}

#[pymethods]
impl PySobolRng {
    #[new]
    #[pyo3(signature = (dimension, scramble_seed = 0))]
    /// Create a new Sobol sequence generator.
    ///
    /// Parameters
    /// ----------
    /// dimension : int
    ///     Number of dimensions (1 to ``MAX_DIMENSION``).
    /// scramble_seed : int, optional
    ///     Seed for Owen scrambling (default ``0`` = no scrambling).
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``dimension`` is 0 or exceeds ``MAX_DIMENSION``.
    pub fn new(dimension: usize, scramble_seed: u64) -> PyResult<Self> {
        let inner = SobolRng::try_new(dimension, scramble_seed).map_err(core_to_py)?;
        Ok(Self { inner })
    }

    #[classattr]
    /// Maximum supported dimension for this Sobol implementation.
    const MAX_DIMENSION: usize = MAX_SOBOL_DIMENSION;

    #[pyo3(text_signature = "($self)")]
    /// Return the next quasi-random point as a list of floats in [0, 1).
    ///
    /// Returns
    /// -------
    /// list[float]
    ///     Point with ``dimension`` components, each in ``[0, 1)``.
    pub fn next_point(&mut self) -> Vec<f64> {
        self.inner.next_point()
    }

    #[pyo3(text_signature = "($self, buf_len)")]
    /// Return the next point filling a buffer of the given length.
    ///
    /// Parameters
    /// ----------
    /// buf_len : int
    ///     Length of the output buffer. Values beyond the generator's
    ///     dimension are left as zero.
    ///
    /// Returns
    /// -------
    /// list[float]
    ///     Buffer with ``min(buf_len, dimension)`` filled values.
    pub fn fill_point(&mut self, buf_len: usize) -> Vec<f64> {
        let mut buf = vec![0.0; buf_len];
        self.inner.fill_point(&mut buf);
        buf
    }

    #[pyo3(text_signature = "($self)")]
    /// Reset the sequence to the beginning (index 0).
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    pub fn __repr__(&self) -> String {
        format!("SobolRng(max_dim={})", MAX_SOBOL_DIMENSION)
    }
}

// ============================================================================
// Brownian bridge construction
// ============================================================================

#[pyclass(name = "BrownianBridge", module = "finstack.core.math.random", frozen)]
/// Brownian bridge construction for path-dependent Monte Carlo.
///
/// Reorders random shocks using binary subdivision to reduce effective
/// dimension for quasi-Monte Carlo methods. Particularly effective for
/// barrier and path-dependent options.
///
/// Parameters
/// ----------
/// num_steps : int
///     Number of time steps in the path.
///
/// Methods
/// -------
/// order
///     Construction order (indices into time grid).
/// multipliers
///     Standard-deviation multipliers for conditional variance.
/// construct_path(z, dt)
///     Build a Brownian path from independent shocks on a uniform grid.
/// construct_path_irregular(z, times)
///     Build a Brownian path from independent shocks on an irregular grid.
///
/// Examples
/// --------
/// >>> from finstack.core.math.random import BrownianBridge
/// >>> bb = BrownianBridge(4)
/// >>> bb.order
/// [2, 1, 3]
pub struct PyBrownianBridge {
    inner: BrownianBridge,
}

#[pymethods]
impl PyBrownianBridge {
    #[new]
    #[pyo3(text_signature = "(num_steps)")]
    /// Create a Brownian bridge construction for the given number of steps.
    ///
    /// Parameters
    /// ----------
    /// num_steps : int
    ///     Number of time steps in the path.
    pub fn new(num_steps: usize) -> Self {
        Self {
            inner: BrownianBridge::new(num_steps),
        }
    }

    /// Construction order as a list of time-grid indices.
    ///
    /// Returns
    /// -------
    /// list[int]
    ///     Indices into the time grid in bridge construction order.
    #[getter]
    pub fn order(&self) -> Vec<usize> {
        self.inner.order().to_vec()
    }

    /// Standard-deviation multipliers for the conditional variance at each
    /// construction step.
    ///
    /// Returns
    /// -------
    /// list[float]
    ///     One multiplier per construction step.
    #[getter]
    pub fn multipliers(&self) -> Vec<f64> {
        self.inner.multipliers().to_vec()
    }

    #[pyo3(text_signature = "($self, z, dt)")]
    /// Build a Brownian path from independent standard-normal shocks.
    ///
    /// Parameters
    /// ----------
    /// z : list[float]
    ///     Independent standard-normal shocks (length = ``num_steps``).
    /// dt : float
    ///     Uniform time-step size.
    ///
    /// Returns
    /// -------
    /// list[float]
    ///     Brownian path of length ``num_steps + 1`` with ``w[0] = 0``.
    pub fn construct_path(&self, z: Vec<f64>, dt: f64) -> Vec<f64> {
        let mut w_out = vec![0.0; z.len() + 1];
        self.inner.construct_path(&z, &mut w_out, dt);
        w_out
    }

    #[pyo3(text_signature = "($self, z, times)")]
    /// Build a Brownian path on an irregular time grid.
    ///
    /// Parameters
    /// ----------
    /// z : list[float]
    ///     Independent standard-normal shocks (length = ``num_steps``).
    /// times : list[float]
    ///     Monotonically increasing time points (length = ``num_steps + 1``),
    ///     with ``times[0] == 0.0``.
    ///
    /// Returns
    /// -------
    /// list[float]
    ///     Brownian path of length ``num_steps + 1`` with ``w[0] = 0``.
    pub fn construct_path_irregular(&self, z: Vec<f64>, times: Vec<f64>) -> Vec<f64> {
        let mut w_out = vec![0.0; z.len() + 1];
        self.inner.construct_path_irregular(&z, &mut w_out, &times);
        w_out
    }

    pub fn __repr__(&self) -> String {
        format!(
            "BrownianBridge(steps={}, order_len={})",
            self.inner.order().len() + 1,
            self.inner.order().len()
        )
    }
}

// ============================================================================
// Poisson distribution free functions
// ============================================================================

#[pyfunction(name = "poisson_inverse_cdf")]
#[pyo3(text_signature = "(lambda_, u)")]
/// Sample from the Poisson distribution using inverse CDF.
///
/// Parameters
/// ----------
/// lambda_ : float
///     Mean number of events (lambda).
/// u : float
///     Uniform random variable in ``[0, 1)``.
///
/// Returns
/// -------
/// int
///     Number of Poisson events.
pub fn py_poisson_inverse_cdf(lambda_: f64, u: f64) -> u64 {
    core_poisson_inverse_cdf(lambda_, u) as u64
}

#[pyfunction(name = "poisson_from_normal")]
#[pyo3(text_signature = "(lambda_, z)")]
/// Sample from the Poisson distribution via a standard-normal input.
///
/// Converts a standard normal variate to a Poisson sample through CDF
/// transform.
///
/// Parameters
/// ----------
/// lambda_ : float
///     Mean number of events (lambda).
/// z : float
///     Standard-normal variate.
///
/// Returns
/// -------
/// int
///     Number of Poisson events.
pub fn py_poisson_from_normal(lambda_: f64, z: f64) -> u64 {
    core_poisson_from_normal(lambda_, z) as u64
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "random")?;
    module.setattr(
        "__doc__",
        "Random number utilities from finstack-core.\n\n\
         Classes:\n\
         - Rng: Production-grade PCG64 random number generator\n\
         - SobolRng: Sobol quasi-random sequence with Owen scrambling\n\
         - BrownianBridge: Brownian bridge construction for path-dependent MC\n\n\
         Functions:\n\
         - box_muller_transform: Generate standard normal samples from uniform inputs\n\
         - poisson_inverse_cdf: Poisson sampling via inverse CDF\n\
         - poisson_from_normal: Poisson sampling from standard normal input",
    )?;

    // Register classes
    module.add_class::<PyRng>()?;
    module.add_class::<PySobolRng>()?;
    module.add_class::<PyBrownianBridge>()?;

    // Register functions
    module.add_function(wrap_pyfunction!(box_muller_transform_py, &module)?)?;
    module.add_function(wrap_pyfunction!(py_poisson_inverse_cdf, &module)?)?;
    module.add_function(wrap_pyfunction!(py_poisson_from_normal, &module)?)?;

    let exports = [
        "BrownianBridge",
        "Rng",
        "SobolRng",
        "box_muller_transform",
        "poisson_from_normal",
        "poisson_inverse_cdf",
    ];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
