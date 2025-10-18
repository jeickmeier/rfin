"""Forward rate agreement instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.dates.daycount import DayCount
from ..common import InstrumentType

class ForwardRateAgreement:
    """Forward rate agreement instrument."""
    
    def __init__(
        self,
        instrument_id: str,
        notional: Money,
        start: date,
        end: date,
        rate: float,
        day_count: DayCount,
        discount_curve: str
    ) -> None:
        """Create a forward rate agreement."""
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
    def rate(self) -> float: ...
    @property
    def day_count(self) -> DayCount: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
