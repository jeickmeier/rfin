"""Statistical distribution bindings.

Provides binomial, exponential, lognormal, chi-squared, gamma, and
Student's t distributions with CDFs, PDFs, quantiles, and sampling.
"""

from typing import List

def binomial_probability(trials: int, successes: int, probability: float) -> float:
    """Compute the probability mass P(X = successes) for Binomial(trials, probability).

    Parameters
    ----------
    trials : int
        Number of trials (n ≥ 0).
    successes : int
        Number of successes (0 ≤ k ≤ n).
    probability : float
        Success probability per trial (0 ≤ p ≤ 1).

    Returns
    -------
    float
        Probability mass at successes.
    """
    ...

def binomial_distribution(trials: int, probability: float) -> List[float]:
    """Generate the complete binomial distribution P(X=k) for k = 0, 1, ..., n.

    Returns a normalized probability vector where dist[k] = P(X = k).

    Parameters
    ----------
    trials : int
        Number of trials (n ≥ 0).
    probability : float
        Success probability per trial (0 ≤ p ≤ 1).

    Returns
    -------
    list[float]
        Vector [P(X=0), P(X=1), ..., P(X=n)] with length n+1, summing to 1.0.

    Examples
    --------
    >>> dist = binomial_distribution(10, 0.5)
    >>> len(dist)
    11
    >>> round(dist[5], 8)  # P(X=5) for fair coin
    0.24609375
    """
    ...

def log_binomial_coefficient(trials: int, successes: int) -> float:
    """Natural logarithm of the binomial coefficient ln(C(n, k)).

    Parameters
    ----------
    trials : int
        Total number of items (n).
    successes : int
        Number of items chosen (k).

    Returns
    -------
    float
        ln(C(n, k)), or -inf if k > n.
    """
    ...

def log_factorial(value: int) -> float:
    """Natural logarithm of factorial ln(n!).

    Parameters
    ----------
    value : int
        Non-negative integer.

    Returns
    -------
    float
        ln(n!).
    """
    ...

def sample_beta(alpha: float, beta: float, seed: int | None = ...) -> float:
    """Sample from a Beta(α, β) distribution.

    Parameters
    ----------
    alpha : float
        First shape parameter (> 0).
    beta : float
        Second shape parameter (> 0).
    seed : int, optional
        RNG seed for deterministic sampling.

    Returns
    -------
    float
        Sample in [0.0, 1.0].
    """
    ...

def exponential_pdf(x: float, lambda_: float) -> float:
    """Probability density function of Exponential(λ).

    Parameters
    ----------
    x : float
        Point at which to evaluate.
    lambda_ : float
        Rate parameter (> 0).

    Returns
    -------
    float
        PDF value.
    """
    ...

def exponential_cdf(x: float, lambda_: float) -> float:
    """Cumulative distribution function of Exponential(λ).

    Parameters
    ----------
    x : float
        Point at which to evaluate.
    lambda_ : float
        Rate parameter (> 0).

    Returns
    -------
    float
        CDF value in [0, 1].
    """
    ...

def exponential_quantile(p: float, lambda_: float) -> float:
    """Quantile function (inverse CDF) of Exponential(λ).

    Parameters
    ----------
    p : float
        Probability in [0, 1).
    lambda_ : float
        Rate parameter (> 0).

    Returns
    -------
    float
        Quantile value.
    """
    ...

def sample_exponential(lambda_: float, seed: int | None = ...) -> float:
    """Sample from Exponential(λ) distribution.

    Parameters
    ----------
    lambda_ : float
        Rate parameter (> 0).
    seed : int, optional
        RNG seed for deterministic sampling.

    Returns
    -------
    float
        Sample ≥ 0.
    """
    ...

def lognormal_pdf(x: float, mu: float, sigma: float) -> float:
    """Probability density function of LogNormal(μ, σ).

    Parameters
    ----------
    x : float
        Point at which to evaluate (> 0).
    mu : float
        Mean of underlying normal.
    sigma : float
        Std dev of underlying normal (> 0).

    Returns
    -------
    float
        PDF value.
    """
    ...

def lognormal_cdf(x: float, mu: float, sigma: float) -> float:
    """Cumulative distribution function of LogNormal(μ, σ).

    Parameters
    ----------
    x : float
        Point at which to evaluate.
    mu : float
        Mean of underlying normal.
    sigma : float
        Std dev of underlying normal (> 0).

    Returns
    -------
    float
        CDF value in [0, 1].
    """
    ...

def lognormal_quantile(p: float, mu: float, sigma: float) -> float:
    """Quantile function (inverse CDF) of LogNormal(μ, σ).

    Parameters
    ----------
    p : float
        Probability in (0, 1).
    mu : float
        Mean of underlying normal.
    sigma : float
        Std dev of underlying normal (> 0).

    Returns
    -------
    float
        Quantile value.
    """
    ...

def sample_lognormal(mu: float, sigma: float, seed: int | None = ...) -> float:
    """Sample from LogNormal(μ, σ) distribution.

    Parameters
    ----------
    mu : float
        Mean of underlying normal.
    sigma : float
        Std dev of underlying normal (> 0).
    seed : int, optional
        RNG seed for deterministic sampling.

    Returns
    -------
    float
        Sample > 0.
    """
    ...

def chi_squared_pdf(x: float, df: float) -> float:
    """Probability density function of Chi-squared(k).

    Parameters
    ----------
    x : float
        Point at which to evaluate (≥ 0).
    df : float
        Degrees of freedom (> 0).

    Returns
    -------
    float
        PDF value.
    """
    ...

def chi_squared_cdf(x: float, df: float) -> float:
    """Cumulative distribution function of Chi-squared(k).

    Parameters
    ----------
    x : float
        Point at which to evaluate.
    df : float
        Degrees of freedom (> 0).

    Returns
    -------
    float
        CDF value in [0, 1].
    """
    ...

def chi_squared_quantile(p: float, df: float) -> float:
    """Quantile function (inverse CDF) of Chi-squared(k).

    Parameters
    ----------
    p : float
        Probability in [0, 1).
    df : float
        Degrees of freedom (> 0).

    Returns
    -------
    float
        Quantile value.
    """
    ...

def sample_chi_squared(df: float, seed: int | None = ...) -> float:
    """Sample from Chi-squared(k) distribution.

    Parameters
    ----------
    df : float
        Degrees of freedom (> 0).
    seed : int, optional
        RNG seed for deterministic sampling.

    Returns
    -------
    float
        Sample ≥ 0.
    """
    ...

def sample_gamma(shape: float, seed: int | None = ...) -> float:
    """Sample from Gamma(shape, 1) distribution.

    Parameters
    ----------
    shape : float
        Shape parameter (> 0).
    seed : int, optional
        RNG seed for deterministic sampling.

    Returns
    -------
    float
        Sample ≥ 0.
    """
    ...

def sample_student_t(df: float, seed: int | None = ...) -> float:
    """Sample from Student's t(ν) distribution.

    Parameters
    ----------
    df : float
        Degrees of freedom (> 0).
    seed : int, optional
        RNG seed for deterministic sampling.

    Returns
    -------
    float
        Sample (can be any real number).
    """
    ...

__all__ = [
    "binomial_probability",
    "binomial_distribution",
    "log_binomial_coefficient",
    "log_factorial",
    "sample_beta",
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
]
