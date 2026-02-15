"""Interest rate future instrument (builder-only API)."""

from typing import Optional, Union
from datetime import date
from ...core.currency import Currency
from ...core.money import Money
from ...core.dates.daycount import DayCount
from ..common import InstrumentType

class InterestRateFutureBuilder:
    """Fluent builder returned by :meth:`InterestRateFuture.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def notional(self, amount: float) -> InterestRateFutureBuilder: ...
    def currency(self, currency: Union[str, Currency]) -> InterestRateFutureBuilder: ...
    def money(self, money: Money) -> InterestRateFutureBuilder: ...
    def quoted_price(self, quoted_price: float) -> InterestRateFutureBuilder: ...
    def expiry(self, expiry: date) -> InterestRateFutureBuilder: ...
    def fixing_date(self, fixing_date: date) -> InterestRateFutureBuilder: ...
    def period_start(self, period_start: date) -> InterestRateFutureBuilder: ...
    def period_end(self, period_end: date) -> InterestRateFutureBuilder: ...
    def disc_id(self, curve_id: str) -> InterestRateFutureBuilder: ...
    def fwd_id(self, curve_id: str) -> InterestRateFutureBuilder: ...
    def position(self, position: Optional[str] = ...) -> InterestRateFutureBuilder: ...
    def day_count(self, day_count: Union[DayCount, str]) -> InterestRateFutureBuilder: ...
    def face_value(self, face_value: float) -> InterestRateFutureBuilder: ...
    def tick_size(self, tick_size: float) -> InterestRateFutureBuilder: ...
    def tick_value(self, tick_value: Optional[float] = ...) -> InterestRateFutureBuilder: ...
    def delivery_months(self, delivery_months: int) -> InterestRateFutureBuilder: ...
    def convexity_adjustment(self, convexity_adjustment: Optional[float] = ...) -> InterestRateFutureBuilder: ...
    def build(self) -> "InterestRateFuture": ...

class InterestRateFuture:
    """Interest rate future for hedging and speculation on future rates.

    InterestRateFuture represents a futures contract on an interest rate,
    typically based on a 3-month rate (e.g., Eurodollar, SOFR futures).
    The future price is quoted as 100 minus the implied rate.

    Interest rate futures are used for hedging interest rate risk, speculating
    on rate movements, and creating synthetic positions. They require discount
    and forward curves for pricing.

    Examples
    --------
    Create an interest rate future:

        >>> from finstack.valuations.instruments import InterestRateFuture
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> future = (
        ...     InterestRateFuture.builder("IR-FUTURE-DEC24")
        ...     .money(Money(1_000_000, Currency("USD")))
        ...     .quoted_price(96.50)  # Implies 3.5% rate (100 - 96.50)
        ...     .expiry(date(2024, 12, 15))
        ...     # Optional dates below are inferred if omitted:
        ...     # fixing_date = expiry, period_start = fixing + 2d,
        ...     # period_end = period_start + delivery_months
        ...     .fixing_date(date(2024, 12, 16))
        ...     .period_start(date(2024, 12, 18))
        ...     .period_end(date(2025, 3, 18))
        ...     .disc_id("USD")
        ...     .fwd_id("USD-LIBOR-3M")
        ...     .build()
        ... )

    Notes
    -----
    - Interest rate futures require discount curve and forward curve
    - Quoted price = 100 - implied rate (e.g., 96.50 = 3.5% rate)
    - Face value is the contract size (typically $1M for Eurodollars)
    - Tick size is the minimum price movement (typically 0.0025 = 1bp)
    - Convexity adjustment accounts for futures vs forward rate differences
    - Position: "long" (default) or "short"

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required).
    - Forward curve: ``forward_curve`` (required).

    See Also
    --------
    :class:`ForwardRateAgreement`: Forward rate agreements
    :class:`InterestRateSwap`: Interest rate swaps
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    - Brigo & Mercurio (2006): see ``docs/REFERENCES.md#brigoMercurio2006``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> InterestRateFutureBuilder: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def quoted_price(self) -> float: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
