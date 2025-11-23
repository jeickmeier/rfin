"""Variance swap instrument."""

from typing import Optional, Union
from datetime import date
from ...core.money import Money
from ...core.dates.schedule import Frequency
from ...core.dates.daycount import DayCount
from ..common import InstrumentType

class VarianceDirection:
    """Pay/receive wrapper for variance swap payoffs."""

    PAY: "VarianceDirection"
    RECEIVE: "VarianceDirection"

class RealizedVarianceMethod:
    """Realized variance calculation method wrapper."""

    CLOSE_TO_CLOSE: "RealizedVarianceMethod"
    PARKINSON: "RealizedVarianceMethod"
    GARMAN_KLASS: "RealizedVarianceMethod"
    ROGERS_SATCHELL: "RealizedVarianceMethod"
    YANG_ZHANG: "RealizedVarianceMethod"

class VarianceSwap:
    """Variance swap instrument."""

    @classmethod
    def create(
        cls,
        instrument_id: str,
        underlying_id: str,
        notional: Money,
        strike_variance: float,
        start_date: date,
        maturity: date,
        discount_curve: str,
        observation_frequency: Frequency,
        *,
        realized_method: Optional[RealizedVarianceMethod] = None,
        side: Optional[Union[VarianceDirection, str]] = None,
        day_count: Optional[DayCount] = None,
    ) -> "VarianceSwap":
        """Create a variance swap."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    @property
    def strike_variance(self) -> float: ...
    @property
    def observation_frequency(self) -> str: ...
    @property
    def realized_method(self) -> str: ...
    @property
    def side(self) -> str: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
