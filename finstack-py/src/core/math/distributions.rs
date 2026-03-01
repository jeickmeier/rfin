use finstack_core::math::distributions::{
    binomial_distribution as core_binomial_distribution,
    binomial_probability as core_binomial_probability, chi_squared_cdf as core_chi_squared_cdf,
    chi_squared_pdf as core_chi_squared_pdf, chi_squared_quantile as core_chi_squared_quantile,
    exponential_cdf as core_exponential_cdf, exponential_pdf as core_exponential_pdf,
    exponential_quantile as core_exponential_quantile,
    log_binomial_coefficient as core_log_binomial_coefficient, log_factorial as core_log_factorial,
    lognormal_cdf as core_lognormal_cdf, lognormal_pdf as core_lognormal_pdf,
    lognormal_quantile as core_lognormal_quantile, sample_beta as core_sample_beta,
    sample_chi_squared as core_sample_chi_squared, sample_exponential as core_sample_exponential,
    sample_gamma as core_sample_gamma, sample_lognormal as core_sample_lognormal,
    sample_student_t as core_sample_student_t,
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
    name = "binomial_distribution",
    text_signature = "(trials, probability)"
)]
/// Generate the complete binomial distribution P(X=k) for k = 0, 1, ..., n.
///
/// Returns a normalized probability vector where ``dist[k]`` = P(X = k).
/// Uses log-space arithmetic to prevent overflow for large n.
///
/// Args:
///     trials (int): Number of independent trials (n ≥ 0).
///     probability (float): Probability of success on each trial (0 ≤ p ≤ 1).
///
/// Returns:
///     list[float]: Vector of probabilities [P(X=0), P(X=1), ..., P(X=n)] with length n+1.
///         The vector sums to 1.0 (normalized).
///
/// Use Cases:
///     - **Credit modeling**: Loss distribution for homogeneous pool of n obligors
///     - **Portfolio analytics**: Number of defaults given conditional default probability
///     - **Structured credit**: Default distribution for CDO/CLO tranches
///
/// Examples:
///     >>> from finstack.core.math.distributions import binomial_distribution
///     >>> dist = binomial_distribution(10, 0.5)
///     >>> len(dist)
///     11
///     >>> round(dist[5], 8)  # P(X=5) for fair coin
///     0.24609375
///     >>> round(sum(dist), 10)  # Sums to 1.0
///     1.0
///
///     Credit portfolio example:
///
///     >>> loss_dist = binomial_distribution(100, 0.05)  # 100 names, 5% PD
///     >>> len(loss_dist)
///     101
pub fn binomial_distribution_py(trials: usize, probability: f64) -> PyResult<Vec<f64>> {
    Ok(core_binomial_distribution(trials, probability))
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
///     RNG seed for deterministic sampling. **If omitted, defaults to 42 and
///     every call without an explicit seed returns the same value.** For
///     multiple independent samples, supply distinct seeds or use
///     :class:`finstack.core.math.random.Rng` directly.
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
    let mut rng = Pcg64Rng::new(seed.unwrap_or(42));
    core_sample_beta(&mut rng as &mut dyn RandomNumberGenerator, alpha, beta)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

// Exponential distribution functions
#[pyfunction(name = "exponential_pdf", text_signature = "(x, lambda_)")]
/// Probability density function of the exponential distribution.
///
/// Args:
///     x (float): Point at which to evaluate the PDF (must be non-negative).
///     lambda_ (float): Rate parameter (must be positive).
///
/// Returns:
///     float: Probability density at ``x``.
///
/// Examples:
///     >>> from finstack.core.math.distributions import exponential_pdf
///     >>> exponential_pdf(1.0, 1.0)
///     0.36787944117144233
pub fn exponential_pdf_py(x: f64, lambda_: f64) -> PyResult<f64> {
    if lambda_ <= 0.0 {
        return Err(PyValueError::new_err("lambda must be positive"));
    }
    Ok(core_exponential_pdf(x, lambda_))
}

#[pyfunction(name = "exponential_cdf", text_signature = "(x, lambda_)")]
/// Cumulative distribution function of the exponential distribution.
///
/// Args:
///     x (float): Point at which to evaluate the CDF.
///     lambda_ (float): Rate parameter (must be positive).
///
/// Returns:
///     float: Cumulative probability up to ``x``.
///
/// Examples:
///     >>> from finstack.core.math.distributions import exponential_cdf
///     >>> exponential_cdf(1.0, 1.0)
///     0.6321205588285577
pub fn exponential_cdf_py(x: f64, lambda_: f64) -> PyResult<f64> {
    if lambda_ <= 0.0 {
        return Err(PyValueError::new_err("lambda must be positive"));
    }
    Ok(core_exponential_cdf(x, lambda_))
}

#[pyfunction(name = "exponential_quantile", text_signature = "(p, lambda_)")]
/// Quantile function (inverse CDF) of the exponential distribution.
///
/// Args:
///     p (float): Probability level in [0, 1].
///     lambda_ (float): Rate parameter (must be positive).
///
/// Returns:
///     float: Quantile at probability ``p``.
///
/// Examples:
///     >>> from finstack.core.math.distributions import exponential_quantile
///     >>> exponential_quantile(0.5, 1.0)
///     0.6931471805599453
pub fn exponential_quantile_py(p: f64, lambda_: f64) -> PyResult<f64> {
    core_exponential_quantile(p, lambda_).map_err(|e| PyValueError::new_err(e.to_string()))
}

#[pyfunction(name = "sample_exponential", text_signature = "(lambda_, seed=None)")]
/// Sample from an exponential distribution.
///
/// Parameters
/// ----------
/// lambda_ : float
///     Rate parameter (must be positive).
/// seed : int, optional
///     RNG seed for deterministic sampling. **If omitted, defaults to 42 and
///     every call without an explicit seed returns the same value.** For
///     multiple independent samples, supply distinct seeds or use
///     :class:`finstack.core.math.random.Rng` directly.
///
/// Returns
/// -------
/// float
///     Sample from Exp(lambda).
///
/// Examples
/// --------
/// >>> from finstack.core.math.distributions import sample_exponential
/// >>> sample_exponential(1.0, seed=42)  # doctest: +SKIP
pub fn sample_exponential_py(lambda_: f64, seed: Option<u64>) -> PyResult<f64> {
    let mut rng = Pcg64Rng::new(seed.unwrap_or(42));
    core_sample_exponential(&mut rng as &mut dyn RandomNumberGenerator, lambda_)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

// Lognormal distribution functions
#[pyfunction(name = "lognormal_pdf", text_signature = "(x, mu, sigma)")]
/// Probability density function of the lognormal distribution.
///
/// Args:
///     x (float): Point at which to evaluate the PDF (must be positive).
///     mu (float): Mean of the underlying normal distribution.
///     sigma (float): Standard deviation of the underlying normal (must be positive).
///
/// Returns:
///     float: Probability density at ``x``.
///
/// Examples:
///     >>> from finstack.core.math.distributions import lognormal_pdf
///     >>> lognormal_pdf(1.0, 0.0, 1.0)  # doctest: +SKIP
pub fn lognormal_pdf_py(x: f64, mu: f64, sigma: f64) -> PyResult<f64> {
    if sigma <= 0.0 {
        return Err(PyValueError::new_err("sigma must be positive"));
    }
    Ok(core_lognormal_pdf(x, mu, sigma))
}

#[pyfunction(name = "lognormal_cdf", text_signature = "(x, mu, sigma)")]
/// Cumulative distribution function of the lognormal distribution.
///
/// Args:
///     x (float): Point at which to evaluate the CDF.
///     mu (float): Mean of the underlying normal distribution.
///     sigma (float): Standard deviation of the underlying normal (must be positive).
///
/// Returns:
///     float: Cumulative probability up to ``x``.
///
/// Examples:
///     >>> from finstack.core.math.distributions import lognormal_cdf
///     >>> lognormal_cdf(1.0, 0.0, 1.0)
///     0.5
pub fn lognormal_cdf_py(x: f64, mu: f64, sigma: f64) -> PyResult<f64> {
    if sigma <= 0.0 {
        return Err(PyValueError::new_err("sigma must be positive"));
    }
    Ok(core_lognormal_cdf(x, mu, sigma))
}

#[pyfunction(name = "lognormal_quantile", text_signature = "(p, mu, sigma)")]
/// Quantile function (inverse CDF) of the lognormal distribution.
///
/// Args:
///     p (float): Probability level in [0, 1].
///     mu (float): Mean of the underlying normal distribution.
///     sigma (float): Standard deviation of the underlying normal (must be positive).
///
/// Returns:
///     float: Quantile at probability ``p``.
///
/// Examples:
///     >>> from finstack.core.math.distributions import lognormal_quantile
///     >>> lognormal_quantile(0.5, 0.0, 1.0)
///     1.0
pub fn lognormal_quantile_py(p: f64, mu: f64, sigma: f64) -> PyResult<f64> {
    core_lognormal_quantile(p, mu, sigma).map_err(|e| PyValueError::new_err(e.to_string()))
}

#[pyfunction(name = "sample_lognormal", text_signature = "(mu, sigma, seed=None)")]
/// Sample from a lognormal distribution.
///
/// Parameters
/// ----------
/// mu : float
///     Mean of the underlying normal distribution.
/// sigma : float
///     Standard deviation of the underlying normal (must be positive).
/// seed : int, optional
///     RNG seed for deterministic sampling. **If omitted, defaults to 42 and
///     every call without an explicit seed returns the same value.** For
///     multiple independent samples, supply distinct seeds or use
///     :class:`finstack.core.math.random.Rng` directly.
///
/// Returns
/// -------
/// float
///     Sample from LogNormal(mu, sigma).
///
/// Examples
/// --------
/// >>> from finstack.core.math.distributions import sample_lognormal
/// >>> sample_lognormal(0.0, 1.0, seed=42)  # doctest: +SKIP
pub fn sample_lognormal_py(mu: f64, sigma: f64, seed: Option<u64>) -> PyResult<f64> {
    let mut rng = Pcg64Rng::new(seed.unwrap_or(42));
    core_sample_lognormal(&mut rng as &mut dyn RandomNumberGenerator, mu, sigma)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

// Chi-squared distribution functions
#[pyfunction(name = "chi_squared_pdf", text_signature = "(x, df)")]
/// Probability density function of the chi-squared distribution.
///
/// Args:
///     x (float): Point at which to evaluate the PDF (must be non-negative).
///     df (float): Degrees of freedom (must be positive).
///
/// Returns:
///     float: Probability density at ``x``.
///
/// Examples:
///     >>> from finstack.core.math.distributions import chi_squared_pdf
///     >>> chi_squared_pdf(1.0, 2.0)  # doctest: +SKIP
pub fn chi_squared_pdf_py(x: f64, df: f64) -> PyResult<f64> {
    if df <= 0.0 {
        return Err(PyValueError::new_err("df must be positive"));
    }
    Ok(core_chi_squared_pdf(x, df))
}

#[pyfunction(name = "chi_squared_cdf", text_signature = "(x, df)")]
/// Cumulative distribution function of the chi-squared distribution.
///
/// Args:
///     x (float): Point at which to evaluate the CDF.
///     df (float): Degrees of freedom (must be positive).
///
/// Returns:
///     float: Cumulative probability up to ``x``.
///
/// Examples:
///     >>> from finstack.core.math.distributions import chi_squared_cdf
///     >>> chi_squared_cdf(1.0, 2.0)
///     0.3934693402873666
pub fn chi_squared_cdf_py(x: f64, df: f64) -> PyResult<f64> {
    if df <= 0.0 {
        return Err(PyValueError::new_err("df must be positive"));
    }
    Ok(core_chi_squared_cdf(x, df))
}

#[pyfunction(name = "chi_squared_quantile", text_signature = "(p, df)")]
/// Quantile function (inverse CDF) of the chi-squared distribution.
///
/// Args:
///     p (float): Probability level in [0, 1].
///     df (float): Degrees of freedom (must be positive).
///
/// Returns:
///     float: Quantile at probability ``p``.
///
/// Examples:
///     >>> from finstack.core.math.distributions import chi_squared_quantile
///     >>> chi_squared_quantile(0.95, 2.0)  # doctest: +SKIP
pub fn chi_squared_quantile_py(p: f64, df: f64) -> PyResult<f64> {
    core_chi_squared_quantile(p, df).map_err(|e| PyValueError::new_err(e.to_string()))
}

#[pyfunction(name = "sample_chi_squared", text_signature = "(df, seed=None)")]
/// Sample from a chi-squared distribution.
///
/// Parameters
/// ----------
/// df : float
///     Degrees of freedom (must be positive).
/// seed : int, optional
///     RNG seed for deterministic sampling. **If omitted, defaults to 42 and
///     every call without an explicit seed returns the same value.** For
///     multiple independent samples, supply distinct seeds or use
///     :class:`finstack.core.math.random.Rng` directly.
///
/// Returns
/// -------
/// float
///     Sample from ChiSquared(df).
///
/// Examples
/// --------
/// >>> from finstack.core.math.distributions import sample_chi_squared
/// >>> sample_chi_squared(2.0, seed=42)  # doctest: +SKIP
pub fn sample_chi_squared_py(df: f64, seed: Option<u64>) -> PyResult<f64> {
    let mut rng = Pcg64Rng::new(seed.unwrap_or(42));
    core_sample_chi_squared(&mut rng as &mut dyn RandomNumberGenerator, df)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

// Gamma and Student-t sampling functions
#[pyfunction(name = "sample_gamma", text_signature = "(shape, seed=None)")]
/// Sample from a gamma distribution with unit scale.
///
/// Parameters
/// ----------
/// shape : float
///     Shape parameter (must be positive).
/// seed : int, optional
///     RNG seed for deterministic sampling. **If omitted, defaults to 42 and
///     every call without an explicit seed returns the same value.** For
///     multiple independent samples, supply distinct seeds or use
///     :class:`finstack.core.math.random.Rng` directly.
///
/// Returns
/// -------
/// float
///     Sample from Gamma(shape, scale=1).
///
/// Examples
/// --------
/// >>> from finstack.core.math.distributions import sample_gamma
/// >>> sample_gamma(2.0, seed=42)  # doctest: +SKIP
pub fn sample_gamma_py(shape: f64, seed: Option<u64>) -> PyResult<f64> {
    let mut rng = Pcg64Rng::new(seed.unwrap_or(42));
    core_sample_gamma(&mut rng as &mut dyn RandomNumberGenerator, shape)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

#[pyfunction(name = "sample_student_t", text_signature = "(df, seed=None)")]
/// Sample from a Student's t-distribution.
///
/// Parameters
/// ----------
/// df : float
///     Degrees of freedom (must be positive).
/// seed : int, optional
///     RNG seed for deterministic sampling. **If omitted, defaults to 42 and
///     every call without an explicit seed returns the same value.** For
///     multiple independent samples, supply distinct seeds or use
///     :class:`finstack.core.math.random.Rng` directly.
///
/// Returns
/// -------
/// float
///     Sample from t(df).
///
/// Examples
/// --------
/// >>> from finstack.core.math.distributions import sample_student_t
/// >>> sample_student_t(10.0, seed=42)  # doctest: +SKIP
pub fn sample_student_t_py(df: f64, seed: Option<u64>) -> PyResult<f64> {
    let mut rng = Pcg64Rng::new(seed.unwrap_or(42));
    core_sample_student_t(&mut rng as &mut dyn RandomNumberGenerator, df)
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "distributions")?;
    module.setattr(
        "__doc__",
        concat!(
            "Mathematical distribution helpers (CDFs, PDFs, quantiles, and sampling).\n\n",
            "Provides binomial, exponential, lognormal, chi-squared, gamma, and Student's t distributions."
        ),
    )?;

    // Binomial
    module.add_function(wrap_pyfunction!(binomial_probability_py, &module)?)?;
    module.add_function(wrap_pyfunction!(binomial_distribution_py, &module)?)?;
    module.add_function(wrap_pyfunction!(log_binomial_coefficient_py, &module)?)?;
    module.add_function(wrap_pyfunction!(log_factorial_py, &module)?)?;

    // Exponential
    module.add_function(wrap_pyfunction!(exponential_pdf_py, &module)?)?;
    module.add_function(wrap_pyfunction!(exponential_cdf_py, &module)?)?;
    module.add_function(wrap_pyfunction!(exponential_quantile_py, &module)?)?;
    module.add_function(wrap_pyfunction!(sample_exponential_py, &module)?)?;

    // Lognormal
    module.add_function(wrap_pyfunction!(lognormal_pdf_py, &module)?)?;
    module.add_function(wrap_pyfunction!(lognormal_cdf_py, &module)?)?;
    module.add_function(wrap_pyfunction!(lognormal_quantile_py, &module)?)?;
    module.add_function(wrap_pyfunction!(sample_lognormal_py, &module)?)?;

    // Chi-squared
    module.add_function(wrap_pyfunction!(chi_squared_pdf_py, &module)?)?;
    module.add_function(wrap_pyfunction!(chi_squared_cdf_py, &module)?)?;
    module.add_function(wrap_pyfunction!(chi_squared_quantile_py, &module)?)?;
    module.add_function(wrap_pyfunction!(sample_chi_squared_py, &module)?)?;

    // Gamma and Student-t
    module.add_function(wrap_pyfunction!(sample_gamma_py, &module)?)?;
    module.add_function(wrap_pyfunction!(sample_student_t_py, &module)?)?;
    module.add_function(wrap_pyfunction!(sample_beta_py, &module)?)?;

    let exports = [
        "binomial_probability",
        "binomial_distribution",
        "log_binomial_coefficient",
        "log_factorial",
        "exponential_pdf",
        "exponential_cdf",
        "exponential_quantile",
        "sample_exponential",
        "lognormal_pdf",
        "lognormal_cdf",
        "lognormal_quantile",
        "sample_lognormal",
        "chi_squared_pdf",
        "chi_squared_cdf",
        "chi_squared_quantile",
        "sample_chi_squared",
        "sample_gamma",
        "sample_student_t",
        "sample_beta",
    ];
    module.setattr("__all__", PyList::new(py, exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
