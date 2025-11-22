"""Forward rate agreement instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.dates.daycount import DayCount
from ..common import InstrumentType

class ForwardRateAgreement:
    """Forward Rate Agreement binding exposing standard FRA parameters."""

    @classmethod
    def create(
        cls,
        instrument_id: str,
        notional: Money,
        fixed_rate: float,
        fixing_date: date,
        start_date: date,
        end_date: date,
        discount_curve: str,
        forward_curve: str,
        *,
        day_count: Optional[DayCount] = None,
        reset_lag: int = 2,
        pay_fixed: bool = True,
    ) -> "ForwardRateAgreement":
        """Create a standard FRA referencing discount and forward curves."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def fixed_rate(self) -> float: ...
    @property
    def day_count(self) -> DayCount: ...
    @property
    def reset_lag(self) -> int: ...
    @property
    def pay_fixed(self) -> bool: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def forward_curve(self) -> str: ...
    @property
    def fixing_date(self) -> date: ...
    @property
    def start_date(self) -> date: ...
    @property
    def end_date(self) -> date: ...
    @property
    def notional(self) -> Money: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
