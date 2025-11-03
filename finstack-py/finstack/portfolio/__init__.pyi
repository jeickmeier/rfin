"""Portfolio management and aggregation for finstack.

This module provides portfolio-level operations including entity and position
management, valuation aggregation, metrics calculation, attribute-based grouping,
and DataFrame exports for analysis.
"""

from .types import Entity, PositionUnit, Position
from .portfolio import Portfolio
from .builder import PortfolioBuilder
from .valuation import PositionValue, PortfolioValuation, value_portfolio
from .metrics import AggregatedMetric, PortfolioMetrics, aggregate_metrics
from .results import PortfolioResults
from .grouping import group_by_attribute, aggregate_by_attribute

# Scenario integration (if available)
try:
    from .scenarios import apply_scenario, apply_and_revalue

    __all__ = [
        "Entity",
        "PositionUnit",
        "Position",
        "Portfolio",
        "PortfolioBuilder",
        "PositionValue",
        "PortfolioValuation",
        "value_portfolio",
        "AggregatedMetric",
        "PortfolioMetrics",
        "aggregate_metrics",
        "PortfolioResults",
        "group_by_attribute",
        "aggregate_by_attribute",
        "apply_scenario",
        "apply_and_revalue",
    ]
except ImportError:
    __all__ = [
        "Entity",
        "PositionUnit",
        "Position",
        "Portfolio",
        "PortfolioBuilder",
        "PositionValue",
        "PortfolioValuation",
        "value_portfolio",
        "AggregatedMetric",
        "PortfolioMetrics",
        "aggregate_metrics",
        "PortfolioResults",
        "group_by_attribute",
        "aggregate_by_attribute",
    ]
