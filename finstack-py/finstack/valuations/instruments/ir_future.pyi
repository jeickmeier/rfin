"""Interest rate future instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.dates.daycount import DayCount
from ..common import InstrumentType

class InterestRateFuture:
    """Interest rate future wrapper exposing a convenience constructor."""

    @classmethod
    def create(
        cls,
        instrument_id: str,
        notional: Money,
        quoted_price: float,
        expiry: date,
        fixing_date: date,
        period_start: date,
        period_end: date,
        discount_curve: str,
        forward_curve: str,
        *,
        position: Optional[str] = "long",
        day_count: Optional[DayCount] = None,
        face_value: float = 1_000_000.0,
        tick_size: float = 0.0025,
        tick_value: Optional[float] = None,
        delivery_months: int = 3,
        convexity_adjustment: Optional[float] = None,
    ) -> "InterestRateFuture":
        """Create an interest rate future."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def quoted_price(self) -> float: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
