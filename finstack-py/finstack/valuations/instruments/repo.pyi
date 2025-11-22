"""Repo instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.dates.daycount import DayCount
from ...core.dates.calendar import BusinessDayConvention
from ..common import InstrumentType

class RepoCollateral:
    """Collateral specification for Repo."""
    def __init__(
        self,
        instrument_id: str,
        quantity: float,
        market_value_id: str,
        *,
        collateral_type: str = "general",
        special_security_id: Optional[str] = None,
        special_rate_adjust_bp: Optional[float] = None,
    ) -> None: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def quantity(self) -> float: ...
    @property
    def market_value_id(self) -> str: ...

class Repo:
    """Repo wrapper exposing a convenience constructor."""

    @classmethod
    def create(
        cls,
        instrument_id: str,
        cash_amount: Money,
        collateral: RepoCollateral,
        repo_rate: float,
        start_date: date,
        maturity: date,
        discount_curve: str,
        *,
        repo_type: str = "term",
        haircut: float = 0.0,
        day_count: Optional[DayCount] = None,
        business_day_convention: Optional[BusinessDayConvention] = None,
        calendar: Optional[str] = None,
        triparty: bool = False,
    ) -> "Repo":
        """Create a repo."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def cash_amount(self) -> Money: ...
    @property
    def repo_rate(self) -> float: ...
    @property
    def start_date(self) -> date: ...
    @property
    def maturity(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
