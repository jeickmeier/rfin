"""Basis swap instrument (builder-only API)."""

from typing import Optional, Union
from datetime import date
from ...core.currency import Currency
from ...core.money import Money
from ...core.dates.daycount import DayCount
from ...core.dates.calendar import BusinessDayConvention
from ..common import InstrumentType

class BasisSwapLeg:
    """Basis swap leg specification."""
    def __init__(
        self,
        forward_curve: str,
        *,
        frequency: Optional[str] = "quarterly",
        day_count: Optional[DayCount] = None,
        business_day_convention: Optional[BusinessDayConvention] = None,
        spread: float = 0.0,
    ) -> None: ...
    @property
    def forward_curve(self) -> str: ...
    @property
    def spread(self) -> float: ...

class BasisSwapBuilder:
    """Fluent builder returned by :meth:`BasisSwap.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def notional(self, amount: float) -> BasisSwapBuilder: ...
    def currency(self, currency: Union[str, Currency]) -> BasisSwapBuilder: ...
    def money(self, money: Money) -> BasisSwapBuilder: ...
    def start_date(self, start_date: date) -> BasisSwapBuilder: ...
    def maturity(self, maturity: date) -> BasisSwapBuilder: ...
    def primary_leg(self, primary_leg: BasisSwapLeg) -> BasisSwapBuilder: ...
    def reference_leg(self, reference_leg: BasisSwapLeg) -> BasisSwapBuilder: ...
    def disc_id(self, curve_id: str) -> BasisSwapBuilder: ...
    def calendar(self, calendar: Optional[str] = ...) -> BasisSwapBuilder: ...
    def stub(self, stub: Optional[str] = ...) -> BasisSwapBuilder: ...
    def build(self) -> "BasisSwap": ...

class BasisSwap:
    """Basis swap for exchanging two floating interest rates.

    BasisSwap represents a swap where both legs pay floating rates, typically
    based on different reference rates (e.g., LIBOR vs SOFR, 3M vs 6M). The
    difference between the two floating rates is the basis spread.

    Basis swaps are used to hedge basis risk, convert between different floating
    rate indices, and manage funding costs. They require forward curves for
    both legs and a discount curve.

    Examples
    --------
    Create a basis swap (LIBOR 3M vs SOFR):

        >>> from finstack.valuations.instruments import BasisSwap, BasisSwapLeg
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> primary_leg = BasisSwapLeg(forward_curve="USD-LIBOR-3M", frequency="quarterly", spread=0.0)
        >>> reference_leg = BasisSwapLeg(
        ...     forward_curve="USD-SOFR",
        ...     frequency="quarterly",
        ...     spread=0.001,  # 10bp basis spread (decimal)
        ... )
        >>> basis_swap = (
        ...     BasisSwap.builder("BASIS-LIBOR-SOFR")
        ...     .money(Money(10_000_000, Currency("USD")))
        ...     .start_date(date(2024, 1, 1))
        ...     .maturity(date(2029, 1, 1))  # 5-year swap
        ...     .primary_leg(primary_leg)
        ...     .reference_leg(reference_leg)
        ...     .disc_id("USD-OIS")
        ...     .build()
        ... )

    Notes
    -----
    - Basis swaps require forward curves for both legs
    - Primary leg typically pays the higher rate
    - Reference leg pays the lower rate plus basis spread
    - Basis spread compensates for differences in credit risk, liquidity, etc.
    - Both legs use floating rates (no fixed leg)

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required).
    - Forward curves: ``primary_leg.forward_curve`` and ``reference_leg.forward_curve`` (required).

    See Also
    --------
    :class:`InterestRateSwap`: Fixed-for-floating swaps
    :class:`ForwardCurve`: Forward rate curves
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - ISDA (2006) Definitions: see ``docs/REFERENCES.md#isda2006Definitions``.
    - Brigo & Mercurio (2006): see ``docs/REFERENCES.md#brigoMercurio2006``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> BasisSwapBuilder: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
