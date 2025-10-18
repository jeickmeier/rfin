"""Portfolio metrics."""

from typing import Optional, Dict, Any
from .valuation import PortfolioValuation

class AggregatedMetric:
    """Aggregated metric across the portfolio.

    Contains portfolio-wide totals as well as breakdowns by entity.

    Examples:
        >>> metric = metrics.get_metric("dv01")
        >>> metric.total
        125.0
        >>> metric.by_entity["ENTITY_A"]
        75.0
    """

    @property
    def metric_id(self) -> str:
        """Get the metric identifier."""
        ...

    @property
    def total(self) -> float:
        """Get the total value across all positions (for summable metrics)."""
        ...

    @property
    def by_entity(self) -> Dict[str, float]:
        """Get aggregated values by entity."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class PortfolioMetrics:
    """Complete portfolio metrics results.

    Holds both aggregated metrics and per-position values.

    Examples:
        >>> metrics = aggregate_metrics(valuation)
        >>> dv01 = metrics.get_metric("dv01")
        >>> position_metrics = metrics.get_position_metrics("POS_1")
    """

    def get_metric(self, metric_id: str) -> Optional[AggregatedMetric]:
        """Get an aggregated metric by identifier.

        Args:
            metric_id: Identifier of the metric to look up.

        Returns:
            AggregatedMetric or None: The metric if found.

        Examples:
            >>> metric = metrics.get_metric("dv01")
        """
        ...

    def get_position_metrics(self, position_id: str) -> Optional[Dict[str, float]]:
        """Get metrics for a specific position.

        Args:
            position_id: Identifier of the position to query.

        Returns:
            dict[str, float] or None: Mapping of metric IDs to values for the position.

        Examples:
            >>> position_metrics = metrics.get_position_metrics("POS_1")
            >>> position_metrics["dv01"]
            5.0
        """
        ...

    def get_total(self, metric_id: str) -> Optional[float]:
        """Get the total value of a specific metric across the portfolio.

        Args:
            metric_id: Identifier of the metric.

        Returns:
            float or None: Total metric value if found.

        Examples:
            >>> total_dv01 = metrics.get_total("dv01")
            >>> total_dv01
            125.0
        """
        ...

    @property
    def aggregated(self) -> Dict[str, AggregatedMetric]:
        """Get aggregated metrics (summable only)."""
        ...

    @property
    def by_position(self) -> Dict[str, Dict[str, float]]:
        """Get raw metrics by position (all metrics)."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

def aggregate_metrics(valuation: PortfolioValuation) -> PortfolioMetrics:
    """Aggregate metrics from portfolio valuation.

    Computes portfolio-wide metrics by summing position-level results where appropriate.
    Only summable metrics (DV01, CS01, Theta, etc.) are aggregated.

    Args:
        valuation: Portfolio valuation results.

    Returns:
        PortfolioMetrics: Aggregated metrics results.

    Raises:
        RuntimeError: If aggregation fails.

    Examples:
        >>> from finstack.portfolio import aggregate_metrics
        >>> metrics = aggregate_metrics(valuation)
        >>> metrics.get_total("dv01")
        125.0
    """
    ...
