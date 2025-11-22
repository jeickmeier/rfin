"""Inflation swap instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ..common import InstrumentType

class InflationSwap:
    """Zero-coupon inflation swap binding."""

    @classmethod
    def create(
        cls,
        instrument_id: str,
        notional: Money,
        fixed_rate: float,
        start_date: date,
        maturity: date,
        discount_curve: str,
        inflation_index: Optional[str] = None,
        *,
        side: Optional[str] = "pay_fixed",
        day_count: Optional[str] = "act_act",
        inflation_index_id: Optional[str] = None,
        lag_override: Optional[str] = None,
        inflation_curve: Optional[str] = None,
    ) -> "InflationSwap":
        """Create an inflation swap fixing against the supplied inflation index."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def fixed_rate(self) -> float: ...
    @property
    def maturity(self) -> date: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
