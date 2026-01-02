"""Volatility surface bindings.

Provides volatility surfaces for various asset classes
including equity, credit, and swaption surfaces.
"""

from typing import List, Tuple, Optional

class VolSurface:
    """Two-dimensional volatility surface for options pricing.

    VolSurface represents a grid of implied volatilities indexed by expiry
    (time to expiration) and strike (moneyness or absolute level). It supports
    bilinear interpolation between grid points and provides methods for querying
    volatilities, applying bumps, and creating scenario variants.

    Volatility surfaces are essential for pricing options, swaptions, and other
    volatility-dependent instruments. They can be constructed from market quotes
    or model-generated grids.

    Parameters
    ----------
    id : str
        Unique identifier for the surface (e.g., "SPX", "EURUSD", "IR-SWAPTION").
        Used to retrieve the surface from a :class:`MarketContext`.
    expiries : list[float]
        Expiry times in years (e.g., [0.25, 0.5, 1.0, 2.0]). Must be in
        ascending order and contain at least one value.
    strikes : list[float]
        Strike values. Can be absolute strikes (e.g., [90, 100, 110]) or
        moneyness ratios (e.g., [0.9, 1.0, 1.1]). Must be in ascending order
        and contain at least one value.
    grid : list[list[float]]
        Two-dimensional volatility grid. Must have shape (len(expiries), len(strikes)).
        Each row corresponds to an expiry, each column to a strike.
        Grid[i][j] is the volatility for expiries[i] and strikes[j].

    Returns
    -------
    VolSurface
        Immutable volatility surface ready for queries and use in MarketContext.

    Raises
    ------
    ValueError
        If expiries or strikes are empty, if grid dimensions don't match,
        if expiries/strikes are not in ascending order, or if volatility
        values are invalid (negative).

    Examples
    --------
        >>> from finstack.core.market_data.surfaces import VolSurface
        >>> surface = VolSurface("SPX", [0.5, 1.0], [0.9, 1.0], [[0.20, 0.21], [0.19, 0.20]])
        >>> print(surface.value(0.5, 1.0))
        0.21

    Notes
    -----
    - Volatility surfaces are immutable once constructed
    - Bilinear interpolation is used between grid points
    - Use :meth:`value_checked` for bounds checking (raises on out-of-bounds)
    - Use :meth:`value_clamped` to clamp to grid boundaries
    - Strikes can be absolute or relative (moneyness); consistency is important
    - Volatility values should be positive (as decimals, e.g., 0.20 for 20%)

    See Also
    --------
    :class:`MarketContext`: Container for volatility surfaces
    :class:`MarketBump`: Scenario bumps for volatility surfaces
    """

    def __init__(
        self,
        id: str,
        expiries: List[float],
        strikes: List[float],
        grid: List[List[float]],
    ) -> None: ...
    @property
    def id(self) -> str:
        """Get the surface identifier.

        Returns
        -------
        str
            Surface ID.
        """
        ...

    @property
    def expiries(self) -> List[float]:
        """Get the expiry times.

        Returns
        -------
        List[float]
            Expiry times in years.
        """
        ...

    @property
    def strikes(self) -> List[float]:
        """Get the strike prices.

        Returns
        -------
        List[float]
            Strike prices.
        """
        ...

    @property
    def grid_shape(self) -> Tuple[int, int]:
        """Get the grid shape.

        Returns
        -------
        Tuple[int, int]
            (n_expiries, n_strikes) shape.
        """
        ...

    def value(self, expiry: float, strike: float) -> float:
        """Get interpolated volatility at a specific expiry and strike with flat extrapolation.

        Performs bilinear interpolation between grid points. If the query point
        is outside the grid boundaries, coordinates are clamped to the grid bounds
        (flat extrapolation). This method never raises and is safe for all inputs.

        Use :meth:`value_checked` for explicit error handling on out-of-bounds,
        or :meth:`value_unchecked` when bounds are guaranteed by the caller.

        Parameters
        ----------
        expiry : float
            Time to expiration in years. Clamped to [min_expiry, max_expiry] if out of bounds.
        strike : float
            Strike value. Clamped to [min_strike, max_strike] if out of bounds.

        Returns
        -------
        float
            Interpolated volatility (as a decimal, e.g., 0.20 for 20%).

        Examples
        --------
            >>> vol = vol_surface.value(0.5, 1.0)  # 6M, ATM
            >>> vol = vol_surface.value(0.75, 1.05)  # Interpolated
            >>> vol = vol_surface.value(10.0, 1.0)  # Clamped to max expiry
        """
        ...

    def value_checked(self, expiry: float, strike: float) -> float:
        """Get volatility with strict bounds checking.

        Raises an error if the query point is outside the grid boundaries.
        Use this when you want explicit error handling for out-of-bounds queries.

        Parameters
        ----------
        expiry : float
            Time to expiration in years. Must be within [min_expiry, max_expiry].
        strike : float
            Strike value. Must be within [min_strike, max_strike].

        Returns
        -------
        float
            Interpolated volatility (as a decimal).

        Raises
        ------
        ValueError
            If expiry or strike is outside the grid boundaries. The error message
            indicates which dimension is out of bounds and the valid range.
        """
        ...

    def value_clamped(self, expiry: float, strike: float) -> float:
        """Get volatility with clamping to grid boundaries.

        Alias for :meth:`value` - both use flat extrapolation (clamping to edge values).

        Parameters
        ----------
        expiry : float
            Time to expiration in years. Clamped to [min_expiry, max_expiry].
        strike : float
            Strike value. Clamped to [min_strike, max_strike].

        Returns
        -------
        float
            Interpolated volatility using clamped coordinates.
        """
        ...

    def value_unchecked(self, expiry: float, strike: float) -> float:
        """Get volatility without bounds checking.

        Panics if the query point is outside the grid boundaries. Use only when
        bounds are guaranteed by the caller for maximum performance.

        Parameters
        ----------
        expiry : float
            Time to expiration in years. Must be within [min_expiry, max_expiry].
        strike : float
            Strike value. Must be within [min_strike, max_strike].

        Returns
        -------
        float
            Interpolated volatility (as a decimal).

        Raises
        ------
        RuntimeError
            If expiry or strike is outside the grid boundaries (panic from Rust).
        """
        ...

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
