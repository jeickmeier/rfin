"""Quanto option instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.currency import Currency

class QuantoOption:
    """Quanto option instrument."""

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        ticker: str,
        equity_strike: float,
        option_type: str,
        expiry: date,
        notional: Money,
        domestic_currency: Currency,
        foreign_currency: Currency,
        correlation: float,
        discount_curve: str,
        foreign_discount_curve: str,
        spot_id: str,
        vol_surface: str,
        *,
        div_yield_id: Optional[str] = None,
        fx_rate_id: Optional[str] = None,
        fx_vol_id: Optional[str] = None,
    ) -> "QuantoOption":
        """Create a quanto option."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def ticker(self) -> str: ...
    @property
    def equity_strike(self) -> Money: ...
    @property
    def option_type(self) -> str: ...
    @property
    def expiry(self) -> date: ...
    @property
    def notional(self) -> Money: ...
    @property
    def correlation(self) -> float: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
