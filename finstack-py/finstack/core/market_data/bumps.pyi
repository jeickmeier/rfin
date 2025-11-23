'"""Market bump specifications for scenario generation."""'

from __future__ import annotations

from datetime import date
from typing import Literal, Sequence

from ..currency import Currency
from ..types import CurveId

class BumpMode:
    """Mode for applying a bump."""

    ADDITIVE: BumpMode
    MULTIPLICATIVE: BumpMode

class BumpUnits:
    """Units used to interpret bump magnitudes."""

    RATE_BP: BumpUnits
    PERCENT: BumpUnits
    FRACTION: BumpUnits
    FACTOR: BumpUnits

class BumpType:
    """Parallel or key-rate bump description."""

    PARALLEL: BumpType

    @staticmethod
    def key_rate(time_years: float) -> BumpType: ...
    @property
    def is_key_rate(self) -> bool: ...
    @property
    def time_years(self) -> float | None: ...

class BumpSpec:
    """Unified specification for market data shifts used in scenario analysis.

    BumpSpec combines a bump mode (additive or multiplicative), units
    (basis points, percent, etc.), magnitude, and type (parallel or key-rate)
    into a single specification. It is used with :class:`MarketBump` to define
    how market data should be shifted for stress testing and risk calculations.

    Parameters
    ----------
    mode : BumpMode
        How the bump is applied:
        - ADDITIVE: Add the bump value to the current rate/price
        - MULTIPLICATIVE: Multiply the current rate/price by (1 + bump_value)
    units : BumpUnits
        Units for interpreting the bump magnitude:
        - RATE_BP: Basis points (0.01% per bp)
        - PERCENT: Percentage points (1% = 0.01)
        - FRACTION: Decimal fraction (0.01 = 1%)
        - FACTOR: Multiplicative factor (1.01 = 1% increase)
    value : float
        Magnitude of the bump in the specified units.
    bump_type : BumpType, optional
        Type of curve shift:
        - PARALLEL: Shift entire curve by same amount (default)
        - KeyRate: Shift only at a specific tenor (via :meth:`BumpType.key_rate`)

    Returns
    -------
    BumpSpec
        Bump specification ready for use with :class:`MarketBump`.

    Examples
    --------
        >>> from finstack.core.market_data.bumps import BumpSpec
        >>> spec = BumpSpec.parallel_bp(10.0)
        >>> print((spec.units, spec.value, spec.bump_type.is_key_rate))
        (BumpUnits.RATE_BP, 10.0, False)

    Notes
    -----
    - Use factory methods (:meth:`parallel_bp`, :meth:`key_rate_bp`, etc.) for
      common patterns
    - Parallel bumps shift the entire curve uniformly
    - Key-rate bumps affect only a specific tenor (used for DV01 calculations)
    - Units must match the market data type (bp for rates, percent for spreads)

    See Also
    --------
    :class:`MarketBump`: Concrete bumps applied to market data
    :class:`BumpMode`: Additive vs multiplicative modes
    :class:`BumpUnits`: Unit conventions
    :class:`BumpType`: Parallel vs key-rate shifts
    """

    def __init__(
        self,
        mode: BumpMode,
        units: BumpUnits,
        value: float,
        bump_type: BumpType | None = ...,
    ) -> None: ...
    @staticmethod
    def parallel_bp(bump_bp: float) -> BumpSpec: ...
    """Create a parallel bump in basis points (additive mode).

    This is the most common bump type for interest rate scenarios. Shifts
    the entire curve by the specified number of basis points.

    Parameters
    ----------
    bump_bp : float
        Bump magnitude in basis points (e.g., 10.0 for 10bp).

    Returns
    -------
    BumpSpec
        Specification for a parallel additive bump in basis points.

    """
    @staticmethod
    def key_rate_bp(time_years: float, bump_bp: float) -> BumpSpec: ...
    """Create a key-rate bump in basis points at a specific tenor.

    Key-rate bumps affect only the specified tenor point on the curve,
    with the impact tapering off for adjacent tenors. Used for DV01
    calculations and partial duration analysis.

    Parameters
    ----------
    time_years : float
        Tenor point in years where the bump is applied (e.g., 5.0 for 5-year).
    bump_bp : float
        Bump magnitude in basis points at the key rate point.

    Returns
    -------
    BumpSpec
        Specification for a key-rate bump.

    """
    @staticmethod
    def multiplier(factor: float) -> BumpSpec: ...
    """Create a multiplicative bump by a factor.

    Multiplies the current value by the factor. For example, factor=1.1
    represents a 10% increase. Commonly used for volatility surfaces.

    Parameters
    ----------
    factor : float
        Multiplicative factor (e.g., 1.1 for 10% increase, 0.9 for 10% decrease).

    Returns
    -------
    BumpSpec
        Specification for a multiplicative bump.

    """
    @staticmethod
    def inflation_shift_pct(bump_pct: float) -> BumpSpec: ...
    """Create an inflation curve shift in percentage points.

    Parameters
    ----------
    bump_pct : float
        Inflation shift in percentage points (e.g., 0.5 for 0.5%).

    Returns
    -------
    BumpSpec
        Specification for an inflation shift.
    """
    @staticmethod
    def correlation_shift_pct(bump_pct: float) -> BumpSpec: ...
    """Create a correlation shift in percentage points.

    Parameters
    ----------
    bump_pct : float
        Correlation shift in percentage points (e.g., 5.0 for 5%).

    Returns
    -------
    BumpSpec
        Specification for a correlation shift.
    """
    @property
    def mode(self) -> BumpMode: ...
    @property
    def units(self) -> BumpUnits: ...
    @property
    def value(self) -> float: ...
    @property
    def bump_type(self) -> BumpType: ...

class MarketBump:
    """Concrete market data shift specification for scenario generation.

    MarketBump defines a specific shift to apply to market data (curves,
    surfaces, FX rates, etc.) when creating scenario variants. Bumps are
    applied to a :class:`MarketContext` via :meth:`MarketContext.apply_bumps`
    to create stressed market environments for risk analysis.

    MarketBump is an abstract specification; the actual bumping logic is
    handled by MarketContext. Multiple bumps can be applied in sequence to
    create complex scenarios.

    Examples
    --------
        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.market_data.bumps import BumpSpec, MarketBump
        >>> from finstack.core.types import CurveId
        >>> bumps = [
        ...     MarketBump.curve(CurveId("USD"), BumpSpec.parallel_bp(10.0)),
        ...     MarketBump.fx_pct(Currency("EUR"), Currency("USD"), 2.0, date(2024, 1, 1)),
        ... ]
        >>> len(bumps)
        2

    Notes
    -----
    - Bumps are applied in the order specified
    - Each bump creates a new MarketContext (original is unchanged)
    - Use :attr:`kind` to inspect the bump type programmatically
    - Bumps can be combined for multi-factor scenarios

    See Also
    --------
    :class:`BumpSpec`: Bump magnitude and type specification
    :meth:`MarketContext.apply_bumps`: Apply bumps to create scenarios
    """

    @classmethod
    def curve(cls, curve_id: CurveId, spec: BumpSpec) -> MarketBump: ...
    """Create a bump for an interest rate or credit curve.

    Applies the specified bump to a discount, forward, hazard, or inflation
    curve identified by curve_id. The bump type (parallel or key-rate)
    determines how the shift is applied across the curve.

    Parameters
    ----------
    curve_id : CurveId
        Identifier of the curve to bump (e.g., "USD", "EUR-LIBOR-3M").
    spec : BumpSpec
        Bump specification defining mode, units, magnitude, and type.

    Returns
    -------
    MarketBump
        Bump specification for the curve.

    """
    @classmethod
    def fx_pct(
        cls,
        base_currency: Currency,
        quote_currency: Currency,
        pct: float,
        as_of: date,
    ) -> MarketBump: ...
    """Create a percentage shift for an FX rate pair.

    Shifts the FX rate between base_currency and quote_currency by the
    specified percentage. Positive values represent appreciation of the
    base currency relative to the quote currency.

    Parameters
    ----------
    base_currency : Currency
        Base currency of the FX pair (e.g., EUR for EUR/USD).
    quote_currency : Currency
        Quote currency of the FX pair (e.g., USD for EUR/USD).
    pct : float
        Percentage shift (e.g., 2.0 for 2% appreciation).
    as_of : date
        Date for which the FX shift applies.

    Returns
    -------
    MarketBump
        Bump specification for the FX rate.

    """
    @classmethod
    def vol_bucket_pct(
        cls,
        surface_id: CurveId,
        pct: float,
        expiries: Sequence[float] | None = ...,
        strikes: Sequence[float] | None = ...,
    ) -> MarketBump: ...
    """Create a percentage shift for a volatility surface bucket.

    Shifts volatility values on a surface by the specified percentage.
    If expiries and/or strikes are provided, only those buckets are shifted;
    otherwise, the entire surface is shifted.

    Parameters
    ----------
    surface_id : CurveId
        Identifier of the volatility surface (e.g., "SPX", "EURUSD").
    pct : float
        Percentage shift (e.g., 5.0 for 5% vol increase).
    expiries : Sequence[float], optional
        Specific expiries (in years) to shift. If None, all expiries.
    strikes : Sequence[float], optional
        Specific strikes (as moneyness or absolute) to shift. If None, all strikes.

    Returns
    -------
    MarketBump
        Bump specification for the volatility surface.

        >>> 
        >>> # Shift specific bucket
        >>> bump = MarketBump.vol_bucket_pct(
        ...     CurveId("SPX"),
        ...     5.0,
        ...     expiries=[0.25, 0.5],
        ...     strikes=[0.9, 1.0, 1.1]
        ... )
    """
    @classmethod
    def base_corr_bucket_pts(
        cls,
        surface_id: CurveId,
        points: float,
        detachments: Sequence[float] | None = ...,
    ) -> MarketBump: ...
    """Create a basis points shift for a base correlation surface.

    Shifts base correlation values by the specified number of basis points.
    Used for structured credit products (CDO, CLO).

    Parameters
    ----------
    surface_id : CurveId
        Identifier of the base correlation surface.
    points : float
        Shift in basis points (e.g., 50.0 for 50bp).
    detachments : Sequence[float], optional
        Specific detachment points to shift. If None, all detachments.

    Returns
    -------
    MarketBump
        Bump specification for the base correlation surface.
    """
    @property
    def kind(self) -> Literal["curve", "fx_pct", "vol_bucket_pct", "base_corr_bucket_pts"]: ...

__all__ = [
    "BumpMode",
    "BumpUnits",
    "BumpType",
    "BumpSpec",
    "MarketBump",
]
