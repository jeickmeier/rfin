"""FX barrier option instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.currency import Currency
from ..common import InstrumentType

class FxBarrierOption:
    """FX barrier option instrument."""

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        strike: float,
        barrier: float,
        option_type: str,
        barrier_type: str,
        expiry: date,
        notional: Money,
        domestic_currency: Currency,
        foreign_currency: Currency,
        discount_curve: str,
        foreign_discount_curve: str,
        fx_spot_id: str,
        fx_vol_surface: str,
        *,
        use_gobet_miri: Optional[bool] = False,
    ) -> "FxBarrierOption":
        """Create an FX barrier option."""
        ...

    @property
    def instrument_id(self) -> str: ...
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
