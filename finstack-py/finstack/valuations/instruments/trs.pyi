"""Total return swap instruments."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.currency import Currency
from ...core.dates.daycount import DayCount
from ..common import InstrumentType
from ..cashflow.builder import ScheduleParams

class TrsSide:
    """Total return swap side wrapper."""
    RECEIVE_TOTAL_RETURN: "TrsSide"
    PAY_TOTAL_RETURN: "TrsSide"

class TrsFinancingLegSpec:
    """Financing leg specification wrapper."""
    @classmethod
    def new(
        cls,
        discount_curve: str,
        forward_curve: str,
        day_count: DayCount,
        *,
        spread_bp: Optional[float] = 0.0,
    ) -> "TrsFinancingLegSpec": ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def forward_curve(self) -> str: ...
    @property
    def spread_bp(self) -> float: ...
    @property
    def day_count(self) -> str: ...

class TrsScheduleSpec:
    """TRS schedule specification wrapper."""
    @classmethod
    def new(
        cls,
        start: date,
        end: date,
        schedule_params: ScheduleParams,
    ) -> "TrsScheduleSpec": ...
    @property
    def start(self) -> date: ...
    @property
    def end(self) -> date: ...

class EquityUnderlying:
    """Equity underlying parameters wrapper."""
    @classmethod
    def new(
        cls,
        ticker: str,
        spot_id: str,
        currency: Currency,
        *,
        div_yield_id: Optional[str] = None,
        contract_size: Optional[float] = None,
    ) -> "EquityUnderlying": ...
    @property
    def ticker(self) -> str: ...
    @property
    def spot_id(self) -> str: ...
    @property
    def currency(self) -> Currency: ...

class IndexUnderlying:
    """Fixed-income index underlying parameters wrapper."""
    @classmethod
    def new(
        cls,
        index_id: str,
        base_currency: Currency,
        *,
        yield_id: Optional[str] = None,
        duration_id: Optional[str] = None,
        convexity_id: Optional[str] = None,
        contract_size: Optional[float] = None,
    ) -> "IndexUnderlying": ...
    @property
    def index_id(self) -> str: ...
    @property
    def base_currency(self) -> Currency: ...

class EquityTotalReturnSwap:
    """Equity TRS wrapper."""

    @classmethod
    def create(
        cls,
        instrument_id: str,
        notional: Money,
        underlying: EquityUnderlying,
        financing: TrsFinancingLegSpec,
        schedule: TrsScheduleSpec,
        side: TrsSide,
        *,
        initial_level: Optional[float] = None,
    ) -> "EquityTotalReturnSwap":
        """Create an equity total return swap."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    @property
    def side(self) -> str: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class FiIndexTotalReturnSwap:
    """Fixed income index TRS wrapper."""

    @classmethod
    def create(
        cls,
        instrument_id: str,
        notional: Money,
        underlying: IndexUnderlying,
        financing: TrsFinancingLegSpec,
        schedule: TrsScheduleSpec,
        side: TrsSide,
        *,
        initial_level: Optional[float] = None,
    ) -> "FiIndexTotalReturnSwap":
        """Create a fixed income index total return swap."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    @property
    def side(self) -> str: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
