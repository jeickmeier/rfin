"""CMS swap instrument."""

from __future__ import annotations

from datetime import date
from typing import Any

from ....core.money import Money
from ....core.dates.daycount import DayCount
from ....core.dates.schedule import Frequency
from ...common import InstrumentType

class CmsSwap:
    """Constant maturity swap instrument."""

    @classmethod
    def from_schedule(
        cls,
        instrument_id: str,
        start_date: date,
        maturity: date,
        frequency: Frequency,
        cms_tenor: float,
        cms_spread: float,
        funding_leg: dict[str, Any],
        notional: Money,
        cms_day_count: DayCount,
        swap_convention: str,
        side: str,
        discount_curve: str,
        forward_curve: str,
        vol_surface: str,
    ) -> CmsSwap: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def cms_tenor(self) -> float: ...
    @property
    def cms_spread(self) -> float: ...
    @property
    def cms_fixing_dates(self) -> list[date]: ...
    @property
    def cms_payment_dates(self) -> list[date]: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def forward_curve(self) -> str: ...
    @property
    def vol_surface(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
