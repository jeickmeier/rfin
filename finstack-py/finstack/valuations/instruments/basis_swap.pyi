"""Basis swap instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.dates.schedule import Frequency
from ...core.dates.daycount import DayCount
from ...core.dates.calendar import BusinessDayConvention
from ..common import InstrumentType

class BasisSwap:
    """Basis swap instrument."""
    
    def __init__(
        self,
        instrument_id: str,
        notional: Money,
        start: date,
        maturity: date,
        primary_index: str,
        reference_index: str,
        spread_bp: float,
        primary_frequency: Frequency,
        reference_frequency: Frequency,
        primary_day_count: DayCount,
        reference_day_count: DayCount,
        primary_bdc: BusinessDayConvention,
        reference_bdc: BusinessDayConvention,
        currency: str,
        discount_curve: str
    ) -> None:
        """Create a basis swap."""
        ...
    
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def start(self) -> date: ...
    @property
    def maturity(self) -> date: ...
    @property
    def primary_index(self) -> str: ...
    @property
    def reference_index(self) -> str: ...
    @property
    def spread_bp(self) -> float: ...
    @property
    def currency(self) -> str: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
