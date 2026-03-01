"""LSMC (Longstaff-Schwartz Monte Carlo) pricer type stubs."""

from __future__ import annotations
from typing import List, Tuple
from finstack.core.money import Money

class AmericanPut:
    """American put option exercise payoff.

    Args:
        strike: Strike price for the put option.

    Examples:
        >>> put = AmericanPut(strike=100.0)
        >>> put.strike
        100.0
    """

    def __init__(self, strike: float) -> None: ...
    @property
    def strike(self) -> float:
        """Strike price."""
        ...

class AmericanCall:
    """American call option exercise payoff.

    Args:
        strike: Strike price for the call option.

    Examples:
        >>> call = AmericanCall(strike=100.0)
        >>> call.strike
        100.0
    """

    def __init__(self, strike: float) -> None: ...
    @property
    def strike(self) -> float:
        """Strike price."""
        ...

class PolynomialBasis:
    """Polynomial basis functions for LSMC regression.

    Creates a basis of {1, x, x², ..., x^degree} for regression in the
    Longstaff-Schwartz algorithm.

    Args:
        degree: Polynomial degree (must be positive).

    Examples:
        >>> basis = PolynomialBasis(degree=3)
        >>> basis.degree
        3
    """

    def __init__(self, degree: int) -> None: ...
    @property
    def degree(self) -> int:
        """Polynomial degree."""
        ...

    @property
    def num_basis(self) -> int:
        """Number of basis functions."""
        ...

class LaguerreBasis:
    """Laguerre polynomial basis functions for LSMC regression.

    Creates a basis using Laguerre polynomials normalized by strike,
    which provides better numerical stability for option pricing.

    Args:
        degree: Polynomial degree (must be 1-4).
        strike: Strike price for normalization (must be positive).

    Examples:
        >>> basis = LaguerreBasis(degree=3, strike=100.0)
        >>> basis.degree
        3
        >>> basis.strike
        100.0
    """

    def __init__(self, degree: int, strike: float) -> None: ...
    @property
    def degree(self) -> int:
        """Polynomial degree."""
        ...

    @property
    def strike(self) -> float:
        """Strike price for normalization."""
        ...

    @property
    def num_basis(self) -> int:
        """Number of basis functions."""
        ...

class LsmcConfig:
    """Configuration for LSMC (Longstaff-Schwartz Monte Carlo) pricer.

    Args:
        num_paths: Number of Monte Carlo paths to simulate.
        exercise_dates: List of step indices where exercise is allowed.
        seed: Random seed for reproducibility (default: 42).

    Examples:
        >>> config = LsmcConfig(num_paths=50000, exercise_dates=[25, 50, 75, 100], seed=42)
    """

    def __init__(
        self,
        num_paths: int,
        exercise_dates: List[int],
        seed: int = 42,
    ) -> None: ...
    @property
    def num_paths(self) -> int:
        """Number of Monte Carlo paths."""
        ...

    @property
    def seed(self) -> int:
        """Random seed."""
        ...

    @property
    def exercise_dates(self) -> List[int]:
        """Exercise date step indices."""
        ...

class LsmcResult:
    """LSMC result containing price estimate and statistics.

    Attributes:
        mean: Point estimate of the option price.
        stderr: Standard error of the estimate.
        ci_95: 95% confidence interval (lower, upper).
        num_paths: Number of paths used.
    """

    @property
    def mean(self) -> Money:
        """Point estimate of the option price."""
        ...

    @property
    def stderr(self) -> float:
        """Standard error of the estimate."""
        ...

    @property
    def ci_95(self) -> Tuple[Money, Money]:
        """95% confidence interval (lower, upper)."""
        ...

    @property
    def num_paths(self) -> int:
        """Number of paths used."""
        ...

    def relative_stderr(self) -> float:
        """Relative standard error (stderr / mean)."""
        ...

class LsmcPricer:
    """LSMC (Longstaff-Schwartz Monte Carlo) pricer for American/Bermudan options.

    Uses backward induction with least-squares regression to price options
    with early exercise features.

    Args:
        config: LSMC configuration with paths, exercise dates, and seed.

    Examples:
        >>> config = LsmcConfig(num_paths=50000, exercise_dates=[25, 50, 75, 100])
        >>> pricer = LsmcPricer(config)
        >>> put = AmericanPut(strike=100.0)
        >>> basis = LaguerreBasis(degree=3, strike=100.0)
        >>> result = pricer.price(
        ...     initial_spot=100.0,
        ...     r=0.05,
        ...     q=0.0,
        ...     sigma=0.20,
        ...     time_to_maturity=1.0,
        ...     num_steps=100,
        ...     exercise=put,
        ...     basis=basis,
        ...     currency="USD",
        ... )
    """

    def __init__(self, config: LsmcConfig) -> None: ...
    def price(
        self,
        initial_spot: float,
        r: float,
        q: float,
        sigma: float,
        time_to_maturity: float,
        num_steps: int,
        exercise: AmericanPut | AmericanCall,
        basis: PolynomialBasis | LaguerreBasis,
        currency: str,
    ) -> LsmcResult:
        """Price an American-style option using LSMC.

        Args:
            initial_spot: Initial spot price of the underlying.
            r: Risk-free interest rate (annual, decimal).
            q: Dividend/foreign rate (annual, decimal).
            sigma: Volatility (annual, decimal).
            time_to_maturity: Time to maturity in years.
            num_steps: Number of time steps for discretization.
            exercise: Exercise payoff (AmericanPut or AmericanCall).
            basis: Basis functions for regression (PolynomialBasis or LaguerreBasis).
            currency: Currency code (e.g., "USD").

        Returns:
            LsmcResult: Statistical estimate of the option value.

        Raises:
            ValueError: If parameters are invalid.
        """
        ...

__all__ = [
    "AmericanPut",
    "AmericanCall",
    "PolynomialBasis",
    "LaguerreBasis",
    "LsmcConfig",
    "LsmcResult",
    "LsmcPricer",
]
