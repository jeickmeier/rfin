"""Type stubs for correlation utility bindings."""

from __future__ import annotations

class CorrelatedBernoulli:
    """Correlated Bernoulli distribution for paired binary events.

    Precomputes joint probabilities from marginal probabilities and a
    correlation, enabling fast repeated sampling.

    Examples:
        >>> from finstack.correlation import CorrelatedBernoulli
        >>> dist = CorrelatedBernoulli(p1=0.1, p2=0.2, correlation=0.5)
        >>> x1, x2 = dist.sample_from_uniform(0.05)
    """

    def __init__(self, p1: float, p2: float, correlation: float) -> None: ...
    def sample_from_uniform(self, u: float) -> tuple[int, int]:
        """Sample a pair of correlated binary outcomes from a uniform draw."""
        ...
    def joint_probabilities(self) -> tuple[float, float, float, float]:
        """Joint probabilities (p11, p10, p01, p00)."""
        ...
    def __repr__(self) -> str: ...

def validate_correlation_matrix(matrix: list[float], n: int) -> None:
    """Validate a correlation matrix.

    Checks symmetry, unit diagonal, bounds, and positive semi-definiteness.

    Parameters
    ----------
    matrix : list[float]
        Flattened row-major correlation matrix.
    n : int
        Number of factors (matrix should be n×n).

    Raises
    ------
    ValidationError
        If the matrix is invalid.
    """
    ...

def cholesky_decompose(matrix: list[float], n: int) -> list[float]:
    """Cholesky decomposition of a correlation matrix.

    Uses diagonal pivoting to handle near-singular matrices.

    Parameters
    ----------
    matrix : list[float]
        Flattened row-major correlation matrix.
    n : int
        Matrix dimension.

    Returns
    -------
    list[float]
        Flattened lower-triangular Cholesky factor (row-major).

    Raises
    ------
    ValidationError
        If the matrix is not positive semi-definite.
    """
    ...

def correlation_bounds(p1: float, p2: float) -> tuple[float, float]:
    """Compute Fréchet-Hoeffding correlation bounds.

    Parameters
    ----------
    p1 : float
        Marginal probability P(X₁=1).
    p2 : float
        Marginal probability P(X₂=1).

    Returns
    -------
    tuple[float, float]
        (ρ_min, ρ_max) of achievable correlation.
    """
    ...

def joint_probabilities(
    p1: float, p2: float, correlation: float
) -> tuple[float, float, float, float]:
    """Compute joint probabilities for correlated Bernoulli events.

    Parameters
    ----------
    p1 : float
        Marginal probability P(X₁=1).
    p2 : float
        Marginal probability P(X₂=1).
    correlation : float
        Correlation between events.

    Returns
    -------
    tuple[float, float, float, float]
        (p11, p10, p01, p00) joint probabilities.
    """
    ...
