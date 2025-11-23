"""Barrier option instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money

class BarrierType:
    """Barrier type enumeration."""

    UP_AND_OUT: "BarrierType"
    UP_AND_IN: "BarrierType"
    DOWN_AND_OUT: "BarrierType"
    DOWN_AND_IN: "BarrierType"

    @classmethod
    def from_name(cls, name: str) -> "BarrierType": ...
    @property
    def name(self) -> str: ...

class BarrierOption:
    """Barrier option instrument."""

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        ticker: str,
        strike: float,
        barrier: float,
        option_type: str,
        barrier_type: str,
        expiry: date,
        notional: Money,
        discount_curve: str,
        spot_id: str,
        vol_surface: str,
        *,
        div_yield_id: Optional[str] = None,
        use_gobet_miri: Optional[bool] = False,
    ) -> "BarrierOption":
        """Create a barrier option."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def ticker(self) -> str: ...
    @property
    def strike(self) -> Money: ...
    @property
    def barrier(self) -> Money: ...
    @property
    def option_type(self) -> str: ...
    @property
    def barrier_type(self) -> str: ...
    @property
    def expiry(self) -> date: ...
    @property
    def notional(self) -> Money: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
