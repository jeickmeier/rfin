"""Convertible bond instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.currency import Currency
from ..common import InstrumentType

class ConvertibleBond:
    """Convertible bond instrument."""

    def __init__(
        self,
        instrument_id: str,
        notional: Money,
        issue: date,
        maturity: date,
        coupon_rate: float,
        conversion_ratio: float,
        underlying: str,
        currency: Currency,
        discount_curve: str,
    ) -> None:
        """Create a convertible bond."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def issue(self) -> date: ...
    @property
    def maturity(self) -> date: ...
    @property
    def coupon_rate(self) -> float: ...
    @property
    def conversion_ratio(self) -> float: ...
    @property
    def underlying(self) -> str: ...
    @property
    def currency(self) -> Currency: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
