"""Basis swap instrument."""

from typing import Optional
from datetime import date
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

class BasisSwap:
    """Basis swap wrapper with convenience constructor."""

    @classmethod
    def create(
        cls,
        instrument_id: str,
        notional: Money,
        start_date: date,
        maturity: date,
        primary_leg: BasisSwapLeg,
        reference_leg: BasisSwapLeg,
        discount_curve: str,
        *,
        calendar: Optional[str] = None,
        stub: Optional[str] = "none",
    ) -> "BasisSwap":
        """Create a floating-for-floating basis swap with two legs."""
        ...

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
