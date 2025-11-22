"""Equity instrument."""

from typing import Optional
from ...core.currency import Currency
from ..common import InstrumentType

class Equity:
    """Spot equity position with optional share count and price override."""

    @classmethod
    def create(
        cls,
        instrument_id: str,
        ticker: str,
        currency: Currency,
        *,
        shares: Optional[float] = None,
        price: Optional[float] = None,
        price_id: Optional[str] = None,
        div_yield_id: Optional[str] = None,
    ) -> "Equity":
        """Create an equity instrument optionally specifying share count and price."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def ticker(self) -> str: ...
    @property
    def currency(self) -> Currency: ...
    @property
    def shares(self) -> float: ...
    @property
    def price_quote(self) -> Optional[float]: ...
    @property
    def price_id(self) -> Optional[str]: ...
    @property
    def div_yield_id(self) -> Optional[str]: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
