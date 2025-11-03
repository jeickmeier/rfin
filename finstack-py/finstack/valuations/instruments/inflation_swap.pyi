"""Inflation swap instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.currency import Currency
from ..common import InstrumentType

class InflationSwap:
    """Inflation swap instrument."""

    def __init__(
        self,
        instrument_id: str,
        notional: Money,
        start: date,
        maturity: date,
        fixed_rate: float,
        inflation_index: str,
        currency: Currency,
        discount_curve: str,
    ) -> None:
        """Create an inflation swap."""
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
    def fixed_rate(self) -> float: ...
    @property
    def inflation_index(self) -> str: ...
    @property
    def currency(self) -> Currency: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
