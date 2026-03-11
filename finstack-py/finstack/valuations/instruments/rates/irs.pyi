"""Interest rate swap instrument."""

from __future__ import annotations
from datetime import date
from ....core.money import Money
from ....core.currency import Currency
from ....core.dates.schedule import Frequency, StubKind
from ....core.dates.daycount import DayCount
from ....core.dates.calendar import BusinessDayConvention
from ...common import InstrumentType

class PayReceive:
    """Pay/receive direction for swap fixed-leg cashflows."""

    PAY_FIXED: "PayReceive"
    RECEIVE_FIXED: "PayReceive"

    @classmethod
    def from_name(cls, name: str) -> "PayReceive": ...
    @property
    def name(self) -> str: ...

class FloatingLegCompounding:
    """Compounding convention for floating rate legs."""

    SIMPLE: FloatingLegCompounding
    SOFR: FloatingLegCompounding
    SONIA: FloatingLegCompounding
    ESTR: FloatingLegCompounding
    TONA: FloatingLegCompounding
    FEDFUNDS: FloatingLegCompounding

    @classmethod
    def from_name(cls, name: str) -> FloatingLegCompounding: ...
    @staticmethod
    def compounded_in_arrears(lookback_days: int, observation_shift: int | None = None) -> FloatingLegCompounding: ...
    @property
    def name(self) -> str: ...

class ParRateMethod:
    """Method for calculating par rates in swaps."""

    FORWARD_BASED: ParRateMethod
    DISCOUNT_RATIO: ParRateMethod

    @classmethod
    def from_name(cls, name: str) -> ParRateMethod: ...
    @property
    def name(self) -> str: ...

class InterestRateSwap:
    """Plain-vanilla interest rate swap with fixed-for-floating legs.

    InterestRateSwap represents a standard interest rate swap where one
    party pays a fixed rate and receives a floating rate (or vice versa)
    on a specified notional over a given term. Swaps are priced using
    discount curves and forward curves stored in a MarketContext.

    Swaps are the most liquid interest rate derivatives and are used for
    hedging, speculation, and asset-liability management. The fixed rate
    is typically set such that the swap has zero value at inception (par swap).

    Examples
    --------
    Create an interest rate swap (pay fixed, receive floating):

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import InterestRateSwap
        >>> swap = (
        ...     InterestRateSwap
        ...     .builder("SWAP-001")
        ...     .money(Money(10_000_000, Currency("USD")))
        ...     .side(PayReceive.PAY_FIXED)
        ...     .fixed_rate(0.035)
        ...     .start(date(2024, 1, 1))
        ...     .maturity(date(2029, 1, 1))
        ...     .discount_curve("USD-OIS")
        ...     .forward_curve("USD-SOFR-3M")
        ...     .build()
        ... )

    Price the swap:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.term_structures import DiscountCurve, ForwardCurve
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import InterestRateSwap
        >>> from finstack.valuations.pricer import create_standard_registry
        >>> swap = (
        ...     InterestRateSwap
        ...     .builder("SWAP-EXAMPLE")
        ...     .money(Money(5_000_000, Currency("USD")))
        ...     .side(PayReceive.PAY_FIXED)
        ...     .fixed_rate(0.03)
        ...     .start(date(2024, 1, 1))
        ...     .maturity(date(2029, 1, 1))
        ...     .discount_curve("USD-OIS")
        ...     .forward_curve("USD-SOFR-3M")
        ...     .build()
        ... )
        >>> ctx = MarketContext()
        >>> ctx.insert(DiscountCurve("USD-OIS", date(2024, 1, 1), [(0.0, 1.0), (5.0, 0.95)]))
        >>> ctx.insert(ForwardCurve("USD-SOFR-3M", 0.25, [(0.0, 0.03), (5.0, 0.032)], base_date=date(2024, 1, 1)))
        >>> registry = create_standard_registry()
        >>> pv = registry.price(swap, "discounting", ctx, date(2024, 1, 1)).value
        >>> pv.currency.code
        'USD'

    Notes
    -----
    - Swaps require both discount and forward curves in MarketContext
    - Fixed leg uses the specified fixed_rate
    - Floating leg uses forward rates from the forward_curve plus any spread
    - Use :meth:`builder` for non-USD swaps or custom conventions
    - Par swap rate can be calculated by solving for fixed_rate = 0 PV

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required).
    - Forward curve: ``forward_curve`` (required for the floating leg).

    See Also
    --------
    :class:`Bond`: Fixed-income bond instruments
    :class:`PricerRegistry`: Pricing entry point
    :class:`MarketContext`: Market data container

    Sources
    -------
    - ISDA (2006) Definitions: see ``docs/REFERENCES.md#isda2006Definitions``.
    - Brigo & Mercurio (2006): see ``docs/REFERENCES.md#brigoMercurio2006``.
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> "InterestRateSwapBuilder":
        """Start a fluent builder (builder-only API)."""
        ...

    @property
    def id(self) -> str: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def side(self) -> PayReceive: ...
    @property
    def fixed_rate(self) -> float: ...
    @property
    def float_spread_bp(self) -> float: ...
    @property
    def start(self) -> date: ...
    @property
    def end(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def forward_curve(self) -> str: ...
    @property
    def compounding(self) -> FloatingLegCompounding: ...
    @property
    def payment_lag_days(self) -> int: ...
    @property
    def end_of_month(self) -> bool: ...
    @property
    def fixing_calendar(self) -> str | None: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...

class InterestRateSwapBuilder:
    """Fluent builder returned by :meth:`InterestRateSwap.builder` when only an ID is provided."""

    def __init__(self, instrument_id: str) -> None: ...
    def notional(self, amount: float) -> InterestRateSwapBuilder: ...
    def currency(self, currency: str | Currency) -> InterestRateSwapBuilder: ...
    def money(self, money: Money) -> InterestRateSwapBuilder: ...
    def side(self, side: PayReceive | str) -> InterestRateSwapBuilder: ...
    def fixed_rate(self, rate: float) -> InterestRateSwapBuilder: ...
    def float_spread_bp(self, spread_bp: float) -> InterestRateSwapBuilder: ...
    def start(self, start: date) -> InterestRateSwapBuilder: ...
    def maturity(self, maturity: date) -> InterestRateSwapBuilder: ...
    def discount_curve(self, curve_id: str) -> InterestRateSwapBuilder: ...
    def disc_id(self, curve_id: str) -> InterestRateSwapBuilder:
        """Deprecated: use ``discount_curve()`` instead."""
        ...
    def forward_curve(self, curve_id: str) -> InterestRateSwapBuilder: ...
    def fwd_id(self, curve_id: str) -> InterestRateSwapBuilder:
        """Deprecated: use ``forward_curve()`` instead."""
        ...
    def fixed_frequency(self, frequency: Frequency | str | int) -> InterestRateSwapBuilder: ...
    def float_frequency(self, frequency: Frequency | str | int) -> InterestRateSwapBuilder: ...
    def frequency(self, frequency: Frequency | str | int) -> InterestRateSwapBuilder: ...
    def fixed_day_count(self, day_count: DayCount | str) -> InterestRateSwapBuilder: ...
    def float_day_count(self, day_count: DayCount | str) -> InterestRateSwapBuilder: ...
    def bdc(self, bdc: BusinessDayConvention | str) -> InterestRateSwapBuilder: ...
    def stub(self, stub: StubKind | str) -> InterestRateSwapBuilder: ...
    def calendar(self, calendar_id: str | None = ...) -> InterestRateSwapBuilder: ...
    def reset_lag_days(self, days: int) -> InterestRateSwapBuilder: ...
    def compounding(self, compounding: FloatingLegCompounding) -> InterestRateSwapBuilder: ...
    def par_method(self, method: ParRateMethod) -> InterestRateSwapBuilder: ...
    def fixing_calendar(self, calendar_id: str) -> InterestRateSwapBuilder: ...
    def payment_lag_days(self, days: int) -> InterestRateSwapBuilder: ...
    def end_of_month(self, eom: bool) -> InterestRateSwapBuilder: ...
    def attributes(self, attributes: dict[str, str] | None = ...) -> InterestRateSwapBuilder: ...
    def build(self) -> InterestRateSwap: ...
    def __repr__(self) -> str: ...
