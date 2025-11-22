"""Interest rate swap instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.dates.schedule import Frequency, StubKind
from ...core.dates.daycount import DayCount
from ...core.dates.calendar import BusinessDayConvention
from ..common import InstrumentType

class PayReceive:
    """Pay/receive direction for swap fixed-leg cashflows."""
    PAY_FIXED: "PayReceive"
    RECEIVE_FIXED: "PayReceive"

    @classmethod
    def from_name(cls, name: str) -> "PayReceive": ...
    @property
    def name(self) -> str: ...

class InterestRateSwap:
    """Plain-vanilla interest rate swap with fixed-for-floating legs."""

    @classmethod
    def usd_pay_fixed(
        cls,
        instrument_id: str,
        notional: Money,
        fixed_rate: float,
        start: date,
        end: date,
    ) -> "InterestRateSwap":
        """Create a USD SOFR swap where the caller pays fixed and receives floating."""
        ...

    @classmethod
    def usd_receive_fixed(
        cls,
        instrument_id: str,
        notional: Money,
        fixed_rate: float,
        start: date,
        end: date,
    ) -> "InterestRateSwap":
        """Create a USD SOFR swap where the caller receives fixed."""
        ...

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        notional: Money,
        fixed_rate: float,
        start: date,
        end: date,
        side: str,
        discount_curve: str,
        forward_curve: str,
        *,
        fixed_frequency: Optional[Frequency] = None,
        float_frequency: Optional[Frequency] = None,
        fixed_day_count: Optional[DayCount] = None,
        float_day_count: Optional[DayCount] = None,
        business_day_convention: Optional[BusinessDayConvention] = None,
        float_spread_bp: float = 0.0,
        reset_lag_days: int = 2,
        calendar: Optional[str] = None,
        stub: Optional[StubKind] = None,
    ) -> "InterestRateSwap":
        """Create a fully customizable interest rate swap with explicit curves and conventions."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def side(self) -> PayReceive: ...
    @property
    def fixed_rate(self) -> float: ...
    @property
    def float_spread_bp(self) -> float: ...
    @property
    def start(self) -> date: ...
    @property
    def end(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def forward_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
