"""Interest rate swap instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.dates.schedule import Frequency
from ...core.dates.daycount import DayCount
from ...core.dates.calendar import BusinessDayConvention
from ..common import InstrumentType

class InterestRateSwap:
    """Interest rate swap instrument."""
    
    def __init__(
        self,
        instrument_id: str,
        notional: Money,
        start: date,
        maturity: date,
        fixed_rate: float,
        fixed_frequency: Frequency,
        fixed_day_count: DayCount,
        fixed_bdc: BusinessDayConvention,
        float_index: str,
        float_frequency: Frequency,
        float_day_count: DayCount,
        float_bdc: BusinessDayConvention,
        discount_curve: str,
        forward_curve: Optional[str] = None
    ) -> None:
        """Create an interest rate swap.
        
        Args:
            instrument_id: Instrument identifier
            notional: Notional amount
            start: Start date
            maturity: Maturity date
            fixed_rate: Fixed rate in decimal form
            fixed_frequency: Fixed leg frequency
            fixed_day_count: Fixed leg day count
            fixed_bdc: Fixed leg business day convention
            float_index: Floating rate index identifier
            float_frequency: Floating leg frequency
            float_day_count: Floating leg day count
            float_bdc: Floating leg business day convention
            discount_curve: Discount curve identifier
            forward_curve: Optional forward curve identifier
        """
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
    def fixed_rate(self) -> float: ...
    @property
    def float_index(self) -> str: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def forward_curve(self) -> Optional[str]: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
