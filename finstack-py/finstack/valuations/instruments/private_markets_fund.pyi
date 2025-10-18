"""Private markets fund instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.currency import Currency
from ..common import InstrumentType

class PrivateMarketsFund:
    """Private markets fund instrument."""
    
    def __init__(
        self,
        instrument_id: str,
        notional: Money,
        fund_type: str,
        start: date,
        maturity: date,
        management_fee_bp: float,
        performance_fee_bp: float,
        currency: Currency,
        discount_curve: str
    ) -> None:
        """Create a private markets fund instrument."""
        ...
    
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def fund_type(self) -> str: ...
    @property
    def start(self) -> date: ...
    @property
    def maturity(self) -> date: ...
    @property
    def management_fee_bp(self) -> float: ...
    @property
    def performance_fee_bp(self) -> float: ...
    @property
    def currency(self) -> Currency: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
