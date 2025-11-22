"""Interest rate option (cap/floor) instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.dates.daycount import DayCount
from ..common import InstrumentType

class InterestRateOption:
    """Interest rate cap/floor instruments using Black pricing."""

    @classmethod
    def cap(
        cls,
        instrument_id: str,
        notional: Money,
        strike: float,
        start_date: date,
        end_date: date,
        discount_curve: str,
        forward_curve: str,
        vol_surface: str,
        *,
        payments_per_year: int = 4,
        day_count: Optional[DayCount] = None,
    ) -> "InterestRateOption":
        """Create a standard interest-rate cap."""
        ...

    @classmethod
    def floor(
        cls,
        instrument_id: str,
        notional: Money,
        strike: float,
        start_date: date,
        end_date: date,
        discount_curve: str,
        forward_curve: str,
        vol_surface: str,
        *,
        payments_per_year: int = 4,
        day_count: Optional[DayCount] = None,
    ) -> "InterestRateOption":
        """Create a standard interest-rate floor."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def strike(self) -> float: ...
    @property
    def start_date(self) -> date: ...
    @property
    def end_date(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def forward_curve(self) -> str: ...
    @property
    def vol_surface(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
