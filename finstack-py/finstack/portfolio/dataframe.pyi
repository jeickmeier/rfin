"""Polars DataFrame exports for portfolio outputs."""

from __future__ import annotations
from typing import Any
from .valuation import PortfolioValuation
from .metrics import PortfolioMetrics

PyDataFrame = Any

def positions_to_polars(valuation: PortfolioValuation) -> PyDataFrame:
    """Return per-position values as a Polars DataFrame."""
    ...

def entities_to_polars(valuation: PortfolioValuation) -> PyDataFrame:
    """Return entity-level aggregates as a Polars DataFrame."""
    ...

def metrics_to_polars(metrics: PortfolioMetrics) -> PyDataFrame:
    """Return per-position metric values in long format."""
    ...

def aggregated_metrics_to_polars(metrics: PortfolioMetrics) -> PyDataFrame:
    """Return aggregated metric totals as a Polars DataFrame."""
    ...
