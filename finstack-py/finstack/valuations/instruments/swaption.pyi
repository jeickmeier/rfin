"""Swaption instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ..common import InstrumentType

class Swaption:
    """Swaption bindings with payer/receiver constructors."""

    @classmethod
    def payer(
        cls,
        instrument_id: str,
        notional: Money,
        strike: float,
        expiry: date,
        swap_start: date,
        swap_end: date,
        discount_curve: str,
        forward_curve: str,
        vol_surface: str,
        exercise: Optional[str] = "european",
        settlement: Optional[str] = "physical",
    ) -> "Swaption":
        """Create a payer swaption (pay fixed underlying swap)."""
        ...

    @classmethod
    def receiver(
        cls,
        instrument_id: str,
        notional: Money,
        strike: float,
        expiry: date,
        swap_start: date,
        swap_end: date,
        discount_curve: str,
        forward_curve: str,
        vol_surface: str,
        exercise: Optional[str] = "european",
        settlement: Optional[str] = "physical",
    ) -> "Swaption":
        """Create a receiver swaption (receive fixed underlying swap)."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def strike(self) -> float: ...
    @property
    def expiry(self) -> date: ...
    @property
    def swap_start(self) -> date: ...
    @property
    def swap_end(self) -> date: ...
    @property
    def option_type(self) -> str: ...
    @property
    def settlement(self) -> str: ...
    @property
    def exercise(self) -> str: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def forward_curve(self) -> str: ...
    @property
    def vol_surface(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
