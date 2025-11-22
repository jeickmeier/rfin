"""Autocallable structured product instrument."""

from typing import List, Optional, Dict, Union, Any
from datetime import date
from ...core.money import Money

class Autocallable:
    """Autocallable structured product instrument."""

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        ticker: str,
        observation_dates: List[date],
        autocall_barriers: List[float],
        coupons: List[float],
        final_barrier: float,
        final_payoff_type: Union[str, Dict[str, Any]],
        participation_rate: float,
        cap_level: float,
        notional: Money,
        discount_curve: str,
        spot_id: str,
        vol_surface: str,
        *,
        div_yield_id: Optional[str] = None,
    ) -> "Autocallable":
        """Create an autocallable structured product."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def ticker(self) -> str: ...
    @property
    def final_barrier(self) -> float: ...
    @property
    def participation_rate(self) -> float: ...
    @property
    def cap_level(self) -> float: ...
    @property
    def notional(self) -> Money: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
