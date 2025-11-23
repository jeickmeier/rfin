"""Lookback option instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money

class LookbackType:
    """Lookback option type."""

    FIXED_STRIKE: "LookbackType"
    FLOATING_STRIKE: "LookbackType"
    @classmethod
    def from_name(cls, name: str) -> "LookbackType": ...
    @property
    def name(self) -> str: ...

class LookbackOption:
    """Lookback option instrument."""

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        ticker: str,
        strike: Optional[float],
        option_type: str,
        lookback_type: str,
        expiry: date,
        notional: Money,
        discount_curve: str,
        spot_id: str,
        vol_surface: str,
        *,
        div_yield_id: Optional[str] = None,
    ) -> "LookbackOption":
        """Create a lookback option."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def ticker(self) -> str: ...
    @property
    def strike(self) -> Optional[Money]: ...
    @property
    def option_type(self) -> str: ...
    @property
    def lookback_type(self) -> str: ...
    @property
    def expiry(self) -> date: ...
    @property
    def notional(self) -> Money: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
