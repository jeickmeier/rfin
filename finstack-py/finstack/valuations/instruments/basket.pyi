"""Basket instrument."""

from typing import Optional, List
from datetime import date
from ...core.money import Money
from ...core.currency import Currency
from ..common import InstrumentType

class Basket:
    """Basket instrument."""

    def __init__(
        self,
        instrument_id: str,
        notional: Money,
        constituents: List[str],
        weights: List[float],
        as_of: date,
        currency: Currency,
    ) -> None:
        """Create a basket instrument."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def constituents(self) -> List[str]: ...
    @property
    def weights(self) -> List[float]: ...
    @property
    def as_of(self) -> date: ...
    @property
    def currency(self) -> Currency: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
