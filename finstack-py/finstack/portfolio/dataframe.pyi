"""Polars DataFrame exports for portfolio outputs."""

from pyo3_polars import PyDataFrame
from .valuation import PortfolioValuation
from .metrics import PortfolioMetrics

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
