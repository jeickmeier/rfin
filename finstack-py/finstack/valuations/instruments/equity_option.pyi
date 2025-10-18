"""Equity option instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.currency import Currency
from ..common import InstrumentType

class EquityOption:
    """Equity option instrument."""
    
    def __init__(
        self,
        instrument_id: str,
        underlying: str,
        quantity: float,
        strike: float,
        expiry: date,
        option_type: str,  # "call" or "put"
        currency: Currency,
        as_of: date
    ) -> None:
        """Create an equity option instrument."""
        ...
    
    @property
    def instrument_id(self) -> str: ...
    @property
    def underlying(self) -> str: ...
    @property
    def quantity(self) -> float: ...
    @property
    def strike(self) -> float: ...
    @property
    def expiry(self) -> date: ...
    @property
    def option_type(self) -> str: ...
    @property
    def currency(self) -> Currency: ...
    @property
    def as_of(self) -> date: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
