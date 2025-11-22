"""Cliquet option instrument."""

from typing import List, Optional
from datetime import date
from ...core.money import Money

class CliquetOption:
    """Cliquet option instrument."""

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        underlying_ticker: str,
        reset_dates: List[date],
        maturity: date,
        notional: Money,
        discount_curve: str,
        vol_surface: str,
        spot_id: str,
        *,
        local_cap: float = 0.0,
        local_floor: float = 0.0,
        global_cap: float = 0.0,
        global_floor: float = 0.0,
        div_yield_id: Optional[str] = None,
    ) -> "CliquetOption":
        """Create a cliquet option."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def underlying_ticker(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def maturity(self) -> date: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
