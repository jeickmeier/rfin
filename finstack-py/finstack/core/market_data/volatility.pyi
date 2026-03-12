"""Volatility conventions, pricing models, Greeks, implied vol solvers, and conversion utilities.

This module provides volatility quoting conventions and option pricing
functions for various models (Bachelier, Black, Shifted Black), along with
Greeks (vega, delta, gamma) and implied volatility extraction.
"""

from __future__ import annotations

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

# =============================================================================
# Legacy convenience wrappers (call-only pricing)
# =============================================================================

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

# =============================================================================
# Black-76 (Lognormal) Model
# =============================================================================

def black_call(forward: float, strike: float, sigma: float, t: float) -> float:
    """Compute the price of a call option under the Black-76 (Lognormal) model.

    Assumes a unit annuity (PV01=1).

    Parameters
    ----------
    forward : float
        Forward rate (must be positive).
    strike : float
        Strike rate (must be positive).
    sigma : float
        Lognormal volatility (e.g. 0.20 for 20%).
    t : float
        Time to expiry in years.

    Returns
    -------
    float
        Call option price per unit annuity.
    """
    ...

def black_put(forward: float, strike: float, sigma: float, t: float) -> float:
    """Compute the price of a put option under the Black-76 (Lognormal) model.

    Assumes a unit annuity (PV01=1).

    Parameters
    ----------
    forward : float
        Forward rate (must be positive).
    strike : float
        Strike rate (must be positive).
    sigma : float
        Lognormal volatility (e.g. 0.20 for 20%).
    t : float
        Time to expiry in years.

    Returns
    -------
    float
        Put option price per unit annuity.
    """
    ...

def black_vega(forward: float, strike: float, sigma: float, t: float) -> float:
    """Compute Black-76 vega: sensitivity of option price to lognormal volatility.

    Same for both calls and puts.

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

    Returns
    -------
    float
        Vega per unit change in vol (per unit annuity).
    """
    ...

def black_delta_call(forward: float, strike: float, sigma: float, t: float) -> float:
    """Compute Black-76 call delta: sensitivity of call price to forward rate.

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

    Returns
    -------
    float
        Call delta (per unit annuity).
    """
    ...

def black_delta_put(forward: float, strike: float, sigma: float, t: float) -> float:
    """Compute Black-76 put delta: sensitivity of put price to forward rate.

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

    Returns
    -------
    float
        Put delta (per unit annuity).
    """
    ...

def black_gamma(forward: float, strike: float, sigma: float, t: float) -> float:
    """Compute Black-76 gamma: second derivative of option price w.r.t. forward.

    Same for both calls and puts.

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

    Returns
    -------
    float
        Gamma (per unit annuity).
    """
    ...

# =============================================================================
# Bachelier (Normal) Model
# =============================================================================

def bachelier_call(forward: float, strike: float, sigma_n: float, t: float) -> float:
    """Compute the price of a call option under the Bachelier (Normal) model.

    Assumes a unit annuity (PV01=1).

    Parameters
    ----------
    forward : float
        Forward rate.
    strike : float
        Strike rate.
    sigma_n : float
        Normal volatility (in rate terms, e.g. 0.005 = 50bp).
    t : float
        Time to expiry in years.

    Returns
    -------
    float
        Call option price per unit annuity.
    """
    ...

def bachelier_put(forward: float, strike: float, sigma_n: float, t: float) -> float:
    """Compute the price of a put option under the Bachelier (Normal) model.

    Assumes a unit annuity (PV01=1).

    Parameters
    ----------
    forward : float
        Forward rate.
    strike : float
        Strike rate.
    sigma_n : float
        Normal volatility (in rate terms, e.g. 0.005 = 50bp).
    t : float
        Time to expiry in years.

    Returns
    -------
    float
        Put option price per unit annuity.
    """
    ...

def bachelier_vega(forward: float, strike: float, sigma_n: float, t: float) -> float:
    """Compute Bachelier vega: sensitivity of option price to normal volatility.

    Same for both calls and puts.

    Parameters
    ----------
    forward : float
        Forward rate.
    strike : float
        Strike rate.
    sigma_n : float
        Normal volatility.
    t : float
        Time to expiry in years.

    Returns
    -------
    float
        Vega per unit change in normal vol (per unit annuity).
    """
    ...

def bachelier_delta_call(forward: float, strike: float, sigma_n: float, t: float) -> float:
    """Compute Bachelier call delta: sensitivity of call price to forward rate.

    Parameters
    ----------
    forward : float
        Forward rate.
    strike : float
        Strike rate.
    sigma_n : float
        Normal volatility.
    t : float
        Time to expiry in years.

    Returns
    -------
    float
        Call delta (per unit annuity).
    """
    ...

def bachelier_delta_put(forward: float, strike: float, sigma_n: float, t: float) -> float:
    """Compute Bachelier put delta: sensitivity of put price to forward rate.

    Parameters
    ----------
    forward : float
        Forward rate.
    strike : float
        Strike rate.
    sigma_n : float
        Normal volatility.
    t : float
        Time to expiry in years.

    Returns
    -------
    float
        Put delta (per unit annuity).
    """
    ...

def bachelier_gamma(forward: float, strike: float, sigma_n: float, t: float) -> float:
    """Compute Bachelier gamma: second derivative of option price w.r.t. forward.

    Same for both calls and puts.

    Parameters
    ----------
    forward : float
        Forward rate.
    strike : float
        Strike rate.
    sigma_n : float
        Normal volatility.
    t : float
        Time to expiry in years.

    Returns
    -------
    float
        Gamma (per unit annuity).
    """
    ...

# =============================================================================
# Shifted Black Model
# =============================================================================

def black_shifted_call(forward: float, strike: float, sigma: float, t: float, shift: float) -> float:
    """Compute the price of a call option under the Shifted Black model.

    Handles negative rates by shifting both forward and strike.

    Parameters
    ----------
    forward : float
        Forward rate (can be negative).
    strike : float
        Strike rate (can be negative).
    sigma : float
        Lognormal volatility.
    t : float
        Time to expiry in years.
    shift : float
        Shift amount (e.g. 0.03 = 3% shift).

    Returns
    -------
    float
        Call option price per unit annuity.
    """
    ...

def black_shifted_put(forward: float, strike: float, sigma: float, t: float, shift: float) -> float:
    """Compute the price of a put option under the Shifted Black model.

    Parameters
    ----------
    forward : float
        Forward rate (can be negative).
    strike : float
        Strike rate (can be negative).
    sigma : float
        Lognormal volatility.
    t : float
        Time to expiry in years.
    shift : float
        Shift amount (e.g. 0.03 = 3% shift).

    Returns
    -------
    float
        Put option price per unit annuity.
    """
    ...

def black_shifted_vega(forward: float, strike: float, sigma: float, t: float, shift: float) -> float:
    """Compute Shifted Black vega with unit annuity.

    Parameters
    ----------
    forward : float
        Forward rate (can be negative).
    strike : float
        Strike rate (can be negative).
    sigma : float
        Lognormal volatility.
    t : float
        Time to expiry in years.
    shift : float
        Shift amount (e.g. 0.03 = 3% shift).

    Returns
    -------
    float
        Vega per unit change in vol (per unit annuity).
    """
    ...

# =============================================================================
# Implied Volatility Solvers
# =============================================================================

def implied_vol_black(price: float, forward: float, strike: float, t: float, is_call: bool) -> float:
    """Extract Black-76 (lognormal) implied volatility from an option price.

    Given a market option price, finds the unique lognormal volatility that
    reproduces the price under the Black-76 model.

    Parameters
    ----------
    price : float
        Market option price per unit annuity (non-negative).
    forward : float
        Forward rate or price (must be positive and finite).
    strike : float
        Strike rate or price (must be positive and finite).
    t : float
        Time to expiry in years (must be positive and finite).
    is_call : bool
        True for a call option, False for a put option.

    Returns
    -------
    float
        The implied lognormal volatility.

    Raises
    ------
    ValueError
        If inputs are invalid or the solver fails to converge.
    """
    ...

def implied_vol_bachelier(price: float, forward: float, strike: float, t: float, is_call: bool) -> float:
    """Extract Bachelier (normal) implied volatility from an option price.

    Given a market option price, finds the unique normal volatility that
    reproduces the price under the Bachelier model.

    Parameters
    ----------
    price : float
        Market option price per unit annuity (non-negative).
    forward : float
        Forward rate (any finite value; negative rates supported).
    strike : float
        Strike rate (any finite value).
    t : float
        Time to expiry in years (must be positive and finite).
    is_call : bool
        True for a call option, False for a put option.

    Returns
    -------
    float
        The implied normal volatility.

    Raises
    ------
    ValueError
        If inputs are invalid or the solver fails to converge.
    """
    ...

def brenner_subrahmanyam_approx(forward: float, strike: float, option_price: float, t: float) -> float:
    """Brenner-Subrahmanyam ATM approximation for Black implied volatility."""
    ...

def manaster_koehler_approx(forward: float, strike: float, t: float) -> float:
    """Manaster-Koehler approximation for Black implied volatility."""
    ...

def implied_vol_initial_guess(forward: float, strike: float, option_price: float, t: float) -> float:
    """Combined initial guess for implied volatility solvers."""
    ...

# =============================================================================
# Volatility Convention Conversion
# =============================================================================

def convert_atm_volatility(
    vol: float,
    from_convention: VolatilityConvention,
    to_convention: VolatilityConvention,
    forward_rate: float,
    time_to_expiry: float,
) -> float:
    """Convert ATM volatility between conventions by equating option prices.

    This function performs ATM (at-the-money, strike = forward) volatility conversion.
    For surface-aware or strike-specific conversions, use a volatility surface.

    Parameters
    ----------
    vol : float
        Input volatility (must be positive and finite).
    from_convention : VolatilityConvention
        Source convention.
    to_convention : VolatilityConvention
        Target convention.
    forward_rate : float
        Forward rate for the underlying.
    time_to_expiry : float
        Time to expiry in years (must be non-negative).

    Returns
    -------
    float
        Converted volatility in the target convention.

    Raises
    ------
    ValueError
        If vol is not positive/finite, time_to_expiry is negative,
        or forward_rate is non-positive for lognormal conventions.
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

    .. deprecated:: 0.2.0
        Use :func:`convert_atm_volatility` instead, which provides explicit error handling.

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
        Threshold below which rates are considered zero (default 1e-8). **Ignored**.

    Returns
    -------
    float
        Converted volatility in the target convention. Returns input volatility on error.
    """
    ...

__all__ = [
    "VolatilityConvention",
    # Legacy convenience wrappers
    "bachelier_price",
    "black_price",
    "black_shifted_price",
    # Black-76
    "black_call",
    "black_put",
    "black_vega",
    "black_delta_call",
    "black_delta_put",
    "black_gamma",
    # Bachelier
    "bachelier_call",
    "bachelier_put",
    "bachelier_vega",
    "bachelier_delta_call",
    "bachelier_delta_put",
    "bachelier_gamma",
    # Shifted Black
    "black_shifted_call",
    "black_shifted_put",
    "black_shifted_vega",
    # Implied vol solvers
    "implied_vol_black",
    "implied_vol_bachelier",
    "brenner_subrahmanyam_approx",
    "manaster_koehler_approx",
    "implied_vol_initial_guess",
    # Conversion
    "convert_atm_volatility",
    "convert_volatility",
]
