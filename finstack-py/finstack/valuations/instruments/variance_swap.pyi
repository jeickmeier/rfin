"""Variance swap instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.currency import Currency
from ..common import InstrumentType

class VarianceSwap:
    """Variance swap instrument."""
    
    def __init__(
        self,
        instrument_id: str,
        notional: Money,
        underlying: str,
        start: date,
        maturity: date,
        strike_variance: float,
        currency: Currency,
        discount_curve: str
    ) -> None:
        """Create a variance swap."""
        ...
    
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def underlying(self) -> str: ...
    @property
    def start(self) -> date: ...
    @property
    def maturity(self) -> date: ...
    @property
    def strike_variance(self) -> float: ...
    @property
    def currency(self) -> Currency: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
