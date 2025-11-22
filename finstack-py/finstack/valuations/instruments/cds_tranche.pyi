"""CDS tranche instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.dates.daycount import DayCount
from ...core.dates.calendar import BusinessDayConvention
from ..common import InstrumentType

class CdsTranche:
    """CDS tranche wrapper exposing a simplified constructor."""

    @classmethod
    def create(
        cls,
        instrument_id: str,
        index_name: str,
        series: int,
        attach_pct: float,
        detach_pct: float,
        notional: Money,
        maturity: date,
        running_coupon_bp: float,
        discount_curve: str,
        credit_index_curve: str,
        *,
        side: Optional[str] = "buy_protection",
        payments_per_year: Optional[int] = 4,
        day_count: Optional[DayCount] = None,
        business_day_convention: Optional[BusinessDayConvention] = None,
        calendar: Optional[str] = None,
        effective_date: Optional[date] = None,
    ) -> "CdsTranche":
        """Create a CDS tranche referencing a credit index."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def attach_pct(self) -> float: ...
    @property
    def detach_pct(self) -> float: ...
    @property
    def running_coupon_bp(self) -> float: ...
    @property
    def maturity(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def credit_index_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
