"""Stochastic volatility model parameter types.

Provides:
- HestonParams: Heston (1993) stochastic volatility model
- SabrParams: SABR (Hagan 2002) stochastic alpha-beta-rho model
- SviParams: SVI (Gatheral 2004) implied variance parameterization
"""

from __future__ import annotations

class HestonParams:
    """Heston stochastic volatility model parameters.

    The Heston model describes joint dynamics of an asset price and its
    instantaneous variance using five parameters.

    Parameters
    ----------
    v0 : float
        Initial variance (must be > 0).
    kappa : float
        Mean reversion speed (must be > 0).
    theta : float
        Long-run variance level (must be > 0).
    sigma : float
        Vol-of-vol (must be > 0).
    rho : float
        Correlation between spot and variance, in (-1, 1).

    Raises
    ------
    ValueError
        If any parameter is out of range.

    Examples
    --------
    >>> params = HestonParams(v0=0.04, kappa=2.0, theta=0.04, sigma=0.3, rho=-0.5)
    >>> params.satisfies_feller_condition()
    True
    >>> price = params.price_european(100.0, 100.0, 0.05, 0.0, 1.0, True)
    """

    def __init__(
        self,
        v0: float,
        kappa: float,
        theta: float,
        sigma: float,
        rho: float,
    ) -> None: ...
    @property
    def v0(self) -> float:
        """Initial variance (v0)."""
        ...
    @property
    def kappa(self) -> float:
        """Mean reversion speed (kappa)."""
        ...
    @property
    def theta(self) -> float:
        """Long-run variance level (theta)."""
        ...
    @property
    def sigma(self) -> float:
        """Vol-of-vol (sigma)."""
        ...
    @property
    def rho(self) -> float:
        """Correlation (rho)."""
        ...
    def satisfies_feller_condition(self) -> bool:
        """Check whether the Feller condition (2*kappa*theta > sigma^2) is satisfied.

        When satisfied, the variance process is strictly positive almost surely.

        Returns
        -------
        bool
            True if the Feller condition holds.
        """
        ...
    def price_european(
        self,
        spot: float,
        strike: float,
        r: float,
        q: float,
        t: float,
        is_call: bool,
    ) -> float:
        """Price a European option using Fourier integration.

        Parameters
        ----------
        spot : float
            Current spot price.
        strike : float
            Strike price.
        r : float
            Risk-free rate (continuous compounding).
        q : float
            Dividend yield (continuous compounding).
        t : float
            Time to expiry in years.
        is_call : bool
            True for call, False for put.

        Returns
        -------
        float
            Option price (non-negative).
        """
        ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class SabrParams:
    """SABR stochastic volatility model parameters.

    The SABR model is the market standard for swaption and cap/floor
    volatility smile modeling.

    Parameters
    ----------
    alpha : float
        Initial volatility level (must be > 0).
    beta : float
        CEV exponent, in [0, 1].
    rho : float
        Correlation, in (-1, 1).
    nu : float
        Vol-of-vol (must be > 0).

    Raises
    ------
    ValueError
        If any parameter is out of range.

    Examples
    --------
    >>> params = SabrParams(alpha=0.035, beta=0.5, rho=-0.2, nu=0.4)
    >>> vol = params.implied_vol_lognormal(0.05, 0.05, 1.0)
    """

    def __init__(
        self,
        alpha: float,
        beta: float,
        rho: float,
        nu: float,
    ) -> None: ...
    @property
    def alpha(self) -> float:
        """Initial volatility level (alpha)."""
        ...
    @property
    def beta(self) -> float:
        """CEV exponent (beta)."""
        ...
    @property
    def rho(self) -> float:
        """Correlation (rho)."""
        ...
    @property
    def nu(self) -> float:
        """Vol-of-vol (nu)."""
        ...
    def implied_vol_lognormal(self, f: float, k: float, t: float) -> float:
        """Lognormal (Black-76) implied volatility using Hagan's approximation.

        Parameters
        ----------
        f : float
            Forward rate.
        k : float
            Strike rate.
        t : float
            Time to expiry in years.

        Returns
        -------
        float
            Black-76 implied volatility.
        """
        ...
    def implied_vol_normal(self, f: float, k: float, t: float) -> float:
        """Normal (Bachelier) implied volatility using Hagan's approximation.

        Parameters
        ----------
        f : float
            Forward rate (may be negative).
        k : float
            Strike rate (may be negative).
        t : float
            Time to expiry in years.

        Returns
        -------
        float
            Normal/Bachelier implied volatility.
        """
        ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class SviParams:
    """SVI (Stochastic Volatility Inspired) raw parameterization.

    Represents one slice of the volatility surface at a fixed expiry using
    five parameters that control the shape of the smile.

    Parameters
    ----------
    a : float
        Overall variance level.
    b : float
        Slope of the wings (must be >= 0).
    rho : float
        Rotation / asymmetry, in (-1, 1).
    m : float
        Translation (shift of minimum variance point).
    sigma : float
        Smoothing parameter (must be > 0).

    Raises
    ------
    ValueError
        If parameters violate no-arbitrage conditions.

    Examples
    --------
    >>> params = SviParams(a=0.04, b=0.4, rho=-0.4, m=0.0, sigma=0.1)
    >>> w = params.total_variance(0.0)
    >>> vol = params.implied_vol(0.0, 1.0)
    """

    def __init__(
        self,
        a: float,
        b: float,
        rho: float,
        m: float,
        sigma: float,
    ) -> None: ...
    @property
    def a(self) -> float:
        """Overall variance level (a)."""
        ...
    @property
    def b(self) -> float:
        """Slope of the wings (b)."""
        ...
    @property
    def rho(self) -> float:
        """Rotation / asymmetry (rho)."""
        ...
    @property
    def m(self) -> float:
        """Translation (m)."""
        ...
    @property
    def sigma(self) -> float:
        """Smoothing parameter (sigma)."""
        ...
    def total_variance(self, k: float) -> float:
        """Compute the total implied variance w(k) at log-moneyness k.

        Parameters
        ----------
        k : float
            Log-moneyness, ln(K/F).

        Returns
        -------
        float
            Total implied variance w(k) = sigma^2 * T.
        """
        ...
    def implied_vol(self, k: float, t: float) -> float:
        """Compute Black-Scholes implied volatility from SVI total variance.

        Parameters
        ----------
        k : float
            Log-moneyness, ln(K/F).
        t : float
            Time to expiry in years (must be > 0).

        Returns
        -------
        float
            Implied volatility. Returns NaN if t <= 0 or total variance is negative.
        """
        ...
    def validate(self) -> None:
        """Validate SVI parameters against no-arbitrage constraints.

        Raises
        ------
        ValueError
            If parameters violate no-arbitrage conditions.
        """
        ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
