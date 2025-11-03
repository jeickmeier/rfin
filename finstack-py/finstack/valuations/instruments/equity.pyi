"""Equity instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.currency import Currency
from ..common import InstrumentType

class Equity:
    """Equity instrument."""

    def __init__(
        self, instrument_id: str, quantity: float, currency: Currency, as_of: date, underlying: Optional[str] = None
    ) -> None:
        """Create an equity instrument.

        Args:
            instrument_id: Instrument identifier
            quantity: Number of shares
            currency: Currency of the equity
            as_of: Valuation date
            underlying: Optional underlying identifier
        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def quantity(self) -> float: ...
    @property
    def currency(self) -> Currency: ...
    @property
    def as_of(self) -> date: ...
    @property
    def underlying(self) -> Optional[str]: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
