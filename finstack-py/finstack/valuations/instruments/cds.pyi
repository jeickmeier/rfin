"""Credit default swap instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ..common import InstrumentType

class CDSPayReceive:
    """Pay/receive indicator for CDS premium leg."""

    PAY_PROTECTION: "CDSPayReceive"
    RECEIVE_PROTECTION: "CDSPayReceive"

    @classmethod
    def from_name(cls, name: str) -> "CDSPayReceive": ...
    @property
    def name(self) -> str: ...

class CreditDefaultSwap:
    """Credit default swap wrapper with helper constructors."""

    @classmethod
    def buy_protection(
        cls,
        instrument_id: str,
        notional: Money,
        spread_bp: float,
        start_date: date,
        maturity: date,
        discount_curve: str,
        credit_curve: str,
        *,
        recovery_rate: Optional[float] = None,
        settlement_delay: Optional[int] = None,
    ) -> "CreditDefaultSwap":
        """Create a CDS where the caller buys protection (pays premium, receives protection)."""
        ...

    @classmethod
    def sell_protection(
        cls,
        instrument_id: str,
        notional: Money,
        spread_bp: float,
        start_date: date,
        maturity: date,
        discount_curve: str,
        credit_curve: str,
        *,
        recovery_rate: Optional[float] = None,
        settlement_delay: Optional[int] = None,
    ) -> "CreditDefaultSwap":
        """Create a CDS where the caller sells protection (receives premium)."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def side(self) -> CDSPayReceive: ...
    @property
    def notional(self) -> Money: ...
    @property
    def spread_bp(self) -> float: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def credit_curve(self) -> str: ...
    @property
    def recovery_rate(self) -> float: ...
    @property
    def settlement_delay(self) -> int: ...
    @property
    def start_date(self) -> date: ...
    @property
    def maturity(self) -> date: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
