"""CDS option instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.currency import Currency
from ..common import InstrumentType

class CdsOption:
    """CDS option instrument."""
    
    def __init__(
        self,
        instrument_id: str,
        notional: Money,
        underlying_cds: str,
        strike_spread_bp: float,
        expiry: date,
        option_type: str,  # "call" or "put"
        currency: Currency,
        discount_curve: str
    ) -> None:
        """Create a CDS option."""
        ...
    
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def underlying_cds(self) -> str: ...
    @property
    def strike_spread_bp(self) -> float: ...
    @property
    def expiry(self) -> date: ...
    @property
    def option_type(self) -> str: ...
    @property
    def currency(self) -> Currency: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
