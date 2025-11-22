"""CDS index instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ..common import InstrumentType

class CDSIndex:
    """CDS index instrument binding exposing a simplified constructor."""

    @classmethod
    def create(
        cls,
        instrument_id: str,
        index_name: str,
        series: int,
        version: int,
        notional: Money,
        fixed_coupon_bp: float,
        start_date: date,
        maturity: date,
        discount_curve: str,
        credit_curve: str,
        *,
        side: Optional[str] = "pay_protection",
        recovery_rate: Optional[float] = None,
        index_factor: Optional[float] = None,
    ) -> "CDSIndex":
        """Create a CDS index instrument with standard ISDA conventions."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def index_name(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def fixed_coupon_bp(self) -> float: ...
    @property
    def side(self) -> str: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def credit_curve(self) -> str: ...
    @property
    def maturity(self) -> date: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
