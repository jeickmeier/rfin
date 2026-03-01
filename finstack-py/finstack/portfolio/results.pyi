"""Portfolio results."""

from __future__ import annotations
from typing import Dict, Any
from finstack.core.money import Money
from .valuation import PortfolioValuation
from .metrics import PortfolioMetrics

class PortfolioResult:
    """Complete results from portfolio evaluation.

    Contains valuation, metrics, and metadata about the calculation. Instances
    are typically produced by higher-level orchestration code that values a
    portfolio, aggregates metrics, and snapshots the active :class:`ResultsMeta`.
    """

    def __init__(
        self,
        valuation: PortfolioValuation,
        metrics: PortfolioMetrics,
        meta: Dict[str, Any],
    ) -> None:
        """Create a new portfolio results instance.

        Args:
            valuation: Portfolio valuation component.
            metrics: Portfolio metrics component.
            meta: Metadata describing calculation context.

        Returns:
            PortfolioResult: New results instance.
        """
        ...

    def total_value(self) -> Money:
        """Get the total portfolio value.

        Returns:
            Money: Total portfolio value in base currency.
        """
        ...

    def get_metric(self, metric_id: str) -> float | None:
        """Get a specific aggregated metric.

        Args:
            metric_id: Identifier of the metric to retrieve.

        Returns:
            float or None: Metric value if found.
        """
        ...

    @property
    def valuation(self) -> PortfolioValuation:
        """Get the portfolio valuation results."""
        ...

    @property
    def metrics(self) -> PortfolioMetrics:
        """Get the aggregated metrics."""
        ...

    @property
    def meta(self) -> Dict[str, Any]:
        """Get metadata about the calculation."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
