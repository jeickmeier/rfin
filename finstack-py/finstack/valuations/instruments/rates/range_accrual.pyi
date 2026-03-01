"""Range accrual instrument."""

from __future__ import annotations
from typing import List
from datetime import date
from ....core.money import Money

class RangeAccrual:
    """Range accrual note with conditional coupon payments.

    RangeAccrual represents a structured product that pays a coupon only when
    the underlying asset price stays within a specified range on observation
    dates. The coupon accrues based on the number of days the price is in range.

    Range accruals are used in structured products to provide enhanced yields
    with conditional payments. They require discount curves, spot prices, and
    volatility surfaces.

    Examples
    --------
    Create a range accrual note:

        >>> from finstack.valuations.instruments import RangeAccrual
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> observation_dates = [
        ...     date(2024, 1, 15),
        ...     date(2024, 2, 15),
        ...     date(2024, 3, 15),
        ...     date(2024, 4, 15),
        ...     date(2024, 5, 15),
        ...     date(2024, 6, 15),
        ... ]
        >>> range_accrual = RangeAccrual.builder(
        ...     "RANGE-ACCRUAL-SPX",
        ...     ticker="SPX",
        ...     observation_dates=observation_dates,
        ...     lower_bound=4000.0,  # Lower range bound
        ...     upper_bound=4500.0,  # Upper range bound
        ...     coupon_rate=0.08,  # 8% coupon if in range
        ...     notional=Money(1_000_000, Currency("USD")),
        ...     discount_curve="USD",
        ...     spot_id="SPX",
        ...     vol_surface="SPX-VOL",
        ...     div_yield_id=None,
        ... )

    Notes
    -----
    - Range accruals require discount curve, spot price, and volatility surface
    - Coupon is paid only when underlying is within range on observation dates
    - Accrual is proportional to number of days in range
    - Lower and upper bounds define the range
    - Higher coupon rates compensate for conditional payment risk

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required).
    - Spot price: ``spot_id`` (required).
    - Volatility surface: ``vol_surface`` (required).
    - Dividend yield: ``div_yield_id`` (optional; used when provided).

    See Also
    --------
    :class:`Bond`: Standard bonds
    :class:`Autocallable`: Autocallable structured products
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    """

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        ticker: str,
        observation_dates: List[date],
        lower_bound: float,
        upper_bound: float,
        coupon_rate: float,
        notional: Money,
        discount_curve: str,
        spot_id: str,
        vol_surface: str,
        *,
        div_yield_id: str | None = None,
    ) -> "RangeAccrual":
        """Create a range accrual instrument.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the instrument (e.g., "RANGE-ACCRUAL-SPX").
        ticker : str
            Underlying equity ticker symbol.
        observation_dates : List[date]
            Dates when underlying price is observed to determine if in range.
            Must be in ascending order.
        lower_bound : float
            Lower bound of the range. Must be > 0 and < upper_bound.
        upper_bound : float
            Upper bound of the range. Must be > lower_bound.
        coupon_rate : float
            Coupon rate as a decimal (e.g., 0.08 for 8%). Paid when price is in range.
        notional : Money
            Notional principal amount.
        discount_curve : str
            Discount curve identifier in MarketContext.
        spot_id : str
            Spot price identifier in MarketContext.
        vol_surface : str
            Volatility surface identifier in MarketContext.
        div_yield_id : str, optional
            Dividend yield identifier in MarketContext.

        Returns
        -------
        RangeAccrual
            Configured range accrual ready for pricing.

        Raises
        ------
        ValueError
            If parameters are invalid (upper_bound <= lower_bound, observation_dates
            out of order, etc.) or if required market data is missing.

        Examples
        --------
            >>> range_accrual = RangeAccrual.builder(
            ...     "RANGE-ACCRUAL-SPX",
            ...     "SPX",
            ...     observation_dates,
            ...     lower_bound=4000.0,
            ...     upper_bound=4500.0,
            ...     coupon_rate=0.08,
            ...     notional=Money(1_000_000, Currency("USD")),
            ...     discount_curve="USD",
            ...     spot_id="SPX",
            ...     vol_surface="SPX-VOL",
            ... )
        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def ticker(self) -> str: ...
    @property
    def lower_bound(self) -> float: ...
    @property
    def upper_bound(self) -> float: ...
    @property
    def coupon_rate(self) -> float: ...
    @property
    def notional(self) -> Money: ...
    @property
    def observation_dates(self) -> List[date]: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
