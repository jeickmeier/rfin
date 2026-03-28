"""Portfolio cashflow aggregation bindings."""

from __future__ import annotations
from typing import Dict, List, Tuple
from datetime import date
from finstack.core.money import Money
from finstack.core.currency import Currency
from finstack.core.dates.periods import Period
from finstack.core.market_data.context import MarketContext
from .portfolio import Portfolio

DateLike = str | date

DateMoney = Tuple[DateLike, Money]

class CashflowWarning:
    """Warning emitted when a position's contractual cashflows could not be built."""

    @property
    def position_id(self) -> str: ...

    @property
    def instrument_id(self) -> str: ...

    @property
    def instrument_type(self) -> str: ...

    @property
    def message(self) -> str: ...

    def __repr__(self) -> str: ...

class PortfolioCashflows:
    """Aggregated portfolio cashflows by date and currency."""

    @property
    def by_date(self) -> Dict[DateLike, Dict[str, Money]]:
        """Cashflows keyed by payment date then currency code."""
        ...

    @property
    def by_position(self) -> Dict[str, List[DateMoney]]:
        """Optional per-position cashflows keyed by position_id."""
        ...

    @property
    def warnings(self) -> List[CashflowWarning]:
        """Warnings for positions whose contractual cashflows could not be built."""
        ...

    def __repr__(self) -> str: ...

class PortfolioCashflowBuckets:
    """Cashflows bucketed by reporting period in base currency."""

    @property
    def by_period(self) -> Dict[str, Money]:
        """Map of period identifier to total cashflow in base currency."""
        ...

    def __repr__(self) -> str: ...

def aggregate_cashflows(portfolio: Portfolio, market_context: MarketContext) -> PortfolioCashflows:
    """Collect and aggregate holder-view cashflows across all positions.

    Returns cashflows by date and currency; no FX conversion is applied.
    """
    ...

def collapse_cashflows_to_base_by_date(
    ladder: PortfolioCashflows,
    market_context: MarketContext,
    base_ccy: Currency,
) -> Dict[DateLike, Money]:
    """Convert a multi-currency cashflow ladder into base currency by date."""
    ...

def cashflows_to_base_by_period(
    ladder: PortfolioCashflows,
    market_context: MarketContext,
    base_ccy: Currency,
    periods: List[Period],
) -> PortfolioCashflowBuckets:
    """Bucket base-currency cashflows into reporting periods."""
    ...
