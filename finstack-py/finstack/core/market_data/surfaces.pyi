"""Volatility surface bindings.

Provides volatility surfaces for various asset classes
including equity, credit, and swaption surfaces.
"""

from typing import List, Tuple, Optional

class VolSurface:
    """Volatility surface for options pricing.

    Parameters
    ----------
    id : str
        Surface identifier.
    expiries : list[float]
        Expiry times in years.
    strikes : list[float]
        Strike prices.
    grid : list[list[float]]
        2D volatility grid (expiry x strike).
    """

    def __init__(
        self,
        id: str,
        expiries: List[float],
        strikes: List[float],
        grid: List[List[float]],
    ) -> None: ...
    @property
    def id(self) -> str: ...
    """Get the surface identifier.
    
    Returns
    -------
    str
        Surface ID.
    """

    @property
    def expiries(self) -> List[float]: ...
    """Get the expiry times.
    
    Returns
    -------
    List[float]
        Expiry times in years.
    """

    @property
    def strikes(self) -> List[float]: ...
    """Get the strike prices.
    
    Returns
    -------
    List[float]
        Strike prices.
    """

    @property
    def grid_shape(self) -> Tuple[int, int]: ...
    """Get the grid shape.
    
    Returns
    -------
    Tuple[int, int]
        (n_expiries, n_strikes) shape.
    """

    def value(self, expiry: float, strike: float) -> float: ...
    """Get volatility at expiry and strike.
    
    Parameters
    ----------
    expiry : float
        Expiry time in years.
    strike : float
        Strike price.
        
    Returns
    -------
    float
        Volatility.
    """

    def value_checked(self, expiry: float, strike: float) -> float: ...
    """Get volatility with bounds checking.
    
    Parameters
    ----------
    expiry : float
        Expiry time in years.
    strike : float
        Strike price.
        
    Returns
    -------
    float
        Volatility.
        
    Raises
    ------
    ValueError
        If expiry or strike is out of bounds.
    """

    def value_clamped(self, expiry: float, strike: float) -> float: ...
    """Get volatility with clamping to bounds.
    
    Parameters
    ----------
    expiry : float
        Expiry time in years.
    strike : float
        Strike price.
        
    Returns
    -------
    float
        Clamped volatility.
    """

    def bump_point(self, expiry: float, strike: float, bump_pct: float) -> "VolSurface":
        """Return a new surface with a single point bumped.

        Parameters
        ----------
        expiry : float
            Expiry time in years.
        strike : float
            Strike value.
        bump_pct : float
            Relative bump percentage (e.g., 0.01 for 1%).

        Returns
        -------
        VolSurface
            New surface with the specified point bumped.
        """
        ...

    def scaled(self, scale: float) -> "VolSurface":
        """Return a new surface with all volatilities scaled by a factor.

        Parameters
        ----------
        scale : float
            Scaling factor (e.g., 1.1 for 10% increase).

        Returns
        -------
        VolSurface
            New surface with scaled volatilities.
        """
        ...

    def apply_bucket_bump(
        self,
        pct: float,
        expiries_filter: Optional[List[float]] = None,
        strikes_filter: Optional[List[float]] = None,
    ) -> "VolSurface":
        """Apply a bucket bump to volatilities matching the filters.

        Parameters
        ----------
        pct : float
            Percentage bump to apply (e.g. 1.0 for 1% bump).
        expiries_filter : list[float], optional
            List of expiries to bump. If None, all expiries are bumped.
        strikes_filter : list[float], optional
            List of strikes to bump. If None, all strikes are bumped.

        Returns
        -------
        VolSurface
            New surface with applied bumps.
        """
        ...

    def __repr__(self) -> str: ...
