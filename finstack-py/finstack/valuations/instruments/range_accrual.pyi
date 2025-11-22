"""Range accrual instrument."""

from typing import Optional, List
from datetime import date
from ...core.money import Money

class RangeAccrual:
    """Range accrual instrument."""

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        ticker: str,
        observation_dates: List[date],
        lower_bound: float,
        upper_bound: float,
        coupon_rate: float,
        notional: Money,
        discount_curve: str,
        spot_id: str,
        vol_surface: str,
        *,
        div_yield_id: Optional[str] = None,
    ) -> "RangeAccrual":
        """Create a range accrual instrument."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def ticker(self) -> str: ...
    @property
    def lower_bound(self) -> float: ...
    @property
    def upper_bound(self) -> float: ...
    @property
    def coupon_rate(self) -> float: ...
    @property
    def notional(self) -> Money: ...
    @property
    def observation_dates(self) -> List[date]: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
