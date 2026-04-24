"""Monte Carlo GBM convenience bindings (``finstack-monte-carlo``).

Exposes simulation primitives: time grids, engine configuration, pricers,
and closed-form Black-Scholes helpers for the GBM-oriented binding surface.
Advanced Rust process, discretization, RNG, payoff, and Greeks types are not
surfaced as standalone Python types yet; their parameters are passed directly
as numeric arguments to the exposed pricer constructors and methods.
"""

from __future__ import annotations

from collections.abc import Sequence

from finstack.core.money import Money

__all__ = [
    "MonteCarloResult",
    "Estimate",
    "TimeGrid",
    "McEngine",
    "EuropeanPricer",
    "PathDependentPricer",
    "LsmcPricer",
    "black_scholes_call",
    "black_scholes_put",
    "price_european_call",
    "price_european_put",
]

class MonteCarloResult:
    """Discounted Monte Carlo estimate with money units and confidence bands.

    Args:
        None

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import price_european_call
        >>> r = price_european_call(100, 100, 0.05, 0.0, 0.2, 1.0, num_paths=10_000)
        >>> r.num_paths
        10000
    """

    @property
    def mean(self) -> Money:
        """Discounted mean present value.

        Args:
            None

        Returns:
            Mean PV as tagged money.

        Example:
            >>> from finstack.monte_carlo import price_european_call
            >>> price_european_call(100, 100, 0.05, 0.0, 0.2, 1.0, num_paths=1000).mean.amount > 0
            True
        """
        ...

    @property
    def stderr(self) -> float:
        """Standard error of the discounted mean.

        Args:
            None

        Returns:
            Standard error (same currency units as mean amount).

        Example:
            >>> from finstack.monte_carlo import price_european_call
            >>> price_european_call(100, 100, 0.05, 0.0, 0.2, 1.0, num_paths=1000).stderr >= 0
            True
        """
        ...

    @property
    def std_dev(self) -> float | None:
        """Sample standard deviation of path discounted values, if available.

        Args:
            None

        Returns:
            Sample standard deviation, or None if not provided by the engine.

        Example:
            >>> from finstack.monte_carlo import price_european_call
            >>> isinstance(
            ...     price_european_call(100, 100, 0.05, 0.0, 0.2, 1.0, num_paths=500).std_dev, (float, type(None))
            ... )
            True
        """
        ...

    @property
    def ci_lower(self) -> Money:
        """Lower bound of the 95% confidence interval for the mean.

        Args:
            None

        Returns:
            Lower CI bound as money.

        Example:
            >>> from finstack.monte_carlo import price_european_call
            >>> r = price_european_call(100, 100, 0.05, 0.0, 0.2, 1.0, num_paths=2000)
            >>> r.ci_lower.amount <= r.mean.amount
            True
        """
        ...

    @property
    def ci_upper(self) -> Money:
        """Upper bound of the 95% confidence interval for the mean.

        Args:
            None

        Returns:
            Upper CI bound as money.

        Example:
            >>> from finstack.monte_carlo import price_european_call
            >>> r = price_european_call(100, 100, 0.05, 0.0, 0.2, 1.0, num_paths=2000)
            >>> r.ci_upper.amount >= r.mean.amount
            True
        """
        ...

    @property
    def num_paths(self) -> int:
        """Number of independent path estimators contributing to the result.

        Equals the configured ``num_paths`` when antithetic variates are off,
        or half the number of simulated paths when antithetic pairing is on.

        Args:
            None

        Returns:
            Path-estimator count.

        Example:
            >>> from finstack.monte_carlo import price_european_call
            >>> price_european_call(100, 100, 0.05, 0.0, 0.2, 1.0, num_paths=1234).num_paths
            1234
        """
        ...

    @property
    def num_simulated_paths(self) -> int:
        """Total number of simulated sample paths driving the estimator.

        Equals :attr:`num_paths` without variance reduction, or
        ``2 * num_paths`` when antithetic variates are enabled.

        Returns:
            Count of simulated sample paths.
        """
        ...

    @property
    def num_skipped(self) -> int:
        """Legacy skipped-path count.

        Current engine loops reject non-finite discounted payoffs rather than
        censoring paths, so new results should report zero here.

        Args:
            None

        Returns:
            Count of skipped paths (0 when no values were filtered).

        Example:
            >>> from finstack.monte_carlo import price_european_call
            >>> price_european_call(100, 100, 0.05, 0.0, 0.2, 1.0, num_paths=1000).num_skipped
            0
        """
        ...

    @property
    def median(self) -> float | None:
        """Median of captured discounted path values, if captured.

        Args:
            None

        Returns:
            Median value, or None when percentile capture is disabled.
        """
        ...

    @property
    def percentile_25(self) -> float | None:
        """25th percentile of captured discounted path values, if captured.

        Args:
            None

        Returns:
            25th percentile value, or None when percentile capture is disabled.
        """
        ...

    @property
    def percentile_75(self) -> float | None:
        """75th percentile of captured discounted path values, if captured.

        Args:
            None

        Returns:
            75th percentile value, or None when percentile capture is disabled.
        """
        ...

    @property
    def min(self) -> float | None:
        """Minimum of captured discounted path values, if captured.

        Args:
            None

        Returns:
            Minimum sampled value, or None when range capture is disabled.
        """
        ...

    @property
    def max(self) -> float | None:
        """Maximum of captured discounted path values, if captured.

        Args:
            None

        Returns:
            Maximum sampled value, or None when range capture is disabled.
        """
        ...

    def relative_stderr(self) -> float:
        """Relative standard error (stderr divided by absolute mean amount).

        Args:
            None

        Returns:
            Dimensionless relative stderr.

        Example:
            >>> from finstack.monte_carlo import price_european_call
            >>> price_european_call(100, 100, 0.05, 0.0, 0.2, 1.0, num_paths=5000).relative_stderr() >= 0
            True
        """
        ...

class Estimate:
    """Scalar Monte Carlo estimate without currency tagging.

    Args:
        None

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import Estimate
        >>> hasattr(Estimate, "mean")
        True
    """

    @property
    def mean(self) -> float:
        """Point estimate (mean).

        Args:
            None

        Returns:
            Mean sample value.

        Example:
            >>> from finstack.monte_carlo import Estimate
            >>> Estimate.__dict__.get("mean") is not None
            True
        """
        ...

    @property
    def stderr(self) -> float:
        """Standard error of the mean.

        Args:
            None

        Returns:
            Standard error.

        Example:
            >>> from finstack.monte_carlo import Estimate
            >>> Estimate.__dict__.get("stderr") is not None
            True
        """
        ...

    @property
    def std_dev(self) -> float | None:
        """Sample standard deviation, if available.

        Args:
            None

        Returns:
            Sample standard deviation or None.

        Example:
            >>> from finstack.monte_carlo import Estimate
            >>> Estimate.__dict__.get("std_dev") is not None
            True
        """
        ...

    @property
    def ci_lower(self) -> float:
        """Lower 95% confidence bound.

        Args:
            None

        Returns:
            Lower bound.

        Example:
            >>> from finstack.monte_carlo import Estimate
            >>> Estimate.__dict__.get("ci_lower") is not None
            True
        """
        ...

    @property
    def ci_upper(self) -> float:
        """Upper 95% confidence bound.

        Args:
            None

        Returns:
            Upper bound.

        Example:
            >>> from finstack.monte_carlo import Estimate
            >>> Estimate.__dict__.get("ci_upper") is not None
            True
        """
        ...

    @property
    def num_paths(self) -> int:
        """Number of independent path estimators contributing to the estimate.

        Equals the configured ``num_paths`` when antithetic variates are off,
        or half the number of simulated paths when antithetic pairing is on.

        Args:
            None

        Returns:
            Path-estimator count.

        Example:
            >>> from finstack.monte_carlo import Estimate
            >>> Estimate.__dict__.get("num_paths") is not None
            True
        """
        ...

    @property
    def num_simulated_paths(self) -> int:
        """Total number of simulated sample paths driving the estimator.

        Equals :attr:`num_paths` without variance reduction, or
        ``2 * num_paths`` when antithetic variates are enabled.

        Returns:
            Count of simulated sample paths.
        """
        ...

    @property
    def num_skipped(self) -> int:
        """Legacy skipped-path count.

        Current engine loops reject non-finite discounted payoffs rather than
        censoring paths, so new estimates should report zero here.

        Args:
            None

        Returns:
            Count of skipped paths (0 when no values were filtered).
        """
        ...

    @property
    def median(self) -> float | None:
        """Median of captured path values, if captured.

        Args:
            None

        Returns:
            Median value, or None when percentile capture is disabled.
        """
        ...

    @property
    def percentile_25(self) -> float | None:
        """25th percentile of captured path values, if captured.

        Args:
            None

        Returns:
            25th percentile value, or None when percentile capture is disabled.
        """
        ...

    @property
    def percentile_75(self) -> float | None:
        """75th percentile of captured path values, if captured.

        Args:
            None

        Returns:
            75th percentile value, or None when percentile capture is disabled.
        """
        ...

    @property
    def min(self) -> float | None:
        """Minimum of captured path values, if captured.

        Args:
            None

        Returns:
            Minimum sampled value, or None when range capture is disabled.
        """
        ...

    @property
    def max(self) -> float | None:
        """Maximum of captured path values, if captured.

        Args:
            None

        Returns:
            Maximum sampled value, or None when range capture is disabled.
        """
        ...

class TimeGrid:
    """Discretised time axis for Monte Carlo stepping.

    Args:
        t_max: Terminal time in years.
        num_steps: Number of steps between 0 and ``t_max``.

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import TimeGrid
        >>> TimeGrid(1.0, 4).num_steps
        4
    """

    def __init__(self, t_max: float, num_steps: int) -> None:
        """Build a uniform grid from ``0`` to ``t_max`` with ``num_steps`` steps.

        Args:
            t_max: Terminal time.
            num_steps: Step count.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import TimeGrid
            >>> TimeGrid(0.5, 10).t_max
            0.5
        """
        ...

    @staticmethod
    def from_times(times: Sequence[float]) -> TimeGrid:
        """Construct a grid from explicit increasing time points.

        Args:
            times: Strictly usable time knot sequence (copied as ``list[float]`` internally).

        Returns:
            A ``TimeGrid`` instance.

        Example:
            >>> from finstack.monte_carlo import TimeGrid
            >>> TimeGrid.from_times([0.0, 0.25, 0.5, 1.0]).num_steps
            3
        """
        ...

    @property
    def num_steps(self) -> int:
        """Number of time steps on the grid.

        Args:
            None

        Returns:
            Step count.

        Example:
            >>> from finstack.monte_carlo import TimeGrid
            >>> TimeGrid(1.0, 100).num_steps
            100
        """
        ...

    @property
    def t_max(self) -> float:
        """Terminal time of the grid.

        Args:
            None

        Returns:
            Maximum time coordinate.

        Example:
            >>> from finstack.monte_carlo import TimeGrid
            >>> TimeGrid(2.0, 8).t_max
            2.0
        """
        ...

    @property
    def is_uniform(self) -> bool:
        """Whether step sizes are uniform.

        Args:
            None

        Returns:
            True if all inner steps share one ``dt``.

        Example:
            >>> from finstack.monte_carlo import TimeGrid
            >>> TimeGrid(1.0, 5).is_uniform
            True
        """
        ...

    @property
    def times(self) -> list[float]:
        """All time coordinates including the origin.

        Args:
            None

        Returns:
            Copy of knot times.

        Example:
            >>> from finstack.monte_carlo import TimeGrid
            >>> TimeGrid(1.0, 2).times[0]
            0.0
        """
        ...

    @property
    def dts(self) -> list[float]:
        """Step sizes between consecutive times.

        Args:
            None

        Returns:
            Per-step ``dt`` values.

        Example:
            >>> from finstack.monte_carlo import TimeGrid
            >>> len(TimeGrid(1.0, 4).dts)
            4
        """
        ...

    def time(self, step: int) -> float:
        """Time at a given step index.

        Args:
            step: Step index in ``[0, num_steps]``.

        Returns:
            Time coordinate.

        Example:
            >>> from finstack.monte_carlo import TimeGrid
            >>> TimeGrid(1.0, 4).time(0)
            0.0
        """
        ...

    def dt(self, step: int) -> float:
        """Step size following the given step index.

        Args:
            step: Step index in ``[0, num_steps - 1]``.

        Returns:
            Increment to the next time.

        Example:
            >>> from finstack.monte_carlo import TimeGrid
            >>> TimeGrid(1.0, 4).dt(0)
            0.25
        """
        ...

class McEngine:
    """Full Monte Carlo engine bound to a :class:`TimeGrid`.

    Args:
        num_paths: Number of paths.
        time_grid: Discretisation grid.
        seed: RNG seed (default ``42``).
        use_parallel: Enable parallel path generation (default ``False``).
        antithetic: Enable antithetic variates (default ``False``).

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import McEngine, TimeGrid
        >>> McEngine(100, TimeGrid(1.0, 50), seed=7).price_european_call(100, 100, 0.05, 0.0, 0.2).num_paths
        100
    """

    def __init__(
        self,
        num_paths: int,
        time_grid: TimeGrid,
        seed: int = 42,
        use_parallel: bool = False,
        antithetic: bool = False,
    ) -> None:
        """See class docstring for parameters.

        Args:
            num_paths: Path count.
            time_grid: Simulation grid.
            seed: Seed.
            use_parallel: Parallel flag.
            antithetic: Antithetic flag.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import McEngine, TimeGrid
            >>> McEngine(10, TimeGrid(1.0, 5), seed=1, use_parallel=True, antithetic=True)  # doctest: +ELLIPSIS
            McEngine(...)
        """
        ...

    def price_european_call(
        self,
        spot: float,
        strike: float,
        rate: float,
        div_yield: float,
        vol: float,
        currency: str | None = None,
    ) -> MonteCarloResult:
        """Price a European call on the engine's grid under GBM.

        Args:
            spot: Initial spot.
            strike: Strike.
            rate: Risk-free rate.
            div_yield: Dividend yield.
            vol: Volatility.
            currency: Currency string or None for USD default.

        Returns:
            Priced result.

        Example:
            >>> from finstack.monte_carlo import McEngine, TimeGrid
            >>> McEngine(500, TimeGrid(1.0, 52)).price_european_call(100, 100, 0.05, 0.0, 0.25).num_paths
            500
        """
        ...

    def price_european_put(
        self,
        spot: float,
        strike: float,
        rate: float,
        div_yield: float,
        vol: float,
        currency: str | None = None,
    ) -> MonteCarloResult:
        """Price a European put on the engine's grid under GBM.

        Args:
            spot: Initial spot.
            strike: Strike.
            rate: Risk-free rate.
            div_yield: Dividend yield.
            vol: Volatility.
            currency: Currency string or None for USD default.

        Returns:
            Priced result.

        Example:
            >>> from finstack.monte_carlo import McEngine, TimeGrid
            >>> McEngine(500, TimeGrid(1.0, 52)).price_european_put(100, 100, 0.05, 0.0, 0.25).num_paths
            500
        """
        ...

class EuropeanPricer:
    """European-option Monte Carlo pricer under GBM (exact time-stepping).

    Args:
        num_paths: Paths (default ``100_000``).
        seed: RNG seed (default ``42``).
        use_parallel: Parallel accumulation flag (default ``False``).

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import EuropeanPricer
        >>> EuropeanPricer(num_paths=1000, seed=1).price_call(100, 100, 0.05, 0.0, 0.2, 1.0).num_paths
        1000
    """

    def __init__(
        self,
        num_paths: int = 100_000,
        seed: int = 42,
        use_parallel: bool = False,
    ) -> None:
        """See class docstring for parameters.

        Args:
            num_paths: Path count.
            seed: Seed.
            use_parallel: Parallel flag.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import EuropeanPricer
            >>> EuropeanPricer(500, 9).seed
            9
        """
        ...

    @property
    def num_paths(self) -> int:
        """Configured path count.

        Args:
            None

        Returns:
            Paths.

        Example:
            >>> from finstack.monte_carlo import EuropeanPricer
            >>> EuropeanPricer(1234).num_paths
            1234
        """
        ...

    @property
    def seed(self) -> int:
        """RNG seed.

        Args:
            None

        Returns:
            Seed.

        Example:
            >>> from finstack.monte_carlo import EuropeanPricer
            >>> EuropeanPricer(seed=55).seed
            55
        """
        ...

    @property
    def use_parallel(self) -> bool:
        """Whether path accumulation runs on the rayon pool.

        Returns:
            Parallel flag as passed to ``__init__``.
        """
        ...

    def price_call(
        self,
        spot: float,
        strike: float,
        rate: float,
        div_yield: float,
        vol: float,
        expiry: float,
        num_steps: int = 252,
        currency: str | None = None,
    ) -> MonteCarloResult:
        """Price a European call.

        Args:
            spot: Spot.
            strike: Strike.
            rate: Risk-free rate.
            div_yield: Dividend yield.
            vol: Volatility.
            expiry: Time to maturity in years.
            num_steps: Time steps (default ``252``).
            currency: ISO string or None for USD.

        Returns:
            Result object.

        Example:
            >>> from finstack.monte_carlo import EuropeanPricer
            >>> EuropeanPricer(800, 0).price_call(100, 100, 0.05, 0.0, 0.2, 1.0, num_steps=52).num_paths
            800
        """
        ...

    def price_put(
        self,
        spot: float,
        strike: float,
        rate: float,
        div_yield: float,
        vol: float,
        expiry: float,
        num_steps: int = 252,
        currency: str | None = None,
    ) -> MonteCarloResult:
        """Price a European put.

        Args:
            spot: Spot.
            strike: Strike.
            rate: Risk-free rate.
            div_yield: Dividend yield.
            vol: Volatility.
            expiry: Time to maturity in years.
            num_steps: Time steps (default ``252``).
            currency: ISO string or None for USD.

        Returns:
            Result object.

        Example:
            >>> from finstack.monte_carlo import EuropeanPricer
            >>> EuropeanPricer(800, 0).price_put(100, 100, 0.05, 0.0, 0.2, 1.0, num_steps=52).num_paths
            800
        """
        ...

class PathDependentPricer:
    """Path-dependent Monte Carlo pricer (Asian-style exotics on GBM).

    Args:
        num_paths: Paths (default ``100_000``).
        seed: RNG seed (default ``42``).
        use_parallel: Parallel accumulation flag (default ``False``).

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import PathDependentPricer
        >>> PathDependentPricer(600, 2).price_asian_call(100, 100, 0.05, 0.0, 0.2, 1.0).num_paths
        600
    """

    def __init__(
        self,
        num_paths: int = 100_000,
        seed: int = 42,
        use_parallel: bool = False,
    ) -> None:
        """See class docstring for parameters.

        Args:
            num_paths: Path count.
            seed: Seed.
            use_parallel: Parallel flag.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import PathDependentPricer
            >>> PathDependentPricer(100, 1, use_parallel=True).num_paths
            100
        """
        ...

    def price_asian_call(
        self,
        spot: float,
        strike: float,
        rate: float,
        div_yield: float,
        vol: float,
        expiry: float,
        num_steps: int = 252,
        currency: str | None = None,
    ) -> MonteCarloResult:
        """Price an arithmetic Asian call (fixings at every step).

        Args:
            spot: Spot.
            strike: Strike.
            rate: Risk-free rate.
            div_yield: Dividend yield.
            vol: Volatility.
            expiry: Maturity in years.
            num_steps: Steps (default ``252``).
            currency: ISO string or None for USD.

        Returns:
            Result object.

        Example:
            >>> from finstack.monte_carlo import PathDependentPricer
            >>> PathDependentPricer(400, 0).price_asian_call(100, 100, 0.05, 0.0, 0.2, 1.0, num_steps=12).num_paths
            400
        """
        ...

    def price_asian_put(
        self,
        spot: float,
        strike: float,
        rate: float,
        div_yield: float,
        vol: float,
        expiry: float,
        num_steps: int = 252,
        currency: str | None = None,
    ) -> MonteCarloResult:
        """Price an arithmetic Asian put (fixings at every step).

        Args:
            spot: Spot.
            strike: Strike.
            rate: Risk-free rate.
            div_yield: Dividend yield.
            vol: Volatility.
            expiry: Maturity in years.
            num_steps: Steps (default ``252``).
            currency: ISO string or None for USD.

        Returns:
            Result object.

        Example:
            >>> from finstack.monte_carlo import PathDependentPricer
            >>> PathDependentPricer(400, 0).price_asian_put(100, 100, 0.05, 0.0, 0.2, 1.0, num_steps=12).num_paths
            400
        """
        ...

    @property
    def num_paths(self) -> int:
        """Configured path count.

        Args:
            None

        Returns:
            Paths.

        Example:
            >>> from finstack.monte_carlo import PathDependentPricer
            >>> PathDependentPricer(777).num_paths
            777
        """
        ...

    @property
    def seed(self) -> int:
        """RNG seed.

        Args:
            None

        Returns:
            Seed.

        Example:
            >>> from finstack.monte_carlo import PathDependentPricer
            >>> PathDependentPricer(seed=44).seed
            44
        """
        ...

class LsmcPricer:
    """Longstaff–Schwartz Monte Carlo pricer for American options under GBM.

    Args:
        num_paths: Paths (default ``100_000``).
        seed: RNG seed (default ``42``).
        use_parallel: Parallel path generation flag (default ``False``).
        basis: Regression basis family. One of ``"laguerre"`` (default),
            ``"polynomial"``, or ``"normalized_polynomial"``. ``None`` is
            treated as ``"laguerre"``.
        basis_degree: Polynomial/Laguerre degree (default ``3``). Must be
            positive; for ``"laguerre"`` it must additionally be in ``[1, 4]``.

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import LsmcPricer
        >>> LsmcPricer(300, 0).price_american_put(100, 100, 0.05, 0.0, 0.3, 1.0, num_steps=10).num_paths
        300
    """

    def __init__(
        self,
        num_paths: int = 100_000,
        seed: int = 42,
        use_parallel: bool = False,
        basis: str | None = None,
        basis_degree: int = 3,
    ) -> None:
        """See class docstring for parameters.

        Args:
            num_paths: Path count.
            seed: Seed.
            use_parallel: Parallel flag.
            basis: Basis family name.
            basis_degree: Polynomial/Laguerre degree.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import LsmcPricer
            >>> LsmcPricer(50, 3).num_paths
            50
        """
        ...

    @property
    def num_paths(self) -> int:
        """Configured path count."""
        ...

    @property
    def seed(self) -> int:
        """RNG seed."""
        ...

    @property
    def use_parallel(self) -> bool:
        """Whether path generation runs on the rayon pool.

        Returns:
            Parallel flag as passed to ``__init__``.
        """
        ...

    @property
    def basis(self) -> str:
        """Regression basis family name.

        Returns:
            One of ``"laguerre"``, ``"polynomial"``, ``"normalized_polynomial"``.
        """
        ...

    @property
    def basis_degree(self) -> int:
        """Configured polynomial/Laguerre degree."""
        ...

    def price_american_put(
        self,
        spot: float,
        strike: float,
        rate: float,
        div_yield: float,
        vol: float,
        expiry: float,
        num_steps: int = 50,
        currency: str | None = None,
    ) -> MonteCarloResult:
        """Price an American put via LSMC.

        Args:
            spot: Spot.
            strike: Strike.
            rate: Risk-free rate.
            div_yield: Dividend yield.
            vol: Volatility.
            expiry: Maturity in years.
            num_steps: Exercise grid steps (default ``50``).
            currency: ISO string or None for USD.

        Returns:
            Result object.

        Example:
            >>> from finstack.monte_carlo import LsmcPricer
            >>> LsmcPricer(200, 0).price_american_put(100, 100, 0.05, 0.0, 0.25, 1.0, num_steps=8).num_paths
            200
        """
        ...

    def price_american_call(
        self,
        spot: float,
        strike: float,
        rate: float,
        div_yield: float,
        vol: float,
        expiry: float,
        num_steps: int = 50,
        currency: str | None = None,
    ) -> MonteCarloResult:
        """Price an American call via LSMC.

        Args:
            spot: Spot.
            strike: Strike.
            rate: Risk-free rate.
            div_yield: Dividend yield.
            vol: Volatility.
            expiry: Maturity in years.
            num_steps: Exercise grid steps (default ``50``).
            currency: ISO string or None for USD.

        Returns:
            Result object.

        Example:
            >>> from finstack.monte_carlo import LsmcPricer
            >>> LsmcPricer(200, 0).price_american_call(100, 100, 0.05, 0.0, 0.25, 1.0, num_steps=8).num_paths
            200
        """
        ...

def black_scholes_call(
    spot: float,
    strike: float,
    rate: float,
    div_yield: float,
    vol: float,
    expiry: float,
) -> float:
    """Black–Scholes European call price (undiscounted payoff convention in Rust).

    Args:
        spot: Spot.
        strike: Strike.
        rate: Risk-free rate.
        div_yield: Dividend yield.
        vol: Volatility.
        expiry: Time to maturity.

    Returns:
        Call price.

    Example:
        >>> from finstack.monte_carlo import black_scholes_call
        >>> black_scholes_call(100, 100, 0.05, 0.0, 0.2, 1.0) > 0
        True
    """
    ...

def black_scholes_put(
    spot: float,
    strike: float,
    rate: float,
    div_yield: float,
    vol: float,
    expiry: float,
) -> float:
    """Black–Scholes European put price.

    Args:
        spot: Spot.
        strike: Strike.
        rate: Risk-free rate.
        div_yield: Dividend yield.
        vol: Volatility.
        expiry: Time to maturity.

    Returns:
        Put price.

    Example:
        >>> from finstack.monte_carlo import black_scholes_put
        >>> black_scholes_put(100, 100, 0.05, 0.0, 0.2, 1.0) > 0
        True
    """
    ...

def price_european_call(
    spot: float,
    strike: float,
    rate: float,
    div_yield: float,
    vol: float,
    expiry: float,
    num_paths: int = 100_000,
    seed: int = 42,
    num_steps: int = 252,
    currency: str | None = None,
) -> MonteCarloResult:
    """Monte Carlo European call under GBM (standalone convenience).

    Args:
        spot: Spot.
        strike: Strike.
        rate: Risk-free rate.
        div_yield: Dividend yield.
        vol: Volatility.
        expiry: Maturity in years.
        num_paths: Paths (default ``100_000``).
        seed: Seed (default ``42``).
        num_steps: Steps (default ``252``).
        currency: ISO string or None for USD.

    Returns:
        Monte Carlo result.

    Example:
        >>> from finstack.monte_carlo import price_european_call
        >>> price_european_call(100, 100, 0.05, 0.0, 0.2, 1.0, num_paths=2000).num_paths
        2000
    """
    ...

def price_european_put(
    spot: float,
    strike: float,
    rate: float,
    div_yield: float,
    vol: float,
    expiry: float,
    num_paths: int = 100_000,
    seed: int = 42,
    num_steps: int = 252,
    currency: str | None = None,
) -> MonteCarloResult:
    """Monte Carlo European put under GBM (standalone convenience).

    Args:
        spot: Spot.
        strike: Strike.
        rate: Risk-free rate.
        div_yield: Dividend yield.
        vol: Volatility.
        expiry: Maturity in years.
        num_paths: Paths (default ``100_000``).
        seed: Seed (default ``42``).
        num_steps: Steps (default ``252``).
        currency: ISO string or None for USD.

    Returns:
        Monte Carlo result.

    Example:
        >>> from finstack.monte_carlo import price_european_put
        >>> price_european_put(100, 100, 0.05, 0.0, 0.2, 1.0, num_paths=2000).num_paths
        2000
    """
    ...
