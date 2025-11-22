"""CDS option instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ..common import InstrumentType

class CdsOption:
    """Option on CDS spread with simplified constructor."""

    @classmethod
    def create(
        cls,
        instrument_id: str,
        notional: Money,
        strike_spread_bp: float,
        expiry: date,
        cds_maturity: date,
        discount_curve: str,
        credit_curve: str,
        vol_surface: str,
        *,
        option_type: Optional[str] = "call",
        recovery_rate: Optional[float] = 0.4,
        underlying_is_index: Optional[bool] = False,
        index_factor: Optional[float] = None,
        forward_adjust_bp: Optional[float] = 0.0,
    ) -> "CdsOption":
        """Create a CDS option referencing a standard CDS contract."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def strike_spread_bp(self) -> float: ...
    @property
    def expiry(self) -> date: ...
    @property
    def cds_maturity(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def credit_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
