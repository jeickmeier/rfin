"""Volatility conventions, pricing models, and conversion utilities.

This module provides volatility quoting conventions and option pricing
functions for various models (Bachelier, Black, Shifted Black).
"""

class VolatilityConvention:
    """Volatility quoting convention.

    Supports Normal (Bachelier), Lognormal (Black), and Shifted Lognormal conventions.
    """

    NORMAL: VolatilityConvention
    LOGNORMAL: VolatilityConvention

    @classmethod
    def shifted_lognormal(cls, shift: float) -> VolatilityConvention:
        """Create a Shifted Lognormal convention with the specified shift.

        Parameters
        ----------
        shift : float
            Shift amount for the shifted lognormal model.

        Returns
        -------
        VolatilityConvention
            Shifted lognormal convention instance.
        """
        ...

    @property
    def kind(self) -> str:
        """String representation of the convention type ('normal', 'lognormal', 'shifted_lognormal')."""
        ...

    @property
    def shift(self) -> float | None:
        """Shift amount (only for Shifted Lognormal, else None)."""
        ...

    def __repr__(self) -> str: ...

def bachelier_price(forward: float, strike: float, sigma_n: float, t: float) -> float:
    """Compute the price of a call option under the Bachelier (Normal) model.

    Assumes a unit annuity (PV01=1).

    Parameters
    ----------
    forward : float
        Forward rate.
    strike : float
        Strike rate.
    sigma_n : float
        Normal volatility (in absolute units, e.g. 0.01 for 100bps).
    t : float
        Time to expiry in years.

    Returns
    -------
    float
        Option price.
    """
    ...

def black_price(forward: float, strike: float, sigma: float, t: float) -> float:
    """Compute the price of a call option under the Black (Lognormal) model.

    Assumes a unit annuity (PV01=1).

    Parameters
    ----------
    forward : float
        Forward rate.
    strike : float
        Strike rate.
    sigma : float
        Lognormal volatility (decimal, e.g. 0.20 for 20%).
    t : float
        Time to expiry in years.

    Returns
    -------
    float
        Option price.
    """
    ...

def black_shifted_price(forward: float, strike: float, sigma: float, t: float, shift: float) -> float:
    """Compute the price of a call option under the Shifted Black model.

    Parameters
    ----------
    forward : float
        Forward rate.
    strike : float
        Strike rate.
    sigma : float
        Lognormal volatility.
    t : float
        Time to expiry in years.
    shift : float
        Shift amount applied to forward and strike.

    Returns
    -------
    float
        Option price.
    """
    ...

def convert_volatility(
    vol: float,
    from_convention: VolatilityConvention,
    to_convention: VolatilityConvention,
    forward_rate: float,
    time_to_expiry: float,
    zero_threshold: float = 1e-8,
) -> float:
    """Convert volatility between conventions by equating option prices.

    Parameters
    ----------
    vol : float
        Input volatility.
    from_convention : VolatilityConvention
        Source convention.
    to_convention : VolatilityConvention
        Target convention.
    forward_rate : float
        Forward rate for the underlying.
    time_to_expiry : float
        Time to expiry in years.
    zero_threshold : float, optional
        Threshold below which rates are considered zero (default 1e-8).

    Returns
    -------
    float
        Converted volatility in the target convention.
    """
    ...

__all__ = [
    "VolatilityConvention",
    "bachelier_price",
    "black_price",
    "black_shifted_price",
    "convert_volatility",
]
