"""Inflation linked bond instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.dates.daycount import DayCount
from ..common import InstrumentType

class InflationLinkedBond:
    """Inflation-linked bond binding with a convenience constructor."""

    @classmethod
    def create(
        cls,
        instrument_id: str,
        notional: Money,
        real_coupon: float,
        issue: date,
        maturity: date,
        base_index: float,
        discount_curve: str,
        inflation_curve: str,
        *,
        indexation: Optional[str] = "tips",
        frequency: Optional[str] = "semi_annual",
        day_count: Optional[DayCount] = None,
        deflation_protection: Optional[str] = "maturity_only",
        calendar: Optional[str] = None,
    ) -> "InflationLinkedBond":
        """Create an inflation-linked bond instrument using standard parameters."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def real_coupon(self) -> float: ...
    @property
    def maturity(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def inflation_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
