"""Joint probability utilities for correlated events.

Provides functions and classes for computing joint probabilities of correlated
Bernoulli random variables, useful for credit modeling and scenario generation.
"""

from typing import Tuple

def joint_probabilities(p1: float, p2: float, correlation: float) -> Tuple[float, float, float, float]:
    """Compute joint probabilities for two correlated Bernoulli random variables.

    Given marginal probabilities p1 and p2 with correlation ρ, returns
    the four joint probabilities (p11, p10, p01, p00) where:

    - p11 = P(X₁=1, X₂=1)
    - p10 = P(X₁=1, X₂=0)
    - p01 = P(X₁=0, X₂=1)
    - p00 = P(X₁=0, X₂=0)

    The correlation is automatically clamped to the feasible Fréchet-Hoeffding bounds.

    Parameters
    ----------
    p1 : float
        Marginal probability P(X₁=1), clamped to [0, 1].
    p2 : float
        Marginal probability P(X₂=1), clamped to [0, 1].
    correlation : float
        Correlation between X₁ and X₂, clamped to feasible bounds.

    Returns
    -------
    tuple[float, float, float, float]
        (p11, p10, p01, p00) that sums to 1.0 and preserves marginals.

    Examples
    --------
    >>> p11, p10, p01, p00 = joint_probabilities(0.6, 0.4, 0.3)
    >>> round(p11 + p10 + p01 + p00, 10)
    1.0
    """
    ...

def correlation_bounds(p1: float, p2: float) -> Tuple[float, float]:
    """Compute the achievable correlation bounds for given marginal probabilities.

    Returns the Fréchet-Hoeffding bounds that constrain feasible correlation.

    Parameters
    ----------
    p1 : float
        Marginal probability P(X₁=1).
    p2 : float
        Marginal probability P(X₂=1).

    Returns
    -------
    tuple[float, float]
        (ρ_min, ρ_max) achievable correlation bounds.

    Examples
    --------
    >>> rho_min, rho_max = correlation_bounds(0.5, 0.5)
    >>> rho_min < 0 < rho_max
    True
    """
    ...

class CorrelatedBernoulli:
    """Correlated Bernoulli distribution for scenario generation.

    Provides methods for working with correlated binary outcomes,
    useful for tree-based pricing and credit modeling.

    Parameters
    ----------
    p1 : float
        Marginal probability of first event.
    p2 : float
        Marginal probability of second event.
    correlation : float
        Correlation between events (clamped to feasible bounds).

    Examples
    --------
    >>> dist = CorrelatedBernoulli(0.5, 0.5, 0.5)
    >>> dist.p1
    0.5
    >>> x1, x2 = dist.sample_from_uniform(0.1)
    """

    def __init__(self, p1: float, p2: float, correlation: float) -> None: ...
    @property
    def p1(self) -> float:
        """Marginal probability P(X₁=1)."""
        ...

    @property
    def p2(self) -> float:
        """Marginal probability P(X₂=1)."""
        ...

    @property
    def correlation(self) -> float:
        """Correlation between X₁ and X₂."""
        ...

    @property
    def joint_p11(self) -> float:
        """Joint probability P(X₁=1, X₂=1)."""
        ...

    @property
    def joint_p10(self) -> float:
        """Joint probability P(X₁=1, X₂=0)."""
        ...

    @property
    def joint_p01(self) -> float:
        """Joint probability P(X₁=0, X₂=1)."""
        ...

    @property
    def joint_p00(self) -> float:
        """Joint probability P(X₁=0, X₂=0)."""
        ...

    @property
    def conditional_p2_given_x1(self) -> float:
        """Conditional probability P(X₂=1 | X₁=1)."""
        ...

    @property
    def conditional_p1_given_x2(self) -> float:
        """Conditional probability P(X₁=1 | X₂=1)."""
        ...

    def joint_probabilities(self) -> Tuple[float, float, float, float]:
        """Get all four joint probabilities as a tuple.

        Returns
        -------
        tuple[float, float, float, float]
            (p11, p10, p01, p00)
        """
        ...

    def sample_from_uniform(self, u: float) -> Tuple[int, int]:
        """Sample a pair of correlated binary outcomes given a uniform random value.

        Parameters
        ----------
        u : float
            Uniform random value in [0, 1].

        Returns
        -------
        tuple[int, int]
            Pair (x1, x2) where each is 0 or 1.
        """
        ...

    def __repr__(self) -> str: ...

__all__ = [
    "joint_probabilities",
    "correlation_bounds",
    "CorrelatedBernoulli",
]
