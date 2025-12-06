"""Portfolio-level P&L attribution bindings.

Wraps the Rust portfolio attribution engine to attribute portfolio P&L across
factors (carry, rates, credit, FX, vol, etc.) with no Python-side logic.
"""

from typing import Dict, Optional, Union
from datetime import date
from finstack.core.money import Money
from finstack.core.market_data.context import MarketContext
from finstack.core.config import FinstackConfig
from .portfolio import Portfolio
from finstack.valuations.attribution import (
    PnlAttribution,
    RatesCurvesAttribution,
    CreditCurvesAttribution,
    AttributionMethod,
)

DateLike = Union[str, date]

class PortfolioAttribution:
    """Portfolio-level P&L attribution result.

    Contains total portfolio P&L by factor and per-position breakdowns.
    All monetary values are in the portfolio base currency.
    """

    @property
    def total_pnl(self) -> Money:
        """Total portfolio P&L."""
        ...

    @property
    def carry(self) -> Money:
        """Carry P&L (theta + accruals)."""
        ...

    @property
    def rates_curves_pnl(self) -> Money:
        """P&L from rate curve moves."""
        ...

    @property
    def credit_curves_pnl(self) -> Money:
        """P&L from credit curve moves."""
        ...

    @property
    def inflation_curves_pnl(self) -> Money:
        """P&L from inflation curves."""
        ...

    @property
    def correlations_pnl(self) -> Money:
        """P&L from correlation changes."""
        ...

    @property
    def fx_pnl(self) -> Money:
        """P&L from FX rate changes."""
        ...

    @property
    def fx_translation_pnl(self) -> Money:
        """FX translation P&L from instrument currency to portfolio base currency."""
        ...

    @property
    def vol_pnl(self) -> Money:
        """P&L from volatility changes."""
        ...

    @property
    def model_params_pnl(self) -> Money:
        """P&L from model parameter shifts."""
        ...

    @property
    def market_scalars_pnl(self) -> Money:
        """P&L from scalar market inputs."""
        ...

    @property
    def residual(self) -> Money:
        """Residual P&L not explained by other factors."""
        ...

    @property
    def by_position(self) -> Dict[str, PnlAttribution]:
        """Position-level attribution by position_id."""
        ...

    @property
    def rates_detail(self) -> Optional[RatesCurvesAttribution]:
        """Optional detailed rates attribution aggregated across positions."""
        ...

    @property
    def credit_detail(self) -> Optional[CreditCurvesAttribution]:
        """Optional detailed credit attribution aggregated across positions."""
        ...

    def to_csv(self) -> str:
        """Return a one-row CSV summarizing total attribution by factor."""
        ...

    def position_detail_to_csv(self) -> str:
        """Return CSV with one row per position showing factor breakdown."""
        ...

    def explain(self) -> str:
        """Human-readable tree of attribution with percentages."""
        ...

    def __repr__(self) -> str: ...

def attribute_portfolio_pnl(
    portfolio: Portfolio,
    market_t0: MarketContext,
    market_t1: MarketContext,
    as_of_t0: DateLike,
    as_of_t1: DateLike,
    method: AttributionMethod,
    config: Optional[FinstackConfig] = ...,
) -> PortfolioAttribution:
    """Perform portfolio-level P&L attribution.

    Args:
        portfolio: Portfolio to attribute.
        market_t0: Market context at T0.
        market_t1: Market context at T1.
        as_of_t0: Valuation date at T0 (e.g., yesterday).
        as_of_t1: Valuation date at T1 (e.g., today).
        method: Attribution methodology (Parallel, Waterfall, or MetricsBased).
        config: Optional finstack configuration (defaults to library default).

    Returns:
        PortfolioAttribution with totals and per-position breakdowns in base currency.
    """
    ...
