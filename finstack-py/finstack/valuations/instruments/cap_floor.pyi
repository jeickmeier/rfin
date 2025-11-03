"""Interest rate option (cap/floor) instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.dates.schedule import Frequency
from ...core.dates.daycount import DayCount
from ...core.dates.calendar import BusinessDayConvention
from ..common import InstrumentType

class InterestRateOption:
    """Interest rate option (cap/floor) instrument."""

    def __init__(
        self,
        instrument_id: str,
        notional: Money,
        start: date,
        maturity: date,
        strike_rate: float,
        option_type: str,  # "cap" or "floor"
        frequency: Frequency,
        day_count: DayCount,
        bdc: BusinessDayConvention,
        currency: str,
        discount_curve: str,
        forward_curve: Optional[str] = None,
    ) -> None:
        """Create an interest rate option."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def start(self) -> date: ...
    @property
    def maturity(self) -> date: ...
    @property
    def strike_rate(self) -> float: ...
    @property
    def option_type(self) -> str: ...
    @property
    def frequency(self) -> Frequency: ...
    @property
    def day_count(self) -> DayCount: ...
    @property
    def bdc(self) -> BusinessDayConvention: ...
    @property
    def currency(self) -> str: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def forward_curve(self) -> Optional[str]: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
