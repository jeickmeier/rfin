"""CMS option instrument."""

from typing import List, Optional
from datetime import date
from ...core.money import Money
from ...core.dates.schedule import Frequency
from ...core.dates.daycount import DayCount

class CmsOption:
    """CMS option instrument."""

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        strike_rate: float,
        cms_tenor: float,
        fixing_dates: List[date],
        accrual_fractions: List[float],
        option_type: str,
        notional: Money,
        discount_curve: str,
        *,
        vol_surface: Optional[str] = None,
        payment_dates: Optional[List[date]] = None,
        swap_fixed_freq: Optional[Frequency] = None,
        swap_float_freq: Optional[Frequency] = None,
        swap_day_count: Optional[DayCount] = None,
    ) -> "CmsOption":
        """Create a CMS option."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def strike_rate(self) -> float: ...
    @property
    def cms_tenor(self) -> float: ...
    @property
    def option_type(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def fixing_dates(self) -> List[date]: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
