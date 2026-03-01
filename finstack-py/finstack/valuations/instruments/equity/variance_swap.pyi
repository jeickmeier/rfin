"""Variance swap instrument."""

from __future__ import annotations
from datetime import date
from ....core.currency import Currency
from ....core.money import Money
from ....core.dates.schedule import Frequency
from ....core.dates.daycount import DayCount
from ...common import InstrumentType

class VarianceDirection:
    """Pay/receive wrapper for variance swap payoffs."""

    PAY: "VarianceDirection"
    RECEIVE: "VarianceDirection"

class RealizedVarianceMethod:
    """Realized variance calculation method wrapper."""

    CLOSE_TO_CLOSE: "RealizedVarianceMethod"
    PARKINSON: "RealizedVarianceMethod"
    GARMAN_KLASS: "RealizedVarianceMethod"
    ROGERS_SATCHELL: "RealizedVarianceMethod"
    YANG_ZHANG: "RealizedVarianceMethod"

class VarianceSwapBuilder:
    """Fluent builder returned by :meth:`VarianceSwap.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def underlying_id(self, underlying_id: str) -> VarianceSwapBuilder: ...
    def notional(self, amount: float) -> VarianceSwapBuilder: ...
    def currency(self, currency: str | Currency) -> VarianceSwapBuilder: ...
    def money(self, money: Money) -> VarianceSwapBuilder: ...
    def strike_variance(self, strike_variance: float) -> VarianceSwapBuilder: ...
    def start_date(self, start_date: date) -> VarianceSwapBuilder: ...
    def maturity(self, maturity: date) -> VarianceSwapBuilder: ...
    def disc_id(self, curve_id: str) -> VarianceSwapBuilder: ...
    def observation_frequency(self, observation_frequency: Frequency) -> VarianceSwapBuilder: ...
    def realized_method(self, realized_method: RealizedVarianceMethod | None = ...) -> VarianceSwapBuilder: ...
    def side(self, side: VarianceDirection | str | None = ...) -> VarianceSwapBuilder: ...
    def day_count(self, day_count: DayCount) -> VarianceSwapBuilder: ...
    def build(self) -> "VarianceSwap": ...

class VarianceSwap:
    """Variance swap for volatility trading.

    VarianceSwap represents a swap where one party pays realized variance
    and receives strike variance (or vice versa). Variance swaps allow direct
    trading of volatility without delta hedging.

    Variance swaps are used for volatility trading, hedging vega exposure,
    and creating volatility strategies. They require discount curves and
    underlying price observations.

    Examples
    --------
    Create a variance swap:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.dates.daycount import DayCount
        >>> from finstack.core.dates.schedule import Frequency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import (
        ...     VarianceDirection,
        ...     VarianceSwap,
        ...     RealizedVarianceMethod,
        ... )
        >>> variance_swap = (
        ...     VarianceSwap
        ...     .builder("VAR-SWAP-SPX")
        ...     .underlying_id("SPX")
        ...     .money(Money(1_000_000, Currency("USD")))
        ...     .strike_variance(0.04)  # 20% vol squared (0.20^2)
        ...     .start_date(date(2024, 1, 1))
        ...     .maturity(date(2024, 12, 31))  # 1-year swap
        ...     .disc_id("USD-OIS")
        ...     .observation_frequency(Frequency.DAILY)
        ...     .realized_method(RealizedVarianceMethod.CLOSE_TO_CLOSE)
        ...     .side(VarianceDirection.RECEIVE)  # Receive realized, pay strike
        ...     .day_count(DayCount.ACT_365F)
        ...     .build()
        ... )

    Notes
    -----
    - Variance swaps require discount curve and underlying price observations
    - Strike variance is the fixed variance rate (volatility squared)
    - Realized variance is calculated from price observations
    - Observation frequency determines how often prices are sampled
    - Realized method determines variance calculation formula

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required).
    - Underlying price observations: referenced by ``underlying_id`` (required).

    See Also
    --------
    :class:`EquityOption`: Equity options (implied volatility)
    :class:`VolSurface`: Volatility surfaces
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - Demeterfi et al. (1999): see ``docs/REFERENCES.md#demeterfiVarianceSwaps1999``.
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> VarianceSwapBuilder: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    @property
    def strike_variance(self) -> float: ...
    @property
    def observation_frequency(self) -> str: ...
    @property
    def realized_method(self) -> str: ...
    @property
    def side(self) -> str: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
