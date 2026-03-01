"""Total return swap instruments."""

from __future__ import annotations
from datetime import date
from ....core.money import Money
from ....core.currency import Currency
from ....core.dates.daycount import DayCount
from ....core.market_data.context import MarketContext
from ...common import InstrumentType
from ...cashflow.builder import ScheduleParams

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
        spread_bp: float | None = 0.0,
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
        div_yield_id: str | None = None,
        contract_size: float | None = None,
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
        yield_id: str | None = None,
        duration_id: str | None = None,
        convexity_id: str | None = None,
        contract_size: float | None = None,
    ) -> "IndexUnderlying": ...
    @property
    def index_id(self) -> str: ...
    @property
    def base_currency(self) -> Currency: ...

class EquityTotalReturnSwapBuilder:
    """Fluent builder returned by :meth:`EquityTotalReturnSwap.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def notional(self, notional: Money) -> "EquityTotalReturnSwapBuilder": ...
    def underlying(self, underlying: EquityUnderlying) -> "EquityTotalReturnSwapBuilder": ...
    def financing(self, financing: TrsFinancingLegSpec) -> "EquityTotalReturnSwapBuilder": ...
    def schedule(self, schedule: TrsScheduleSpec) -> "EquityTotalReturnSwapBuilder": ...
    def side(self, side: TrsSide) -> "EquityTotalReturnSwapBuilder": ...
    def initial_level(self, initial_level: float | None = ...) -> "EquityTotalReturnSwapBuilder": ...
    def dividend_tax_rate(self, rate: float) -> "EquityTotalReturnSwapBuilder": ...
    def add_discrete_dividend(self, ex_date: date, amount: float) -> "EquityTotalReturnSwapBuilder": ...
    def build(self) -> "EquityTotalReturnSwap": ...

class EquityTotalReturnSwap:
    """Equity total return swap for synthetic equity exposure.

    Examples
    --------
    Create an equity TRS:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.dates.daycount import DayCount
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.cashflow.builder import ScheduleParams
        >>> from finstack.valuations.instruments import (
        ...     EquityTotalReturnSwap,
        ...     EquityUnderlying,
        ...     TrsFinancingLegSpec,
        ...     TrsScheduleSpec,
        ...     TrsSide,
        ... )
        >>> underlying = EquityUnderlying.new(
        ...     ticker="SPX", spot_id="SPX", currency=Currency("USD"), div_yield_id=None, contract_size=None
        ... )
        >>> financing = TrsFinancingLegSpec.new(
        ...     discount_curve="USD",
        ...     forward_curve="USD-LIBOR-3M",
        ...     day_count=DayCount.ACT_360,
        ...     spread_bp=25.0,  # 25bp spread
        ... )
        >>> schedule = TrsScheduleSpec.new(
        ...     start=date(2024, 1, 1),
        ...     end=date(2025, 1, 1),
        ...     schedule_params=ScheduleParams.quarterly_act360(),
        ... )
        >>> trs = (
        ...     EquityTotalReturnSwap
        ...     .builder("TRS-SPX")
        ...     .notional(Money(10_000_000, Currency("USD")))
        ...     .underlying(underlying)
        ...     .financing(financing)
        ...     .schedule(schedule)
        ...     .side(TrsSide.RECEIVE_TOTAL_RETURN)
        ...     .initial_level(4_000.0)
        ...     .build()
        ... )
    """

    @classmethod
    def builder(cls, instrument_id: str) -> EquityTotalReturnSwapBuilder: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    @property
    def side(self) -> str: ...
    def value(self, market: MarketContext, as_of: date) -> Money: ...
    def pv_total_return_leg(self, market: MarketContext, as_of: date) -> Money: ...
    def pv_financing_leg(self, market: MarketContext, as_of: date) -> Money: ...
    def financing_annuity(self, market: MarketContext, as_of: date) -> float: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
