"""Stochastic process parameter types for Monte Carlo simulation."""

from __future__ import annotations

class GbmParams:
    """Parameters for Geometric Brownian Motion.

    SDE: dS_t = (r - q) S_t dt + sigma S_t dW_t

    Args:
        r: Risk-free rate (annual)
        q: Dividend/foreign rate (annual)
        sigma: Volatility (annual)
    """

    def __init__(self, r: float, q: float, sigma: float) -> None: ...

    @property
    def r(self) -> float:
        """Risk-free rate."""
        ...

    @property
    def q(self) -> float:
        """Dividend/foreign rate."""
        ...

    @property
    def sigma(self) -> float:
        """Volatility."""
        ...

class HestonParams:
    """Parameters for the Heston stochastic volatility model.

    SDE:
        dS_t = (r - q) S_t dt + sqrt(v_t) S_t dW1_t
        dv_t = kappa (theta - v_t) dt + sigma_v sqrt(v_t) dW2_t
        Corr(dW1, dW2) = rho

    Args:
        r: Risk-free rate
        q: Dividend yield
        kappa: Mean reversion speed (> 0)
        theta: Long-term variance (> 0)
        sigma_v: Vol-of-vol (> 0)
        rho: Correlation between asset and variance in [-1, 1]
        v0: Initial variance (> 0)
    """

    def __init__(
        self,
        r: float,
        q: float,
        kappa: float,
        theta: float,
        sigma_v: float,
        rho: float,
        v0: float,
    ) -> None: ...

    @property
    def r(self) -> float: ...
    @property
    def q(self) -> float: ...
    @property
    def kappa(self) -> float: ...
    @property
    def theta(self) -> float: ...
    @property
    def sigma_v(self) -> float: ...
    @property
    def rho(self) -> float: ...
    @property
    def v0(self) -> float: ...

    def satisfies_feller(self) -> bool:
        """Check if the Feller condition (2*kappa*theta >= sigma_v^2) is satisfied.

        When satisfied, the variance process stays strictly positive.
        """
        ...

class CirParams:
    """Parameters for the CIR (Cox-Ingersoll-Ross) square-root diffusion.

    SDE: dv_t = kappa (theta - v_t) dt + sigma sqrt(v_t) dW_t

    Used for modeling short rates, stochastic volatility, and credit intensities.

    Args:
        kappa: Mean reversion speed (> 0)
        theta: Long-term mean (>= 0)
        sigma: Volatility of volatility (> 0)
    """

    def __init__(self, kappa: float, theta: float, sigma: float) -> None: ...

    @property
    def kappa(self) -> float: ...
    @property
    def theta(self) -> float: ...
    @property
    def sigma(self) -> float: ...

    def satisfies_feller(self) -> bool:
        """Check if Feller condition (2*kappa*theta >= sigma^2) is satisfied."""
        ...

class HullWhite1FParams:
    """Parameters for the Hull-White one-factor short rate model.

    SDE: dr_t = kappa [theta(t) - r_t] dt + sigma dW_t

    Supports both constant theta (Vasicek model) and time-dependent theta(t)
    for fitting the initial yield curve.

    Args:
        kappa: Mean reversion speed
        sigma: Instantaneous volatility
        theta: Constant mean reversion level (for Vasicek/constant theta)
    """

    def __init__(self, kappa: float, sigma: float, theta: float) -> None: ...

    @staticmethod
    def with_time_dependent_theta(
        kappa: float,
        sigma: float,
        theta_curve: list[float],
        theta_times: list[float],
    ) -> HullWhite1FParams:
        """Create with time-dependent theta(t).

        Args:
            kappa: Mean reversion speed
            sigma: Volatility
            theta_curve: List of theta values (piecewise constant)
            theta_times: List of time breakpoints (must be sorted)
        """
        ...

    @property
    def kappa(self) -> float: ...
    @property
    def sigma(self) -> float: ...

    def theta_at_time(self, t: float) -> float:
        """Get theta(t) at a given time."""
        ...

class MertonJumpParams:
    """Parameters for the Merton jump-diffusion model.

    SDE: dS_t/S_t = (r - q - lambda*k) dt + sigma dW_t + (J-1) dN_t

    where:
        lambda = jump intensity (average jumps per year)
        J ~ LogNormal(mu_j, sigma_j^2)
        k = E[J-1] = exp(mu_j + sigma_j^2/2) - 1

    Args:
        r: Risk-free rate
        q: Dividend yield
        sigma: Continuous diffusion volatility
        lambda_: Jump intensity (jumps per year)
        mu_j: Mean of log-jump size
        sigma_j: Std dev of log-jump size
    """

    def __init__(
        self,
        r: float,
        q: float,
        sigma: float,
        lambda_: float,
        mu_j: float,
        sigma_j: float,
    ) -> None: ...

    @property
    def r(self) -> float: ...
    @property
    def q(self) -> float: ...
    @property
    def sigma(self) -> float: ...
    @property
    def lambda_(self) -> float: ...
    @property
    def mu_j(self) -> float: ...
    @property
    def sigma_j(self) -> float: ...

    def jump_compensation(self) -> float:
        """Compute jump compensation term k = E[J - 1]."""
        ...

    def compensated_drift(self) -> float:
        """Compensated drift rate for risk-neutral measure (r - q - lambda*k)."""
        ...

class SchwartzSmithParams:
    """Parameters for the Schwartz-Smith two-factor commodity model.

    SDE:
        dX_t = -kappa_x X_t dt + sigma_x dW_X   (short-term, mean-reverting)
        dY_t = mu_y dt + sigma_y dW_Y            (long-term trend)
        S_t = exp(X_t + Y_t)                     (spot price)
        Corr(dW_X, dW_Y) = rho

    Args:
        kappa_x: Mean reversion speed for short-term deviation (> 0)
        sigma_x: Short-term volatility (> 0)
        mu_y: Long-term drift
        sigma_y: Long-term volatility (> 0)
        rho: Correlation between X and Y in [-1, 1]
    """

    def __init__(
        self,
        kappa_x: float,
        sigma_x: float,
        mu_y: float,
        sigma_y: float,
        rho: float,
    ) -> None: ...

    @property
    def kappa_x(self) -> float: ...
    @property
    def sigma_x(self) -> float: ...
    @property
    def mu_y(self) -> float: ...
    @property
    def sigma_y(self) -> float: ...
    @property
    def rho(self) -> float: ...

class BrownianParams:
    """Parameters for 1D Brownian motion with drift.

    SDE: dX_t = mu dt + sigma dW_t

    Args:
        mu: Constant drift
        sigma: Constant diffusion
    """

    def __init__(self, mu: float, sigma: float) -> None: ...

    @property
    def mu(self) -> float: ...
    @property
    def sigma(self) -> float: ...

class MultiOuParams:
    """Parameters for multi-dimensional Ornstein-Uhlenbeck process.

    SDE (component i): dX_i = kappa_i (theta_i - X_i) dt + sigma_i dW_i
    with optional correlation across the driving Brownian motions.

    Args:
        kappas: Mean reversion speeds (> 0)
        thetas: Long-run means
        sigmas: Volatilities (>= 0)
        correlation: Optional correlation matrix (n x n, row-major flat list)
    """

    def __init__(
        self,
        kappas: list[float],
        thetas: list[float],
        sigmas: list[float],
        correlation: list[float] | None = None,
    ) -> None: ...

    @property
    def kappas(self) -> list[float]: ...
    @property
    def thetas(self) -> list[float]: ...
    @property
    def sigmas(self) -> list[float]: ...
    @property
    def correlation(self) -> list[float] | None: ...

    def dim(self) -> int:
        """Get the number of dimensions."""
        ...

__all__ = [
    "GbmParams",
    "HestonParams",
    "CirParams",
    "HullWhite1FParams",
    "MertonJumpParams",
    "SchwartzSmithParams",
    "BrownianParams",
    "MultiOuParams",
]
