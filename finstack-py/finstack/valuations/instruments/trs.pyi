"""Total return swap instruments."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.currency import Currency
from ..common import InstrumentType

class EquityTotalReturnSwap:
    """Equity total return swap instrument."""
    
    def __init__(
        self,
        instrument_id: str,
        notional: Money,
        underlying: str,
        start: date,
        maturity: date,
        spread_bp: float,
        currency: Currency,
        discount_curve: str
    ) -> None:
        """Create an equity total return swap."""
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
    def spread_bp(self) -> float: ...
    @property
    def currency(self) -> Currency: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class FiIndexTotalReturnSwap:
    """Fixed income index total return swap instrument."""
    
    def __init__(
        self,
        instrument_id: str,
        notional: Money,
        index_name: str,
        start: date,
        maturity: date,
        spread_bp: float,
        currency: Currency,
        discount_curve: str
    ) -> None:
        """Create a fixed income index total return swap."""
        ...
    
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def index_name(self) -> str: ...
    @property
    def start(self) -> date: ...
    @property
    def maturity(self) -> date: ...
    @property
    def spread_bp(self) -> float: ...
    @property
    def currency(self) -> Currency: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
