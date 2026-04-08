"""Basis swap instrument (builder-only API)."""

from __future__ import annotations
from typing import Self
from datetime import date
from ....core.currency import Currency
from ....core.money import Money
from ....core.dates.daycount import DayCount
from ....core.dates.calendar import BusinessDayConvention
from ...common import InstrumentType

class BasisSwapLeg:
    """Basis swap leg specification.

    Each leg owns its own dates, discount curve, calendar, and stub conventions.
    """
    def __init__(
        self,
        forward_curve: str,
        *,
        discount_curve: str,
        start_date: date,
        end_date: date,
        frequency: str | None = "quarterly",
        day_count: DayCount | None = None,
        business_day_convention: BusinessDayConvention | None = None,
        calendar_id: str | None = None,
        stub: str | None = None,
        spread_bp: float = 0.0,
        payment_lag_days: int = 0,
        reset_lag_days: int = 0,
    ) -> None: ...
    @property
    def forward_curve(self) -> str: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def spread_bp(self) -> float: ...

class BasisSwapBuilder:
    """Fluent builder returned by :meth:`BasisSwap.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def notional(self, amount: float) -> BasisSwapBuilder: ...
    def currency(self, currency: str | Currency) -> BasisSwapBuilder: ...
    def money(self, money: Money) -> BasisSwapBuilder: ...
    def primary_leg(self, primary_leg: BasisSwapLeg) -> BasisSwapBuilder: ...
    def reference_leg(self, reference_leg: BasisSwapLeg) -> BasisSwapBuilder: ...
    def build(self) -> "BasisSwap": ...

class BasisSwap:
    """Basis swap for exchanging two floating interest rates.

    BasisSwap represents a swap where both legs pay floating rates, typically
    based on different reference rates (e.g., 3M SOFR vs 6M SOFR). The
    difference between the two floating rates is the basis spread.

    Each leg owns its own dates, discount curve, calendar, and stub conventions.

    Examples
    --------
    Create a basis swap (3M SOFR vs 6M SOFR):

        >>> from finstack.valuations.instruments import BasisSwap, BasisSwapLeg
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> primary_leg = BasisSwapLeg(
        ...     "USD-SOFR-3M",
        ...     discount_curve="USD-OIS",
        ...     start_date=date(2024, 1, 2),
        ...     end_date=date(2029, 1, 2),
        ...     frequency="quarterly",
        ...     spread_bp=5.0,
        ... )
        >>> reference_leg = BasisSwapLeg(
        ...     "USD-SOFR-6M",
        ...     discount_curve="USD-OIS",
        ...     start_date=date(2024, 1, 2),
        ...     end_date=date(2029, 1, 2),
        ...     frequency="semi_annual",
        ... )
        >>> basis_swap = (
        ...     BasisSwap
        ...     .builder("BASIS-3M-6M")
        ...     .money(Money(10_000_000, Currency("USD")))
        ...     .primary_leg(primary_leg)
        ...     .reference_leg(reference_leg)
        ...     .build()
        ... )

    MarketContext Requirements
    -------------------------
    - Discount curves: per-leg ``discount_curve`` (required).
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
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def to_json(self) -> str:
        """Serialize to JSON in envelope format.

        Returns:
            str: JSON string with schema version and tagged instrument spec.
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> "Self":
        """Deserialize from JSON in envelope format.

        Args:
            json_str: JSON string in envelope format.

        Returns:
            The deserialized instrument.

        Raises:
            ValueError: If JSON is malformed or contains a different instrument type.
        """
        ...
