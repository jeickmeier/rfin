"""CDS tranche instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.currency import Currency
from ..common import InstrumentType

class CdsTranche:
    """CDS tranche instrument."""

    def __init__(
        self,
        instrument_id: str,
        notional: Money,
        attachment_point: float,
        detachment_point: float,
        start: date,
        maturity: date,
        spread_bp: float,
        currency: Currency,
        discount_curve: str,
    ) -> None:
        """Create a CDS tranche."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def attachment_point(self) -> float: ...
    @property
    def detachment_point(self) -> float: ...
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
