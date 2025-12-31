use finstack_core::math::distributions::{
    binomial_probability as core_binomial_probability,
    log_binomial_coefficient as core_log_binomial_coefficient, log_factorial as core_log_factorial,
    sample_beta as core_sample_beta,
};
use finstack_core::math::random::{Pcg64Rng, RandomNumberGenerator};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;

#[pyfunction(
    name = "binomial_probability",
    text_signature = "(trials, successes, probability)"
)]
/// Compute the probability mass at a target count for a Binomial distribution.
///
/// Computes the probability ``P(X = successes)`` where ``X ~ Binomial(trials, probability)``.
///
/// Args:
///     trials (int): Total number of Bernoulli trials (``n``).
///     successes (int): Target number of successes (``k``).
///     probability (float): Probability of success per trial (``p``), in the range [0, 1].
///
/// Returns:
///     float: Probability mass at ``successes``.
///
/// Examples:
///     >>> from finstack.core.math.distributions import binomial_probability
///     >>> binomial_probability(10, 3, 0.5)
///     0.1171875
pub fn binomial_probability_py(trials: usize, successes: usize, probability: f64) -> PyResult<f64> {
    Ok(core_binomial_probability(trials, successes, probability))
}

#[pyfunction(
    name = "log_binomial_coefficient",
    text_signature = "(trials, successes)"
)]
/// Natural logarithm of the binomial coefficient.
///
/// Computes ``ln(C(trials, successes)) = ln(n! / (k!(n-k)!))``.
///
/// Args:
///     trials (int): Total number of items (``n``).
///     successes (int): Number of items chosen (``k``).
///
/// Returns:
///     float: Natural logarithm of the binomial coefficient.
///
/// Examples:
///     >>> from finstack.core.math.distributions import log_binomial_coefficient
///     >>> round(log_binomial_coefficient(5, 2), 6)
///     2.397895
pub fn log_binomial_coefficient_py(trials: usize, successes: usize) -> PyResult<f64> {
    Ok(core_log_binomial_coefficient(trials, successes))
}

#[pyfunction(name = "log_factorial", text_signature = "(value)")]
/// Natural logarithm of a factorial.
///
/// Computes ``ln(value!)`` using exact arithmetic for small values and a
/// stable approximation (e.g., Stirling-like) when needed.
///
/// Args:
///     value (int): Non-negative integer ``n`` whose factorial is evaluated.
///
/// Returns:
///     float: ``ln(n!)``.
///
/// Raises:
///     ValueError: If ``value`` is negative.
///
/// Examples:
///     >>> from finstack.core.math.distributions import log_factorial
///     >>> round(log_factorial(5), 6)
///     4.787492
pub fn log_factorial_py(value: usize) -> PyResult<f64> {
    Ok(core_log_factorial(value))
}

#[pyfunction(name = "sample_beta", text_signature = "(alpha, beta, seed=None)")]
/// Sample from a Beta(α, β) distribution using the core RNG implementation.
///
/// Parameters
/// ----------
/// alpha : float
///     First shape parameter (must be positive).
/// beta : float
///     Second shape parameter (must be positive).
/// seed : int, optional
///     Optional RNG seed for deterministic sampling. If omitted, a fixed
///     default seed is used for reproducible examples.
///
/// Returns
/// -------
/// float
///     Sample in ``[0.0, 1.0]`` drawn from ``Beta(alpha, beta)``.
///
/// Raises
/// ------
/// ValueError
///     If ``alpha`` or ``beta`` are not positive.
pub fn sample_beta_py(alpha: f64, beta: f64, seed: Option<u64>) -> PyResult<f64> {
    if alpha <= 0.0 || beta <= 0.0 {
        return Err(PyValueError::new_err("alpha and beta must be positive"));
    }
    let mut rng = Pcg64Rng::new(seed.unwrap_or(42));
    Ok(core_sample_beta(
        &mut rng as &mut dyn RandomNumberGenerator,
        alpha,
        beta,
    ))
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "distributions")?;
    module.setattr(
        "__doc__",
        "Mathematical distribution helpers (probabilities, logarithms).",
    )?;
    module.add_function(wrap_pyfunction!(binomial_probability_py, &module)?)?;
    module.add_function(wrap_pyfunction!(log_binomial_coefficient_py, &module)?)?;
    module.add_function(wrap_pyfunction!(log_factorial_py, &module)?)?;
    module.add_function(wrap_pyfunction!(sample_beta_py, &module)?)?;
    let exports = [
        "binomial_probability",
        "log_binomial_coefficient",
        "log_factorial",
        "sample_beta",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
