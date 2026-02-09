"""Portfolio management and aggregation for finstack.

This module provides portfolio-level operations including entity and position
management, valuation aggregation, metrics calculation, attribute-based grouping,
P&L attribution, margin aggregation, and DataFrame exports for analysis.
"""

from __future__ import annotations

from typing import TypedDict

from finstack.core.config import FinstackConfig
from finstack.core.market_data.context import MarketContext

from .types import Book, BookId, Entity, PositionUnit, Position, DUMMY_ENTITY_ID
from .portfolio import Portfolio
from .builder import PortfolioBuilder
from .valuation import (
    PositionValue,
    PortfolioValuation,
    PortfolioValuationOptions,
    value_portfolio,
    value_portfolio_with_options,
)
from .metrics import AggregatedMetric, PortfolioMetrics, aggregate_metrics, is_summable
from .results import PortfolioResult
from .grouping import group_by_attribute, aggregate_by_attribute, aggregate_by_book
from .attribution import PortfolioAttribution, attribute_portfolio_pnl
from .cashflows import (
    PortfolioCashflows,
    PortfolioCashflowBuckets,
    aggregate_cashflows,
    collapse_cashflows_to_base_by_date,
    cashflows_to_base_by_period,
)
from .dataframe import (
    positions_to_polars,
    entities_to_polars,
    metrics_to_polars,
    aggregated_metrics_to_polars,
)
from .margin import (
    NettingSetId,
    NettingSet,
    NettingSetManager,
    NettingSetMargin,
    PortfolioMarginResult,
    PortfolioMarginAggregator,
)

class MaxYieldWithCccLimitResult(TypedDict):
    """Return shape for :func:`optimize_max_yield_with_ccc_limit`.

    This mirrors the Rust/PyO3 helper output (a dict) but provides IDE-friendly
    typing for notebooks and examples.
    """

    label: str | None
    status: str
    objective_value: float
    ccc_weight: float
    optimal_weights: dict[str, float]
    current_weights: dict[str, float]
    weight_deltas: dict[str, float]

def optimize_max_yield_with_ccc_limit(
    portfolio: Portfolio,
    market_context: MarketContext,
    ccc_limit: float = 0.20,
    strict_risk: bool = False,
    config: FinstackConfig | None = None,
) -> MaxYieldWithCccLimitResult: ...

# Scenario integration (if available)
try:
    from .scenarios import apply_scenario, apply_and_revalue

    __all__ = [
        "Entity",
        "BookId",
        "Book",
        "PositionUnit",
        "Position",
        "DUMMY_ENTITY_ID",
        "Portfolio",
        "PortfolioBuilder",
        "PositionValue",
        "PortfolioValuation",
        "PortfolioValuationOptions",
        "value_portfolio",
        "value_portfolio_with_options",
        "AggregatedMetric",
        "PortfolioMetrics",
        "aggregate_metrics",
        "is_summable",
        "PortfolioResult",
        "group_by_attribute",
        "aggregate_by_attribute",
        "aggregate_by_book",
        "PortfolioAttribution",
        "attribute_portfolio_pnl",
        "PortfolioCashflows",
        "PortfolioCashflowBuckets",
        "aggregate_cashflows",
        "collapse_cashflows_to_base_by_date",
        "cashflows_to_base_by_period",
        "positions_to_polars",
        "entities_to_polars",
        "metrics_to_polars",
        "aggregated_metrics_to_polars",
        "NettingSetId",
        "NettingSet",
        "NettingSetManager",
        "NettingSetMargin",
        "PortfolioMarginResult",
        "PortfolioMarginAggregator",
        "MaxYieldWithCccLimitResult",
        "optimize_max_yield_with_ccc_limit",
        "apply_scenario",
        "apply_and_revalue",
    ]
except ImportError:
    __all__ = [
        "Entity",
        "BookId",
        "Book",
        "PositionUnit",
        "Position",
        "DUMMY_ENTITY_ID",
        "Portfolio",
        "PortfolioBuilder",
        "PositionValue",
        "PortfolioValuation",
        "PortfolioValuationOptions",
        "value_portfolio",
        "value_portfolio_with_options",
        "AggregatedMetric",
        "PortfolioMetrics",
        "aggregate_metrics",
        "is_summable",
        "PortfolioResult",
        "group_by_attribute",
        "aggregate_by_attribute",
        "aggregate_by_book",
        "PortfolioAttribution",
        "attribute_portfolio_pnl",
        "PortfolioCashflows",
        "PortfolioCashflowBuckets",
        "aggregate_cashflows",
        "collapse_cashflows_to_base_by_date",
        "cashflows_to_base_by_period",
        "positions_to_polars",
        "entities_to_polars",
        "metrics_to_polars",
        "aggregated_metrics_to_polars",
        "NettingSetId",
        "NettingSet",
        "NettingSetManager",
        "NettingSetMargin",
        "PortfolioMarginResult",
        "PortfolioMarginAggregator",
        "MaxYieldWithCccLimitResult",
        "optimize_max_yield_with_ccc_limit",
    ]
