"""Portfolio management and aggregation for finstack.

This module provides portfolio-level operations including entity and position
management, valuation aggregation, metrics calculation, attribute-based grouping,
P&L attribution, margin aggregation, and DataFrame exports for analysis.
"""

from __future__ import annotations

from finstack.core.config import FinstackConfig
from finstack.core.market_data.context import MarketContext

from .types import Book, BookId, Entity, PositionUnit, Position, PositionSpec, DUMMY_ENTITY_ID
from .portfolio import Portfolio, PortfolioSpec
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
from .grouping import group_by_attribute, aggregate_by_attribute, aggregate_by_book, aggregate_by_multiple_attributes
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
from .optimization import (
    WeightingScheme,
    MissingMetricPolicy,
    Inequality,
    OptimizationStatus,
    TradeDirection,
    TradeType,
    PerPositionMetric,
    MetricExpr,
    Objective,
    PositionFilter,
    Constraint,
    TradeSpec,
    OptimizationResult,
    CandidatePosition,
    TradeUniverse,
    PortfolioOptimizationProblem,
    MaxYieldWithCccLimitResult,
    DefaultLpOptimizer,
)

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
    from finstack.scenarios import ApplicationReport

    __all__ = [
        "Entity",
        "BookId",
        "Book",
        "PositionUnit",
        "Position",
        "PositionSpec",
        "DUMMY_ENTITY_ID",
        "Portfolio",
        "PortfolioSpec",
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
        "aggregate_by_multiple_attributes",
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
        "WeightingScheme",
        "MissingMetricPolicy",
        "Inequality",
        "OptimizationStatus",
        "TradeDirection",
        "TradeType",
        "PerPositionMetric",
        "MetricExpr",
        "Objective",
        "PositionFilter",
        "Constraint",
        "TradeSpec",
        "OptimizationResult",
        "CandidatePosition",
        "TradeUniverse",
        "PortfolioOptimizationProblem",
        "DefaultLpOptimizer",
        "apply_scenario",
        "apply_and_revalue",
        "ApplicationReport",
    ]
except ImportError:
    __all__ = [
        "Entity",
        "BookId",
        "Book",
        "PositionUnit",
        "Position",
        "PositionSpec",
        "DUMMY_ENTITY_ID",
        "Portfolio",
        "PortfolioSpec",
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
        "aggregate_by_multiple_attributes",
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
        "WeightingScheme",
        "MissingMetricPolicy",
        "Inequality",
        "OptimizationStatus",
        "TradeDirection",
        "TradeType",
        "PerPositionMetric",
        "MetricExpr",
        "Objective",
        "PositionFilter",
        "Constraint",
        "TradeSpec",
        "OptimizationResult",
        "CandidatePosition",
        "TradeUniverse",
        "PortfolioOptimizationProblem",
        "DefaultLpOptimizer",
    ]
