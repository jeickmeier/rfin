"""Range accrual instrument."""

from __future__ import annotations
from datetime import date
from ....core.money import Money
from ....core.dates.daycount import DayCount

class BoundsType:
    """How range bounds are interpreted (absolute price levels or relative to initial spot)."""

    ABSOLUTE: BoundsType
    RELATIVE_TO_INITIAL_SPOT: BoundsType

    @classmethod
    def from_name(cls, name: str) -> BoundsType: ...
    @property
    def name(self) -> str: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...

class RangeAccrualBuilder:
    """Fluent builder returned by :meth:`RangeAccrual.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def ticker(self, ticker: str) -> RangeAccrualBuilder: ...
    def observation_dates(self, dates: list[date]) -> RangeAccrualBuilder: ...
    def lower_bound(self, lower_bound: float) -> RangeAccrualBuilder: ...
    def upper_bound(self, upper_bound: float) -> RangeAccrualBuilder: ...
    def bounds_type(self, bounds_type: BoundsType | str) -> RangeAccrualBuilder: ...
    def coupon_rate(self, coupon_rate: float) -> RangeAccrualBuilder: ...
    def notional(self, notional: Money) -> RangeAccrualBuilder: ...
    def day_count(self, day_count: DayCount | str) -> RangeAccrualBuilder: ...
    def discount_curve(self, discount_curve: str) -> RangeAccrualBuilder: ...
    def spot_id(self, spot_id: str) -> RangeAccrualBuilder: ...
    def vol_surface(self, vol_surface: str) -> RangeAccrualBuilder: ...
    def div_yield_id(self, div_yield_id: str | None = ...) -> RangeAccrualBuilder: ...
    def payment_date(self, payment_date: date) -> RangeAccrualBuilder: ...
    def past_fixings_in_range(self, count: int | None = ...) -> RangeAccrualBuilder: ...
    def total_past_observations(self, count: int | None = ...) -> RangeAccrualBuilder: ...
    def implied_volatility(self, implied_volatility: float | None = ...) -> RangeAccrualBuilder: ...
    def tree_steps(self, tree_steps: int | None = ...) -> RangeAccrualBuilder: ...
    def attributes(self, attributes: dict[str, str] | None = ...) -> RangeAccrualBuilder: ...
    def build(self) -> RangeAccrual: ...

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
    Create a range accrual note using the fluent builder:

        >>> from finstack.valuations.instruments import RangeAccrual
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> observation_dates = [
        ...     date(2024, 1, 15),
        ...     date(2024, 2, 15),
        ...     date(2024, 3, 15),
        ... ]
        >>> range_accrual = (
        ...     RangeAccrual
        ...     .builder("RANGE-ACCRUAL-SPX")
        ...     .ticker("SPX")
        ...     .observation_dates(observation_dates)
        ...     .lower_bound(4000.0)
        ...     .upper_bound(4500.0)
        ...     .coupon_rate(0.08)
        ...     .notional(Money(1_000_000, Currency("USD")))
        ...     .discount_curve("USD")
        ...     .spot_id("SPX")
        ...     .vol_surface("SPX-VOL")
        ...     .build()
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
    def builder(cls, instrument_id: str) -> RangeAccrualBuilder: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def underlying_ticker(self) -> str: ...
    @property
    def lower_bound(self) -> float: ...
    @property
    def upper_bound(self) -> float: ...
    @property
    def coupon_rate(self) -> float: ...
    @property
    def notional(self) -> Money: ...
    @property
    def observation_dates(self) -> list[date]: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
