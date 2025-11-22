"""Money-market deposit with simple interest accrual."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.dates.daycount import DayCount
from ..common import InstrumentType

class Deposit:
    """Money-market deposit with simple interest accrual."""

    def __init__(
        self,
        instrument_id: str,
        notional: Money,
        start: date,
        end: date,
        day_count: DayCount,
        discount_curve: str,
        quote_rate: Optional[float] = None,
    ) -> None:
        """Create a deposit with explicit start/end dates and optional quoted rate."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def start(self) -> date: ...
    @property
    def end(self) -> date: ...
    @property
    def day_count(self) -> DayCount: ...
    @property
    def quote_rate(self) -> Optional[float]: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
