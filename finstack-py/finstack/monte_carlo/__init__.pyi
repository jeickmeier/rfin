"""Monte Carlo pricing bindings (``finstack-monte-carlo``).

Exposes simulation primitives: time grids, engine configuration, stochastic
process parameters, discretisation schemes, payoffs, pricers, and closed-form
Black–Scholes helpers.
"""

from __future__ import annotations

from collections.abc import Sequence

from finstack.core.money import Money

__all__ = [
    "MonteCarloResult",
    "Estimate",
    "TimeGrid",
    "McEngineConfig",
    "McEngine",
    "GbmProcess",
    "MultiGbmProcess",
    "BrownianProcess",
    "HestonProcess",
    "CirProcess",
    "MertonJumpProcess",
    "BatesProcess",
    "SchwartzSmithProcess",
    "ExactGbm",
    "ExactMultiGbm",
    "EulerMaruyama",
    "LogEuler",
    "Milstein",
    "EuropeanCall",
    "EuropeanPut",
    "DigitalCall",
    "DigitalPut",
    "ForwardLong",
    "ForwardShort",
    "AsianCall",
    "AsianPut",
    "BarrierOption",
    "BasketCall",
    "BasketPut",
    "AmericanPut",
    "AmericanCall",
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
        """Number of simulated paths used to form the estimate.

        Args:
            None

        Returns:
            Path count.

        Example:
            >>> from finstack.monte_carlo import price_european_call
            >>> price_european_call(100, 100, 0.05, 0.0, 0.2, 1.0, num_paths=1234).num_paths
            1234
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
        """Number of paths or samples.

        Args:
            None

        Returns:
            Path count.

        Example:
            >>> from finstack.monte_carlo import Estimate
            >>> Estimate.__dict__.get("num_paths") is not None
            True
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

class McEngineConfig:
    """Lightweight config for bundled GBM European pricing helpers.

    Args:
        num_paths: Number of paths.
        seed: RNG seed.
        time_to_maturity: Horizon in years (default ``1.0``).
        num_steps: Steps for the internal uniform grid (default ``252``).

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import McEngineConfig
        >>> McEngineConfig(1000, 1).num_steps
        252
    """

    def __init__(
        self,
        num_paths: int,
        seed: int,
        time_to_maturity: float = 1.0,
        num_steps: int = 252,
    ) -> None:
        """See class docstring for parameters.

        Args:
            num_paths: Path count.
            seed: Seed.
            time_to_maturity: Terminal time.
            num_steps: Discretisation steps.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import McEngineConfig
            >>> McEngineConfig(10, 0, time_to_maturity=0.5).time_to_maturity
            0.5
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
            >>> from finstack.monte_carlo import McEngineConfig
            >>> McEngineConfig(777, 0).num_paths
            777
        """
        ...

    @property
    def seed(self) -> int:
        """RNG seed.

        Args:
            None

        Returns:
            Seed value.

        Example:
            >>> from finstack.monte_carlo import McEngineConfig
            >>> McEngineConfig(1, 99).seed
            99
        """
        ...

    @property
    def time_to_maturity(self) -> float:
        """Simulation horizon in years.

        Args:
            None

        Returns:
            Terminal time.

        Example:
            >>> from finstack.monte_carlo import McEngineConfig
            >>> McEngineConfig(1, 0, time_to_maturity=2.0).time_to_maturity
            2.0
        """
        ...

    @property
    def num_steps(self) -> int:
        """Internal uniform time-grid steps.

        Args:
            None

        Returns:
            Step count.

        Example:
            >>> from finstack.monte_carlo import McEngineConfig
            >>> McEngineConfig(1, 0, num_steps=100).num_steps
            100
        """
        ...

    def price_call(
        self,
        spot: float,
        strike: float,
        rate: float,
        div_yield: float,
        vol: float,
        currency: str,
    ) -> MonteCarloResult:
        """Price a European call under GBM using the bundled engine path.

        Args:
            spot: Spot at ``t=0``.
            strike: Strike.
            rate: Risk-free rate (continuous).
            div_yield: Dividend yield (continuous).
            vol: Volatility.
            currency: ISO code string (or value accepted by currency extract).

        Returns:
            Monte Carlo result.

        Example:
            >>> from finstack.monte_carlo import McEngineConfig
            >>> McEngineConfig(2000, 3).price_call(100, 100, 0.05, 0.0, 0.2, "USD").num_paths
            2000
        """
        ...

    def price_put(
        self,
        spot: float,
        strike: float,
        rate: float,
        div_yield: float,
        vol: float,
        currency: str,
    ) -> MonteCarloResult:
        """Price a European put under GBM using the bundled engine path.

        Args:
            spot: Spot at ``t=0``.
            strike: Strike.
            rate: Risk-free rate (continuous).
            div_yield: Dividend yield (continuous).
            vol: Volatility.
            currency: ISO code string (or value accepted by currency extract).

        Returns:
            Monte Carlo result.

        Example:
            >>> from finstack.monte_carlo import McEngineConfig
            >>> McEngineConfig(2000, 3).price_put(100, 100, 0.05, 0.0, 0.2, "USD").num_paths
            2000
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

class GbmProcess:
    """Geometric Brownian motion parameters (rate, dividend yield, volatility).

    Args:
        rate: Risk-free rate (continuous).
        div_yield: Dividend yield (continuous).
        vol: Volatility.

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import GbmProcess
        >>> GbmProcess(0.05, 0.01, 0.2).vol
        0.2
    """

    def __init__(self, rate: float, div_yield: float, vol: float) -> None:
        """Store GBM parameters for downstream pricers.

        Args:
            rate: Risk-free rate.
            div_yield: Dividend yield.
            vol: Volatility.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import GbmProcess
            >>> GbmProcess(0.0, 0.0, 0.15).rate
            0.0
        """
        ...

    @property
    def rate(self) -> float:
        """Risk-free rate.

        Args:
            None

        Returns:
            Rate.

        Example:
            >>> from finstack.monte_carlo import GbmProcess
            >>> GbmProcess(0.03, 0.0, 0.1).rate
            0.03
        """
        ...

    @property
    def div_yield(self) -> float:
        """Dividend yield.

        Args:
            None

        Returns:
            Yield.

        Example:
            >>> from finstack.monte_carlo import GbmProcess
            >>> GbmProcess(0.03, 0.02, 0.1).div_yield
            0.02
        """
        ...

    @property
    def vol(self) -> float:
        """Volatility.

        Args:
            None

        Returns:
            Vol.

        Example:
            >>> from finstack.monte_carlo import GbmProcess
            >>> GbmProcess(0.03, 0.02, 0.1).vol
            0.1
        """
        ...

class MultiGbmProcess:
    """Correlated multi-asset GBM parameters.

    Args:
        rates: Per-asset risk-free rates.
        div_yields: Per-asset dividend yields.
        vols: Per-asset volatilities.
        correlation: Row-major ``n*n`` correlation matrix entries.

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import MultiGbmProcess
        >>> MultiGbmProcess([0.05, 0.05], [0.0, 0.0], [0.2, 0.3], [1, 0, 0, 1]).num_assets
        2
    """

    def __init__(
        self,
        rates: Sequence[float],
        div_yields: Sequence[float],
        vols: Sequence[float],
        correlation: Sequence[float],
    ) -> None:
        """See class docstring for parameters.

        Args:
            rates: Rate vector.
            div_yields: Dividend vector.
            vols: Vol vector.
            correlation: Flat correlation matrix.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import MultiGbmProcess
            >>> MultiGbmProcess([0.01], [0.0], [0.1], [1.0]).num_assets
            1
        """
        ...

    @property
    def num_assets(self) -> int:
        """Number of assets (length of rate vector).

        Args:
            None

        Returns:
            Asset count.

        Example:
            >>> from finstack.monte_carlo import MultiGbmProcess
            >>> MultiGbmProcess([0, 0, 0], [0, 0, 0], [0.1, 0.1, 0.1], [1] * 9).num_assets
            3
        """
        ...

class BrownianProcess:
    """Arithmetic Brownian motion parameters.

    Args:
        mu: Drift.
        sigma: Diffusion coefficient.

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import BrownianProcess
        >>> BrownianProcess(0.01, 0.2).sigma
        0.2
    """

    def __init__(self, mu: float, sigma: float) -> None:
        """Store ABM parameters.

        Args:
            mu: Drift.
            sigma: Vol parameter.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import BrownianProcess
            >>> BrownianProcess(0.0, 0.05).mu
            0.0
        """
        ...

    @property
    def mu(self) -> float:
        """Drift.

        Args:
            None

        Returns:
            Drift.

        Example:
            >>> from finstack.monte_carlo import BrownianProcess
            >>> BrownianProcess(-0.01, 0.1).mu
            -0.01
        """
        ...

    @property
    def sigma(self) -> float:
        """Diffusion scale.

        Args:
            None

        Returns:
            Sigma.

        Example:
            >>> from finstack.monte_carlo import BrownianProcess
            >>> BrownianProcess(0.0, 0.3).sigma
            0.3
        """
        ...

class HestonProcess:
    """Heston stochastic volatility parameters.

    Args:
        rate: Risk-free rate.
        div_yield: Dividend yield.
        v0: Initial variance.
        kappa: Mean-reversion speed.
        theta: Long-run variance.
        xi: Vol-of-vol.
        rho: Spot–variance correlation.

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import HestonProcess
        >>> HestonProcess(0.05, 0.0, 0.04, 2.0, 0.04, 0.3, -0.7).kappa
        2.0
    """

    def __init__(
        self,
        rate: float,
        div_yield: float,
        v0: float,
        kappa: float,
        theta: float,
        xi: float,
        rho: float,
    ) -> None:
        """See class docstring for parameters.

        Args:
            rate: Risk-free rate.
            div_yield: Dividend yield.
            v0: Initial variance.
            kappa: Mean reversion.
            theta: Long-run variance.
            xi: Vol-of-vol.
            rho: Correlation.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import HestonProcess
            >>> HestonProcess(0.0, 0.0, 0.01, 1.0, 0.01, 0.2, 0.0).v0
            0.01
        """
        ...

    @property
    def satisfies_feller(self) -> bool:
        """Whether ``2*kappa*theta > xi**2`` (Feller condition).

        Args:
            None

        Returns:
            True if Feller holds.

        Example:
            >>> from finstack.monte_carlo import HestonProcess
            >>> isinstance(HestonProcess(0.05, 0.0, 0.04, 1.0, 0.04, 0.5, 0.0).satisfies_feller, bool)
            True
        """
        ...

    @property
    def rate(self) -> float:
        """Risk-free rate.

        Args:
            None

        Returns:
            Rate.

        Example:
            >>> from finstack.monte_carlo import HestonProcess
            >>> HestonProcess(0.04, 0.0, 0.04, 1.0, 0.04, 0.3, 0.0).rate
            0.04
        """
        ...

    @property
    def div_yield(self) -> float:
        """Dividend yield.

        Args:
            None

        Returns:
            Yield.

        Example:
            >>> from finstack.monte_carlo import HestonProcess
            >>> HestonProcess(0.04, 0.01, 0.04, 1.0, 0.04, 0.3, 0.0).div_yield
            0.01
        """
        ...

    @property
    def v0(self) -> float:
        """Initial variance.

        Args:
            None

        Returns:
            Variance.

        Example:
            >>> from finstack.monte_carlo import HestonProcess
            >>> HestonProcess(0.04, 0.0, 0.05, 1.0, 0.04, 0.3, 0.0).v0
            0.05
        """
        ...

    @property
    def kappa(self) -> float:
        """Mean reversion speed.

        Args:
            None

        Returns:
            Kappa.

        Example:
            >>> from finstack.monte_carlo import HestonProcess
            >>> HestonProcess(0.04, 0.0, 0.04, 1.5, 0.04, 0.3, 0.0).kappa
            1.5
        """
        ...

    @property
    def theta(self) -> float:
        """Long-run variance.

        Args:
            None

        Returns:
            Theta.

        Example:
            >>> from finstack.monte_carlo import HestonProcess
            >>> HestonProcess(0.04, 0.0, 0.04, 1.0, 0.05, 0.3, 0.0).theta
            0.05
        """
        ...

    @property
    def xi(self) -> float:
        """Vol-of-vol.

        Args:
            None

        Returns:
            Xi.

        Example:
            >>> from finstack.monte_carlo import HestonProcess
            >>> HestonProcess(0.04, 0.0, 0.04, 1.0, 0.04, 0.35, 0.0).xi
            0.35
        """
        ...

    @property
    def rho(self) -> float:
        """Spot–variance correlation.

        Args:
            None

        Returns:
            Rho.

        Example:
            >>> from finstack.monte_carlo import HestonProcess
            >>> HestonProcess(0.04, 0.0, 0.04, 1.0, 0.04, 0.3, -0.5).rho
            -0.5
        """
        ...

class CirProcess:
    """Cox–Ingersoll–Ross process parameters.

    Args:
        kappa: Mean reversion.
        theta: Long-run level.
        sigma: Volatility of the process.
        x0: Initial value.

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import CirProcess
        >>> CirProcess(0.5, 0.03, 0.05, 0.03).x0
        0.03
    """

    def __init__(self, kappa: float, theta: float, sigma: float, x0: float) -> None:
        """See class docstring for parameters.

        Args:
            kappa: Mean reversion speed.
            theta: Long-run level.
            sigma: Volatility.
            x0: Initial state.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import CirProcess
            >>> CirProcess(1.0, 0.02, 0.01, 0.02).theta
            0.02
        """
        ...

    @property
    def satisfies_feller(self) -> bool:
        """Whether ``2*kappa*theta > sigma**2``.

        Args:
            None

        Returns:
            True if Feller holds.

        Example:
            >>> from finstack.monte_carlo import CirProcess
            >>> isinstance(CirProcess(1.0, 0.04, 0.1, 0.04).satisfies_feller, bool)
            True
        """
        ...

    @property
    def kappa(self) -> float:
        """Mean reversion speed.

        Args:
            None

        Returns:
            Kappa.

        Example:
            >>> from finstack.monte_carlo import CirProcess
            >>> CirProcess(0.8, 0.03, 0.05, 0.03).kappa
            0.8
        """
        ...

    @property
    def theta(self) -> float:
        """Long-run level.

        Args:
            None

        Returns:
            Theta.

        Example:
            >>> from finstack.monte_carlo import CirProcess
            >>> CirProcess(0.8, 0.04, 0.05, 0.03).theta
            0.04
        """
        ...

    @property
    def sigma(self) -> float:
        """Volatility parameter.

        Args:
            None

        Returns:
            Sigma.

        Example:
            >>> from finstack.monte_carlo import CirProcess
            >>> CirProcess(0.8, 0.03, 0.06, 0.03).sigma
            0.06
        """
        ...

    @property
    def x0(self) -> float:
        """Initial value.

        Args:
            None

        Returns:
            Initial ``x``.

        Example:
            >>> from finstack.monte_carlo import CirProcess
            >>> CirProcess(0.8, 0.03, 0.05, 0.02).x0
            0.02
        """
        ...

class MertonJumpProcess:
    """Merton jump-diffusion parameters.

    Args:
        rate: Risk-free rate.
        div_yield: Dividend yield.
        sigma: Diffusion volatility.
        jump_intensity: Jump arrival intensity.
        jump_mean: Mean jump size (log-space convention in Rust core).
        jump_vol: Jump size volatility.

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import MertonJumpProcess
        >>> MertonJumpProcess(0.05, 0.0, 0.2, 1.0, -0.05, 0.1).jump_intensity
        1.0
    """

    def __init__(
        self,
        rate: float,
        div_yield: float,
        sigma: float,
        jump_intensity: float,
        jump_mean: float,
        jump_vol: float,
    ) -> None:
        """See class docstring for parameters.

        Args:
            rate: Risk-free rate.
            div_yield: Dividend yield.
            sigma: Diffusion vol.
            jump_intensity: Jump intensity.
            jump_mean: Jump mean.
            jump_vol: Jump vol.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import MertonJumpProcess
            >>> MertonJumpProcess(0.0, 0.0, 0.15, 0.5, 0.0, 0.05).sigma
            0.15
        """
        ...

    @property
    def rate(self) -> float:
        """Risk-free rate.

        Args:
            None

        Returns:
            Rate.

        Example:
            >>> from finstack.monte_carlo import MertonJumpProcess
            >>> MertonJumpProcess(0.03, 0.0, 0.2, 1.0, 0.0, 0.1).rate
            0.03
        """
        ...

    @property
    def sigma(self) -> float:
        """Diffusion volatility.

        Args:
            None

        Returns:
            Sigma.

        Example:
            >>> from finstack.monte_carlo import MertonJumpProcess
            >>> MertonJumpProcess(0.03, 0.0, 0.25, 1.0, 0.0, 0.1).sigma
            0.25
        """
        ...

    @property
    def jump_intensity(self) -> float:
        """Jump intensity.

        Args:
            None

        Returns:
            Lambda.

        Example:
            >>> from finstack.monte_carlo import MertonJumpProcess
            >>> MertonJumpProcess(0.03, 0.0, 0.2, 2.0, 0.0, 0.1).jump_intensity
            2.0
        """
        ...

    @property
    def jump_mean(self) -> float:
        """Jump mean parameter.

        Args:
            None

        Returns:
            Mean jump.

        Example:
            >>> from finstack.monte_carlo import MertonJumpProcess
            >>> MertonJumpProcess(0.03, 0.0, 0.2, 1.0, -0.02, 0.1).jump_mean
            -0.02
        """
        ...

    @property
    def jump_vol(self) -> float:
        """Jump volatility parameter.

        Args:
            None

        Returns:
            Jump vol.

        Example:
            >>> from finstack.monte_carlo import MertonJumpProcess
            >>> MertonJumpProcess(0.03, 0.0, 0.2, 1.0, 0.0, 0.08).jump_vol
            0.08
        """
        ...

class BatesProcess:
    """Bates model parameters (Heston with jumps).

    Args:
        rate: Risk-free rate.
        div_yield: Dividend yield.
        v0: Initial variance.
        kappa: Mean reversion.
        theta: Long-run variance.
        xi: Vol-of-vol.
        rho: Correlation.
        jump_intensity: Jump intensity.
        jump_mean: Jump mean.
        jump_vol: Jump volatility.

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import BatesProcess
        >>> BatesProcess(0.05, 0.0, 0.04, 1.0, 0.04, 0.3, -0.7, 0.5, 0.0, 0.1).v0
        0.04
    """

    def __init__(
        self,
        rate: float,
        div_yield: float,
        v0: float,
        kappa: float,
        theta: float,
        xi: float,
        rho: float,
        jump_intensity: float,
        jump_mean: float,
        jump_vol: float,
    ) -> None:
        """See class docstring for parameters.

        Args:
            rate: Risk-free rate.
            div_yield: Dividend yield.
            v0: Initial variance.
            kappa: Mean reversion.
            theta: Long-run variance.
            xi: Vol-of-vol.
            rho: Correlation.
            jump_intensity: Jump intensity.
            jump_mean: Jump mean.
            jump_vol: Jump vol.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import BatesProcess
            >>> BatesProcess(0.0, 0.0, 0.01, 1.0, 0.01, 0.2, 0.0, 0.1, 0.0, 0.05).kappa
            1.0
        """
        ...

    @property
    def v0(self) -> float:
        """Initial variance.

        Args:
            None

        Returns:
            Variance.

        Example:
            >>> from finstack.monte_carlo import BatesProcess
            >>> BatesProcess(0.05, 0.0, 0.05, 1.0, 0.04, 0.3, 0.0, 0.1, 0.0, 0.05).v0
            0.05
        """
        ...

    @property
    def kappa(self) -> float:
        """Mean reversion speed.

        Args:
            None

        Returns:
            Kappa.

        Example:
            >>> from finstack.monte_carlo import BatesProcess
            >>> BatesProcess(0.05, 0.0, 0.04, 1.2, 0.04, 0.3, 0.0, 0.1, 0.0, 0.05).kappa
            1.2
        """
        ...

    @property
    def theta(self) -> float:
        """Long-run variance.

        Args:
            None

        Returns:
            Theta.

        Example:
            >>> from finstack.monte_carlo import BatesProcess
            >>> BatesProcess(0.05, 0.0, 0.04, 1.0, 0.05, 0.3, 0.0, 0.1, 0.0, 0.05).theta
            0.05
        """
        ...

    @property
    def xi(self) -> float:
        """Vol-of-vol.

        Args:
            None

        Returns:
            Xi.

        Example:
            >>> from finstack.monte_carlo import BatesProcess
            >>> BatesProcess(0.05, 0.0, 0.04, 1.0, 0.04, 0.25, 0.0, 0.1, 0.0, 0.05).xi
            0.25
        """
        ...

    @property
    def rho(self) -> float:
        """Spot–variance correlation.

        Args:
            None

        Returns:
            Rho.

        Example:
            >>> from finstack.monte_carlo import BatesProcess
            >>> BatesProcess(0.05, 0.0, 0.04, 1.0, 0.04, 0.3, -0.6, 0.1, 0.0, 0.05).rho
            -0.6
        """
        ...

    @property
    def jump_intensity(self) -> float:
        """Jump intensity.

        Args:
            None

        Returns:
            Intensity.

        Example:
            >>> from finstack.monte_carlo import BatesProcess
            >>> BatesProcess(0.05, 0.0, 0.04, 1.0, 0.04, 0.3, 0.0, 0.7, 0.0, 0.05).jump_intensity
            0.7
        """
        ...

class SchwartzSmithProcess:
    """Schwartz–Smith two-factor commodity model parameters.

    Args:
        kappa: Mean reversion of short factor.
        sigma_chi: Short-factor volatility.
        sigma_xi: Long-factor volatility.
        rho: Correlation between factors.
        mu_xi: Drift of long factor.
        lambda_chi: Risk premium on short factor.

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import SchwartzSmithProcess
        >>> SchwartzSmithProcess(1.0, 0.2, 0.1, 0.3, 0.05, 0.02).kappa
        1.0
    """

    def __init__(
        self,
        kappa: float,
        sigma_chi: float,
        sigma_xi: float,
        rho: float,
        mu_xi: float,
        lambda_chi: float,
    ) -> None:
        """See class docstring for parameters.

        Args:
            kappa: Mean reversion.
            sigma_chi: Short vol.
            sigma_xi: Long vol.
            rho: Correlation.
            mu_xi: Long drift.
            lambda_chi: Risk premium.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import SchwartzSmithProcess
            >>> SchwartzSmithProcess(0.5, 0.15, 0.08, 0.0, 0.03, 0.01).sigma_chi
            0.15
        """
        ...

    @property
    def kappa(self) -> float:
        """Mean reversion of the short factor.

        Args:
            None

        Returns:
            Kappa.

        Example:
            >>> from finstack.monte_carlo import SchwartzSmithProcess
            >>> SchwartzSmithProcess(0.9, 0.2, 0.1, 0.0, 0.03, 0.01).kappa
            0.9
        """
        ...

    @property
    def sigma_chi(self) -> float:
        """Short-factor volatility.

        Args:
            None

        Returns:
            Sigma chi.

        Example:
            >>> from finstack.monte_carlo import SchwartzSmithProcess
            >>> SchwartzSmithProcess(1.0, 0.22, 0.1, 0.0, 0.03, 0.01).sigma_chi
            0.22
        """
        ...

    @property
    def sigma_xi(self) -> float:
        """Long-factor volatility.

        Args:
            None

        Returns:
            Sigma xi.

        Example:
            >>> from finstack.monte_carlo import SchwartzSmithProcess
            >>> SchwartzSmithProcess(1.0, 0.2, 0.12, 0.0, 0.03, 0.01).sigma_xi
            0.12
        """
        ...

    @property
    def rho(self) -> float:
        """Factor correlation.

        Args:
            None

        Returns:
            Rho.

        Example:
            >>> from finstack.monte_carlo import SchwartzSmithProcess
            >>> SchwartzSmithProcess(1.0, 0.2, 0.1, 0.25, 0.03, 0.01).rho
            0.25
        """
        ...

class ExactGbm:
    """Exact single-asset GBM discretisation scheme handle.

    Args:
        None

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import ExactGbm
        >>> ExactGbm()  # doctest: +ELLIPSIS
        ExactGbm()
    """

    def __init__(self) -> None:
        """Create the scheme marker.

        Args:
            None

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import ExactGbm
            >>> isinstance(ExactGbm(), ExactGbm)
            True
        """
        ...

class ExactMultiGbm:
    """Exact multi-asset GBM discretisation scheme handle.

    Args:
        None

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import ExactMultiGbm
        >>> ExactMultiGbm()  # doctest: +ELLIPSIS
        ExactMultiGbm()
    """

    def __init__(self) -> None:
        """Create the scheme marker.

        Args:
            None

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import ExactMultiGbm
            >>> isinstance(ExactMultiGbm(), ExactMultiGbm)
            True
        """
        ...

class EulerMaruyama:
    """Euler–Maruyama discretisation scheme handle.

    Args:
        None

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import EulerMaruyama
        >>> EulerMaruyama()  # doctest: +ELLIPSIS
        EulerMaruyama()
    """

    def __init__(self) -> None:
        """Create the scheme marker.

        Args:
            None

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import EulerMaruyama
            >>> isinstance(EulerMaruyama(), EulerMaruyama)
            True
        """
        ...

class LogEuler:
    """Log-Euler (log-space Euler) discretisation scheme handle.

    Args:
        None

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import LogEuler
        >>> LogEuler()  # doctest: +ELLIPSIS
        LogEuler()
    """

    def __init__(self) -> None:
        """Create the scheme marker.

        Args:
            None

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import LogEuler
            >>> isinstance(LogEuler(), LogEuler)
            True
        """
        ...

class Milstein:
    """Milstein discretisation scheme handle.

    Args:
        None

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import Milstein
        >>> Milstein()  # doctest: +ELLIPSIS
        Milstein()
    """

    def __init__(self) -> None:
        """Create the scheme marker.

        Args:
            None

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import Milstein
            >>> isinstance(Milstein(), Milstein)
            True
        """
        ...

class EuropeanCall:
    """European call payoff parameters.

    Args:
        strike: Strike price.
        notional: Contract notional multiplier (default ``1.0``).
        maturity_step: Observation step index (default ``252``).

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import EuropeanCall
        >>> EuropeanCall(100.0).strike
        100.0
    """

    def __init__(
        self,
        strike: float,
        notional: float = 1.0,
        maturity_step: int = 252,
    ) -> None:
        """See class docstring for parameters.

        Args:
            strike: Strike.
            notional: Notional.
            maturity_step: Maturity step.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import EuropeanCall
            >>> EuropeanCall(90.0, notional=2.0, maturity_step=100).maturity_step
            100
        """
        ...

    @property
    def strike(self) -> float:
        """Strike price.

        Args:
            None

        Returns:
            Strike.

        Example:
            >>> from finstack.monte_carlo import EuropeanCall
            >>> EuropeanCall(101.0).strike
            101.0
        """
        ...

    @property
    def notional(self) -> float:
        """Notional multiplier.

        Args:
            None

        Returns:
            Notional.

        Example:
            >>> from finstack.monte_carlo import EuropeanCall
            >>> EuropeanCall(100.0, notional=3.0).notional
            3.0
        """
        ...

    @property
    def maturity_step(self) -> int:
        """Maturity step index on the path.

        Args:
            None

        Returns:
            Step.

        Example:
            >>> from finstack.monte_carlo import EuropeanCall
            >>> EuropeanCall(100.0, maturity_step=10).maturity_step
            10
        """
        ...

class EuropeanPut:
    """European put payoff parameters.

    Args:
        strike: Strike price.
        notional: Notional multiplier (default ``1.0``).
        maturity_step: Observation step (default ``252``).

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import EuropeanPut
        >>> EuropeanPut(100.0).strike
        100.0
    """

    def __init__(
        self,
        strike: float,
        notional: float = 1.0,
        maturity_step: int = 252,
    ) -> None:
        """See class docstring for parameters.

        Args:
            strike: Strike.
            notional: Notional.
            maturity_step: Maturity step.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import EuropeanPut
            >>> EuropeanPut(90.0, notional=2.0, maturity_step=100).maturity_step
            100
        """
        ...

    @property
    def strike(self) -> float:
        """Strike price.

        Args:
            None

        Returns:
            Strike.

        Example:
            >>> from finstack.monte_carlo import EuropeanPut
            >>> EuropeanPut(99.0).strike
            99.0
        """
        ...

    @property
    def notional(self) -> float:
        """Notional multiplier.

        Args:
            None

        Returns:
            Notional.

        Example:
            >>> from finstack.monte_carlo import EuropeanPut
            >>> EuropeanPut(100.0, notional=4.0).notional
            4.0
        """
        ...

    @property
    def maturity_step(self) -> int:
        """Maturity step index.

        Args:
            None

        Returns:
            Step.

        Example:
            >>> from finstack.monte_carlo import EuropeanPut
            >>> EuropeanPut(100.0, maturity_step=20).maturity_step
            20
        """
        ...

class DigitalCall:
    """Digital call payoff parameters (pays notional if ``S > K`` at maturity).

    Args:
        strike: Strike.
        notional: Notional (default ``1.0``).
        maturity_step: Observation step (default ``252``).

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import DigitalCall
        >>> DigitalCall(100.0).strike
        100.0
    """

    def __init__(
        self,
        strike: float,
        notional: float = 1.0,
        maturity_step: int = 252,
    ) -> None:
        """See class docstring for parameters.

        Args:
            strike: Strike.
            notional: Notional.
            maturity_step: Maturity step.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import DigitalCall
            >>> DigitalCall(95.0, maturity_step=50).strike
            95.0
        """
        ...

    @property
    def strike(self) -> float:
        """Strike price.

        Args:
            None

        Returns:
            Strike.

        Example:
            >>> from finstack.monte_carlo import DigitalCall
            >>> DigitalCall(102.0).strike
            102.0
        """
        ...

class DigitalPut:
    """Digital put payoff parameters (pays notional if ``S < K`` at maturity).

    Args:
        strike: Strike.
        notional: Notional (default ``1.0``).
        maturity_step: Observation step (default ``252``).

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import DigitalPut
        >>> DigitalPut(100.0).strike
        100.0
    """

    def __init__(
        self,
        strike: float,
        notional: float = 1.0,
        maturity_step: int = 252,
    ) -> None:
        """See class docstring for parameters.

        Args:
            strike: Strike.
            notional: Notional.
            maturity_step: Maturity step.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import DigitalPut
            >>> DigitalPut(98.0, maturity_step=30).strike
            98.0
        """
        ...

    @property
    def strike(self) -> float:
        """Strike price.

        Args:
            None

        Returns:
            Strike.

        Example:
            >>> from finstack.monte_carlo import DigitalPut
            >>> DigitalPut(97.0).strike
            97.0
        """
        ...

class ForwardLong:
    """Long forward payoff parameters (``S - K`` at maturity).

    Args:
        strike: Delivery strike.
        notional: Notional (default ``1.0``).
        maturity_step: Observation step (default ``252``).

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import ForwardLong
        >>> ForwardLong(100.0).strike
        100.0
    """

    def __init__(
        self,
        strike: float,
        notional: float = 1.0,
        maturity_step: int = 252,
    ) -> None:
        """See class docstring for parameters.

        Args:
            strike: Strike.
            notional: Notional.
            maturity_step: Maturity step.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import ForwardLong
            >>> ForwardLong(103.0, maturity_step=60).strike
            103.0
        """
        ...

    @property
    def strike(self) -> float:
        """Strike / forward price.

        Args:
            None

        Returns:
            Strike.

        Example:
            >>> from finstack.monte_carlo import ForwardLong
            >>> ForwardLong(101.0).strike
            101.0
        """
        ...

class ForwardShort:
    """Short forward payoff parameters (``K - S`` at maturity).

    Args:
        strike: Delivery strike.
        notional: Notional (default ``1.0``).
        maturity_step: Observation step (default ``252``).

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import ForwardShort
        >>> ForwardShort(100.0).strike
        100.0
    """

    def __init__(
        self,
        strike: float,
        notional: float = 1.0,
        maturity_step: int = 252,
    ) -> None:
        """See class docstring for parameters.

        Args:
            strike: Strike.
            notional: Notional.
            maturity_step: Maturity step.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import ForwardShort
            >>> ForwardShort(104.0, maturity_step=40).strike
            104.0
        """
        ...

    @property
    def strike(self) -> float:
        """Strike / forward price.

        Args:
            None

        Returns:
            Strike.

        Example:
            >>> from finstack.monte_carlo import ForwardShort
            >>> ForwardShort(100.5).strike
            100.5
        """
        ...

class AsianCall:
    """Arithmetic Asian call payoff parameters.

    Args:
        strike: Strike on average price.
        notional: Notional (default ``1.0``).
        maturity_step: Last averaging step (default ``252``).

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import AsianCall
        >>> AsianCall(100.0).strike
        100.0
    """

    def __init__(
        self,
        strike: float,
        notional: float = 1.0,
        maturity_step: int = 252,
    ) -> None:
        """See class docstring for parameters.

        Args:
            strike: Strike.
            notional: Notional.
            maturity_step: Maturity step.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import AsianCall
            >>> AsianCall(96.0, maturity_step=80).strike
            96.0
        """
        ...

    @property
    def strike(self) -> float:
        """Strike on the average.

        Args:
            None

        Returns:
            Strike.

        Example:
            >>> from finstack.monte_carlo import AsianCall
            >>> AsianCall(105.0).strike
            105.0
        """
        ...

class AsianPut:
    """Arithmetic Asian put payoff parameters.

    Args:
        strike: Strike on average price.
        notional: Notional (default ``1.0``).
        maturity_step: Last averaging step (default ``252``).

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import AsianPut
        >>> AsianPut(100.0).strike
        100.0
    """

    def __init__(
        self,
        strike: float,
        notional: float = 1.0,
        maturity_step: int = 252,
    ) -> None:
        """See class docstring for parameters.

        Args:
            strike: Strike.
            notional: Notional.
            maturity_step: Maturity step.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import AsianPut
            >>> AsianPut(94.0, maturity_step=70).strike
            94.0
        """
        ...

    @property
    def strike(self) -> float:
        """Strike on the average.

        Args:
            None

        Returns:
            Strike.

        Example:
            >>> from finstack.monte_carlo import AsianPut
            >>> AsianPut(98.0).strike
            98.0
        """
        ...

class BarrierOption:
    """Barrier option payoff parameters.

    Args:
        strike: Option strike.
        barrier: Barrier level.
        is_call: True for call, False for put (default ``True``).
        is_up: True for up barrier, False for down (default ``True``).
        is_knock_out: True for knock-out, False for knock-in (default ``True``).
        notional: Notional (default ``1.0``).
        maturity_step: Observation step (default ``252``).

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import BarrierOption
        >>> BarrierOption(100.0, 120.0).barrier
        120.0
    """

    def __init__(
        self,
        strike: float,
        barrier: float,
        is_call: bool = True,
        is_up: bool = True,
        is_knock_out: bool = True,
        notional: float = 1.0,
        maturity_step: int = 252,
    ) -> None:
        """See class docstring for parameters.

        Args:
            strike: Strike.
            barrier: Barrier.
            is_call: Call/put flag.
            is_up: Up/down flag.
            is_knock_out: Knock-out/in flag.
            notional: Notional.
            maturity_step: Maturity step.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import BarrierOption
            >>> BarrierOption(100.0, 110.0, is_call=False).strike
            100.0
        """
        ...

    @property
    def strike(self) -> float:
        """Strike price.

        Args:
            None

        Returns:
            Strike.

        Example:
            >>> from finstack.monte_carlo import BarrierOption
            >>> BarrierOption(100.0, 115.0).strike
            100.0
        """
        ...

    @property
    def barrier(self) -> float:
        """Barrier level.

        Args:
            None

        Returns:
            Barrier.

        Example:
            >>> from finstack.monte_carlo import BarrierOption
            >>> BarrierOption(100.0, 115.0).barrier
            115.0
        """
        ...

class BasketCall:
    """Basket call payoff parameters (average minus strike).

    Args:
        strike: Strike on weighted basket average.
        weights: Asset weights (should match basket dimension in use).
        notional: Notional (default ``1.0``).
        maturity_step: Observation step (default ``252``).

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import BasketCall
        >>> BasketCall(100.0, [0.5, 0.5]).strike
        100.0
    """

    def __init__(
        self,
        strike: float,
        weights: Sequence[float],
        notional: float = 1.0,
        maturity_step: int = 252,
    ) -> None:
        """See class docstring for parameters.

        Args:
            strike: Strike.
            weights: Weights list.
            notional: Notional.
            maturity_step: Maturity step.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import BasketCall
            >>> BasketCall(100.0, [1.0], maturity_step=10).strike
            100.0
        """
        ...

    @property
    def strike(self) -> float:
        """Strike on basket average.

        Args:
            None

        Returns:
            Strike.

        Example:
            >>> from finstack.monte_carlo import BasketCall
            >>> BasketCall(101.0, [0.25, 0.75]).strike
            101.0
        """
        ...

class BasketPut:
    """Basket put payoff parameters (strike minus average).

    Args:
        strike: Strike on weighted basket average.
        weights: Asset weights.
        notional: Notional (default ``1.0``).
        maturity_step: Observation step (default ``252``).

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import BasketPut
        >>> BasketPut(100.0, [0.5, 0.5]).strike
        100.0
    """

    def __init__(
        self,
        strike: float,
        weights: Sequence[float],
        notional: float = 1.0,
        maturity_step: int = 252,
    ) -> None:
        """See class docstring for parameters.

        Args:
            strike: Strike.
            weights: Weights list.
            notional: Notional.
            maturity_step: Maturity step.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import BasketPut
            >>> BasketPut(100.0, [1.0], maturity_step=12).strike
            100.0
        """
        ...

    @property
    def strike(self) -> float:
        """Strike on basket average.

        Args:
            None

        Returns:
            Strike.

        Example:
            >>> from finstack.monte_carlo import BasketPut
            >>> BasketPut(99.0, [0.5, 0.5]).strike
            99.0
        """
        ...

class AmericanPut:
    """American put exercise specification for LSMC-style pricers.

    Args:
        strike: Strike.

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import AmericanPut
        >>> AmericanPut(100.0).strike
        100.0
    """

    def __init__(self, strike: float) -> None:
        """Create payoff parameters.

        Args:
            strike: Strike.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import AmericanPut
            >>> AmericanPut(95.0).strike
            95.0
        """
        ...

    @property
    def strike(self) -> float:
        """Strike price.

        Args:
            None

        Returns:
            Strike.

        Example:
            >>> from finstack.monte_carlo import AmericanPut
            >>> AmericanPut(100.0).strike
            100.0
        """
        ...

class AmericanCall:
    """American call exercise specification for LSMC-style pricers.

    Args:
        strike: Strike.

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import AmericanCall
        >>> AmericanCall(100.0).strike
        100.0
    """

    def __init__(self, strike: float) -> None:
        """Create payoff parameters.

        Args:
            strike: Strike.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import AmericanCall
            >>> AmericanCall(105.0).strike
            105.0
        """
        ...

    @property
    def strike(self) -> float:
        """Strike price.

        Args:
            None

        Returns:
            Strike.

        Example:
            >>> from finstack.monte_carlo import AmericanCall
            >>> AmericanCall(100.0).strike
            100.0
        """
        ...

class EuropeanPricer:
    """European-option Monte Carlo pricer under GBM (exact time-stepping).

    Args:
        num_paths: Paths (default ``100_000``).
        seed: RNG seed (default ``42``).

    Returns:
        N/A (instance type).

    Example:
        >>> from finstack.monte_carlo import EuropeanPricer
        >>> EuropeanPricer(num_paths=1000, seed=1).price_call(100, 100, 0.05, 0.0, 0.2, 1.0).num_paths
        1000
    """

    def __init__(self, num_paths: int = 100_000, seed: int = 42) -> None:
        """See class docstring for parameters.

        Args:
            num_paths: Path count.
            seed: Seed.

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
    ) -> None:
        """See class docstring for parameters.

        Args:
            num_paths: Path count.
            seed: Seed.

        Returns:
            None

        Example:
            >>> from finstack.monte_carlo import LsmcPricer
            >>> LsmcPricer(50, 3).num_paths
            50
        """
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
