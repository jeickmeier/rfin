"""Equity option instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ..common import InstrumentType

class EquityOption:
    """Equity option priced via Black–Scholes style models."""

    @classmethod
    def european_call(
        cls,
        instrument_id: str,
        ticker: str,
        strike: float,
        expiry: date,
        notional: Money,
        contract_size: Optional[float] = 1.0,
    ) -> "EquityOption":
        """Create a European call option with standard market conventions."""
        ...

    @classmethod
    def european_put(
        cls,
        instrument_id: str,
        ticker: str,
        strike: float,
        expiry: date,
        notional: Money,
        contract_size: Optional[float] = 1.0,
    ) -> "EquityOption":
        """Create a European put option with standard market conventions."""
        ...

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        ticker: str,
        strike: float,
        expiry: date,
        notional: Money,
        discount_curve: str,
        spot_id: str,
        vol_surface: str,
        *,
        div_yield_id: Optional[str] = None,
        contract_size: Optional[float] = 1.0,
    ) -> "EquityOption":
        """Create an equity option with explicit discount curve, spot id, vol surface and optional dividend yield."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def ticker(self) -> str: ...
    @property
    def strike(self) -> Money: ...
    @property
    def contract_size(self) -> float: ...
    @property
    def option_type(self) -> str: ...
    @property
    def exercise_style(self) -> str: ...
    @property
    def expiry(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def vol_surface(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
