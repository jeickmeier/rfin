"""Asian option instrument."""

from typing import Optional, List
from datetime import date
from ...core.money import Money

class AveragingMethod:
    """Averaging method enumeration."""

    ARITHMETIC: "AveragingMethod"
    GEOMETRIC: "AveragingMethod"

    @classmethod
    def from_name(cls, name: str) -> "AveragingMethod": ...
    @property
    def name(self) -> str: ...

class AsianOption:
    """Asian option instrument with arithmetic or geometric averaging."""

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        ticker: str,
        strike: float,
        expiry: date,
        fixing_dates: List[date],
        notional: Money,
        discount_curve: str,
        spot_id: str,
        vol_surface: str,
        *,
        averaging_method: Optional[str] = "arithmetic",
        option_type: Optional[str] = "call",
        div_yield_id: Optional[str] = None,
    ) -> "AsianOption":
        """Create an Asian option with explicit parameters."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def ticker(self) -> str: ...
    @property
    def strike(self) -> Money: ...
    @property
    def option_type(self) -> str: ...
    @property
    def averaging_method(self) -> str: ...
    @property
    def expiry(self) -> date: ...
    @property
    def fixing_dates(self) -> List[date]: ...
    @property
    def notional(self) -> Money: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def spot_id(self) -> str: ...
    @property
    def vol_surface(self) -> str: ...
    @property
    def div_yield_id(self) -> Optional[str]: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
