"""Interest rate future instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ..common import InstrumentType

class InterestRateFuture:
    """Interest rate future instrument."""

    def __init__(
        self,
        instrument_id: str,
        notional: Money,
        start: date,
        maturity: date,
        rate: float,
        currency: str,
        discount_curve: str,
    ) -> None:
        """Create an interest rate future."""
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
    def rate(self) -> float: ...
    @property
    def currency(self) -> str: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
