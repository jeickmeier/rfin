"""Market data bindings from ``finstack-core``: curves, FX, and market context.

Provides term-structure curve types (discount, forward, hazard, price,
volatility index), FX rate matrix, and the unified :class:`MarketContext`
container.

Example::

    >>> import datetime
    >>> from finstack.core.market_data import DiscountCurve
    >>> curve = DiscountCurve(
    ...     id="USD-OIS",
    ...     base_date=datetime.date(2024, 1, 1),
    ...     knots=[(0.25, 0.99), (0.5, 0.98), (1.0, 0.96)],
    ... )
    >>> curve.df(0.5)
    0.98
"""

from __future__ import annotations

import datetime
from typing import Optional, Union

from finstack.core.currency import Currency

__all__ = [
    # curves
    "DiscountCurve",
    "ForwardCurve",
    "HazardCurve",
    "PriceCurve",
    "VolatilityIndexCurve",
    # fx
    "FxConversionPolicy",
    "FxRateResult",
    "FxMatrix",
    # context
    "MarketContext",
]

# ---------------------------------------------------------------------------
# Curves
# ---------------------------------------------------------------------------

class DiscountCurve:
    """Discount factor curve for present-value calculations.

    Constructed from ``(time, discount_factor)`` knot pairs with configurable
    interpolation and extrapolation.

    Parameters
    ----------
    id : str
        Unique curve identifier (e.g. ``"USD-OIS"``).
    base_date : datetime.date
        Valuation date.
    knots : list[tuple[float, float]]
        ``(time_years, discount_factor)`` pairs.
    interp : str
        Interpolation style (default ``"monotone_convex"``).
    extrapolation : str
        Extrapolation policy (default ``"flat_forward"``).
    day_count : str
        Day-count convention (default ``"act_365f"``).

    Raises
    ------
    ValueError
        If the curve cannot be built from the given parameters.

    Examples
    --------
    >>> import datetime
    >>> from finstack.core.market_data import DiscountCurve
    >>> dc = DiscountCurve(
    ...     id="USD-OIS",
    ...     base_date=datetime.date(2024, 1, 1),
    ...     knots=[(0.0, 1.0), (1.0, 0.96), (5.0, 0.82)],
    ... )
    >>> dc.df(1.0)
    0.96
    >>> dc.zero(1.0)  # continuously-compounded zero rate
    0.040821994520255166
    """

    def __init__(
        self,
        id: str,
        base_date: datetime.date,
        knots: list[tuple[float, float]],
        interp: str = "monotone_convex",
        extrapolation: str = "flat_forward",
        day_count: str = "act_365f",
    ) -> None:
        """Construct a discount curve from knot points.

        Parameters
        ----------
        id : str
            Unique curve identifier.
        base_date : datetime.date
            Valuation date.
        knots : list[tuple[float, float]]
            ``(time_years, discount_factor)`` pairs.
        interp : str
            Interpolation style (default ``"monotone_convex"``).
        extrapolation : str
            Extrapolation policy (default ``"flat_forward"``).
        day_count : str
            Day-count convention (default ``"act_365f"``).

        Raises
        ------
        ValueError
            If the curve cannot be built.
        """
        ...

    def df(self, t: float) -> float:
        """Discount factor at year fraction *t*.

        Parameters
        ----------
        t : float
            Time in year fractions from the base date.

        Returns
        -------
        float
            Discount factor.
        """
        ...

    def zero(self, t: float) -> float:
        """Continuously-compounded zero rate at year fraction *t*.

        Parameters
        ----------
        t : float
            Time in year fractions.

        Returns
        -------
        float
            Zero rate.
        """
        ...

    def forward_rate(self, t1: float, t2: float) -> float:
        """Continuously-compounded forward rate between *t1* and *t2*.

        Parameters
        ----------
        t1 : float
            Start time in year fractions.
        t2 : float
            End time in year fractions.

        Returns
        -------
        float
            Forward rate.

        Raises
        ------
        ValueError
            If *t1* >= *t2*.
        """
        ...

    @property
    def id(self) -> str:
        """Curve identifier string.

        Returns
        -------
        str
        """
        ...

    @property
    def base_date(self) -> datetime.date:
        """Valuation base date.

        Returns
        -------
        datetime.date
        """
        ...

    def __repr__(self) -> str: ...

class ForwardCurve:
    """Forward rate curve for a floating-rate index with a fixed tenor.

    Constructed from ``(time, forward_rate)`` knot pairs.

    Parameters
    ----------
    id : str
        Unique curve identifier (e.g. ``"USD-SOFR-3M"``).
    tenor : float
        Index tenor in years (e.g. ``0.25`` for 3 months).
    knots : list[tuple[float, float]]
        ``(time_years, forward_rate)`` pairs.
    base_date : datetime.date
        Valuation date.
    day_count : str
        Day-count convention (default ``"act_360"``).
    interp : str
        Interpolation style (default ``"linear"``).
    extrapolation : str
        Extrapolation policy (default ``"flat_forward"``).

    Raises
    ------
    ValueError
        If the curve cannot be built from the given parameters.
    """

    def __init__(
        self,
        id: str,
        tenor: float,
        knots: list[tuple[float, float]],
        base_date: datetime.date,
        day_count: str = "act_360",
        interp: str = "linear",
        extrapolation: str = "flat_forward",
    ) -> None:
        """Construct a forward rate curve from knot points.

        Parameters
        ----------
        id : str
            Unique curve identifier.
        tenor : float
            Index tenor in years.
        knots : list[tuple[float, float]]
            ``(time_years, forward_rate)`` pairs.
        base_date : datetime.date
            Valuation date.
        day_count : str
            Day-count convention (default ``"act_360"``).
        interp : str
            Interpolation style (default ``"linear"``).
        extrapolation : str
            Extrapolation policy (default ``"flat_forward"``).

        Raises
        ------
        ValueError
            If the curve cannot be built.
        """
        ...

    def rate(self, t: float) -> float:
        """Forward rate at year fraction *t*.

        Parameters
        ----------
        t : float
            Time in year fractions.

        Returns
        -------
        float
            Forward rate.
        """
        ...

    @property
    def id(self) -> str:
        """Curve identifier string.

        Returns
        -------
        str
        """
        ...

    @property
    def base_date(self) -> datetime.date:
        """Valuation base date.

        Returns
        -------
        datetime.date
        """
        ...

    def __repr__(self) -> str: ...

class HazardCurve:
    """Credit hazard-rate curve for default probability modeling.

    Constructed from ``(time, hazard_rate)`` knot pairs.

    Parameters
    ----------
    id : str
        Unique curve identifier (e.g. ``"ACME-HZD"``).
    base_date : datetime.date
        Valuation date.
    knots : list[tuple[float, float]]
        ``(time_years, hazard_rate)`` pairs.
    recovery_rate : float
        Recovery rate (default ``0.4``).
    day_count : str
        Day-count convention (default ``"act_365f"``).

    Raises
    ------
    ValueError
        If the curve cannot be built from the given parameters.
    """

    def __init__(
        self,
        id: str,
        base_date: datetime.date,
        knots: list[tuple[float, float]],
        recovery_rate: float = 0.4,
        day_count: str = "act_365f",
    ) -> None:
        """Construct a hazard curve from knot points.

        Parameters
        ----------
        id : str
            Unique curve identifier.
        base_date : datetime.date
            Valuation date.
        knots : list[tuple[float, float]]
            ``(time_years, hazard_rate)`` pairs.
        recovery_rate : float
            Recovery rate (default ``0.4``).
        day_count : str
            Day-count convention (default ``"act_365f"``).

        Raises
        ------
        ValueError
            If the curve cannot be built.
        """
        ...

    def survival(self, t: float) -> float:
        """Survival probability at year fraction *t*.

        Parameters
        ----------
        t : float
            Time in year fractions.

        Returns
        -------
        float
            Survival probability in ``[0, 1]``.
        """
        ...

    def hazard_rate(self, t: float) -> float:
        """Instantaneous hazard rate at year fraction *t*.

        Parameters
        ----------
        t : float
            Time in year fractions.

        Returns
        -------
        float
            Hazard rate.
        """
        ...

    @property
    def id(self) -> str:
        """Curve identifier string.

        Returns
        -------
        str
        """
        ...

    @property
    def base_date(self) -> datetime.date:
        """Valuation base date.

        Returns
        -------
        datetime.date
        """
        ...

    def __repr__(self) -> str: ...

class PriceCurve:
    """Forward price curve for commodities and other price-based assets.

    Constructed from ``(time, forward_price)`` knot pairs.

    Parameters
    ----------
    id : str
        Unique curve identifier (e.g. ``"WTI-FORWARD"``).
    base_date : datetime.date
        Valuation date.
    knots : list[tuple[float, float]]
        ``(time_years, forward_price)`` pairs.
    extrapolation : str
        Extrapolation policy (default ``"flat_zero"``).
    interp : str
        Interpolation style (default ``"linear"``).
    day_count : str
        Day-count convention (default ``"act_365f"``).

    Raises
    ------
    ValueError
        If the curve cannot be built from the given parameters.
    """

    def __init__(
        self,
        id: str,
        base_date: datetime.date,
        knots: list[tuple[float, float]],
        extrapolation: str = "flat_zero",
        interp: str = "linear",
        day_count: str = "act_365f",
    ) -> None:
        """Construct a price curve from knot points.

        Parameters
        ----------
        id : str
            Unique curve identifier.
        base_date : datetime.date
            Valuation date.
        knots : list[tuple[float, float]]
            ``(time_years, forward_price)`` pairs.
        extrapolation : str
            Extrapolation policy (default ``"flat_zero"``).
        interp : str
            Interpolation style (default ``"linear"``).
        day_count : str
            Day-count convention (default ``"act_365f"``).

        Raises
        ------
        ValueError
            If the curve cannot be built.
        """
        ...

    def price(self, t: float) -> float:
        """Forward price at year fraction *t*.

        Parameters
        ----------
        t : float
            Time in year fractions.

        Returns
        -------
        float
            Forward price.
        """
        ...

    @property
    def id(self) -> str:
        """Curve identifier string.

        Returns
        -------
        str
        """
        ...

    @property
    def base_date(self) -> datetime.date:
        """Valuation base date.

        Returns
        -------
        datetime.date
        """
        ...

    def __repr__(self) -> str: ...

class VolatilityIndexCurve:
    """Volatility index forward curve (e.g. VIX term structure).

    Constructed from ``(time, forward_level)`` knot pairs.

    Parameters
    ----------
    id : str
        Unique curve identifier (e.g. ``"VIX"``).
    base_date : datetime.date
        Valuation date.
    knots : list[tuple[float, float]]
        ``(time_years, forward_level)`` pairs.
    extrapolation : str
        Extrapolation policy (default ``"flat_zero"``).
    interp : str
        Interpolation style (default ``"linear"``).
    day_count : str
        Day-count convention (default ``"act_365f"``).

    Raises
    ------
    ValueError
        If the curve cannot be built from the given parameters.
    """

    def __init__(
        self,
        id: str,
        base_date: datetime.date,
        knots: list[tuple[float, float]],
        extrapolation: str = "flat_zero",
        interp: str = "linear",
        day_count: str = "act_365f",
    ) -> None:
        """Construct a volatility index curve from knot points.

        Parameters
        ----------
        id : str
            Unique curve identifier.
        base_date : datetime.date
            Valuation date.
        knots : list[tuple[float, float]]
            ``(time_years, forward_level)`` pairs.
        extrapolation : str
            Extrapolation policy (default ``"flat_zero"``).
        interp : str
            Interpolation style (default ``"linear"``).
        day_count : str
            Day-count convention (default ``"act_365f"``).

        Raises
        ------
        ValueError
            If the curve cannot be built.
        """
        ...

    def forward_level(self, t: float) -> float:
        """Forward volatility index level at year fraction *t*.

        Parameters
        ----------
        t : float
            Time in year fractions.

        Returns
        -------
        float
            Forward volatility index level.
        """
        ...

    @property
    def id(self) -> str:
        """Curve identifier string.

        Returns
        -------
        str
        """
        ...

    @property
    def base_date(self) -> datetime.date:
        """Valuation base date.

        Returns
        -------
        datetime.date
        """
        ...

    def __repr__(self) -> str: ...

# ---------------------------------------------------------------------------
# FX
# ---------------------------------------------------------------------------

class FxConversionPolicy:
    """FX conversion policy controlling when rates are sampled.

    Immutable enum-style type with class-level constants.
    """

    CASHFLOW_DATE: FxConversionPolicy
    """Use spot/forward on the cashflow date."""
    PERIOD_END: FxConversionPolicy
    """Use period end date."""
    PERIOD_AVERAGE: FxConversionPolicy
    """Use an average over the period."""
    CUSTOM: FxConversionPolicy
    """Custom strategy defined by the caller."""

    @classmethod
    def from_name(cls, name: str) -> FxConversionPolicy:
        """Parse from a string label.

        Parameters
        ----------
        name : str
            Policy label (e.g. ``"cashflow_date"``, ``"period_end"``).

        Returns
        -------
        FxConversionPolicy

        Raises
        ------
        ValueError
            If *name* is not recognised.
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class FxRateResult:
    """Result of an FX rate query.

    Immutable value type returned by :meth:`FxMatrix.rate`.
    """

    @property
    def rate(self) -> float:
        """The FX conversion rate.

        Returns
        -------
        float
        """
        ...

    @property
    def triangulated(self) -> bool:
        """Whether the rate was obtained via triangulation.

        Returns
        -------
        bool
        """
        ...

    def __repr__(self) -> str: ...

class FxMatrix:
    """Foreign-exchange rate matrix for currency conversion.

    Manages explicit FX quotes and supports rate lookup with optional
    triangulation.
    """

    def __init__(self) -> None:
        """Create an empty FX matrix."""
        ...

    def set_quote(
        self,
        base: Union[Currency, str],
        quote: Union[Currency, str],
        rate: float,
    ) -> None:
        """Set an explicit FX quote.

        Parameters
        ----------
        base : Currency | str
            Base (from) currency.
        quote : Currency | str
            Quote (to) currency.
        rate : float
            The conversion rate (``1 base = rate quote``).

        Raises
        ------
        ValueError
            If a currency code is invalid or rate is non-finite.
        """
        ...

    def rate(
        self,
        base: Union[Currency, str],
        quote: Union[Currency, str],
        date: datetime.date,
        policy: Optional[Union[FxConversionPolicy, str]] = None,
    ) -> FxRateResult:
        """Look up an FX rate.

        Parameters
        ----------
        base : Currency | str
            Base (from) currency.
        quote : Currency | str
            Quote (to) currency.
        date : datetime.date
            Applicable date for the rate.
        policy : FxConversionPolicy | str | None
            Conversion policy (default ``"cashflow_date"``).

        Returns
        -------
        FxRateResult
            The looked-up rate with metadata.

        Raises
        ------
        ValueError
            If the rate cannot be determined.
        """
        ...

    def __repr__(self) -> str: ...

# ---------------------------------------------------------------------------
# Market context
# ---------------------------------------------------------------------------

class MarketContext:
    """Unified market data container for curves, surfaces, and FX.

    Provides a single access point for all market data required by
    pricing and analytics functions. Curves are stored behind ``Arc``
    and the context is cheap to clone.
    """

    def __init__(self) -> None:
        """Create an empty market context."""
        ...

    def insert(
        self,
        curve: Union[
            DiscountCurve,
            ForwardCurve,
            HazardCurve,
            PriceCurve,
            VolatilityIndexCurve,
        ],
    ) -> MarketContext:
        """Insert a curve into the context (fluent, returns ``self``).

        Accepts any curve type: :class:`DiscountCurve`, :class:`ForwardCurve`,
        :class:`HazardCurve`, :class:`PriceCurve`, or
        :class:`VolatilityIndexCurve`.

        Parameters
        ----------
        curve : DiscountCurve | ForwardCurve | HazardCurve | PriceCurve | VolatilityIndexCurve
            The curve to insert.

        Returns
        -------
        MarketContext
            ``self`` for method chaining.

        Raises
        ------
        TypeError
            If *curve* is not a supported curve type.
        """
        ...

    def insert_fx(self, fx: FxMatrix) -> None:
        """Insert an FX matrix into the context.

        Parameters
        ----------
        fx : FxMatrix
            FX rate matrix.
        """
        ...

    def get_discount(self, id: str) -> DiscountCurve:
        """Retrieve a discount curve by identifier.

        Parameters
        ----------
        id : str
            Curve identifier.

        Returns
        -------
        DiscountCurve

        Raises
        ------
        ValueError
            If no discount curve with this *id* exists.
        """
        ...

    def get_forward(self, id: str) -> ForwardCurve:
        """Retrieve a forward curve by identifier.

        Parameters
        ----------
        id : str
            Curve identifier.

        Returns
        -------
        ForwardCurve

        Raises
        ------
        ValueError
            If no forward curve with this *id* exists.
        """
        ...

    def get_hazard(self, id: str) -> HazardCurve:
        """Retrieve a hazard curve by identifier.

        Parameters
        ----------
        id : str
            Curve identifier.

        Returns
        -------
        HazardCurve

        Raises
        ------
        ValueError
            If no hazard curve with this *id* exists.
        """
        ...

    def get_price_curve(self, id: str) -> PriceCurve:
        """Retrieve a price curve by identifier.

        Parameters
        ----------
        id : str
            Curve identifier.

        Returns
        -------
        PriceCurve

        Raises
        ------
        ValueError
            If no price curve with this *id* exists.
        """
        ...

    def get_vol_index_curve(self, id: str) -> VolatilityIndexCurve:
        """Retrieve a volatility index curve by identifier.

        Parameters
        ----------
        id : str
            Curve identifier.

        Returns
        -------
        VolatilityIndexCurve

        Raises
        ------
        ValueError
            If no vol-index curve with this *id* exists.
        """
        ...

    @property
    def fx(self) -> Optional[FxMatrix]:
        """Access the FX matrix (returns ``None`` if not set).

        Returns
        -------
        FxMatrix | None
        """
        ...

    def __repr__(self) -> str: ...
