"""Portfolio metrics."""

from typing import Optional, Dict, Any
from .valuation import PortfolioValuation

class AggregatedMetric:
    """Aggregated metric across the portfolio.

    Contains portfolio-wide totals as well as breakdowns by entity.

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

    Examples
    --------
        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.portfolio import (
        ...     PortfolioBuilder,
        ...     Entity,
        ...     Position,
        ...     PositionUnit,
        ...     value_portfolio,
        ...     aggregate_metrics,
        ... )
        >>> from finstack.valuations.instruments import Equity
        >>> entity = Entity("ACME")
        >>> equity = Equity.create("EQ-ACME", ticker="ACME", currency=Currency("USD"), price=120.0)
        >>> position = Position("POS-1", entity.id, equity.instrument_id, equity, 100.0, PositionUnit.UNITS)
        >>> portfolio = (
        ...     PortfolioBuilder("FUND_A")
        ...     .base_ccy(Currency("USD"))
        ...     .as_of(date(2025, 1, 1))
        ...     .entity(entity)
        ...     .position(position)
        ...     .build()
        ... )
        >>> valuation = value_portfolio(portfolio, MarketContext())
        >>> metrics = aggregate_metrics(valuation)
        >>> metrics.get_total("delta")
        0.0
    """

    def get_metric(self, metric_id: str) -> Optional[AggregatedMetric]:
        """Get an aggregated metric by identifier.

        Args:
            metric_id: Identifier of the metric to look up.

        Returns:
            AggregatedMetric or None: The metric if found.

        """
        ...

    def get_position_metrics(self, position_id: str) -> Optional[Dict[str, float]]:
        """Get metrics for a specific position.

        Args:
            position_id: Identifier of the position to query.

        Returns:
            dict[str, float] or None: Mapping of metric IDs to values for the position.

        """
        ...

    def get_total(self, metric_id: str) -> Optional[float]:
        """Get the total value of a specific metric across the portfolio.

        Args:
            metric_id: Identifier of the metric.

        Returns:
            float or None: Total metric value if found.

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

def aggregate_metrics(valuation: PortfolioValuation) -> PortfolioMetrics: ...

"""Aggregate risk metrics from portfolio valuation.

Computes portfolio-wide risk metrics by summing position-level results
where appropriate. Only summable metrics (DV01, CS01, Theta, etc.) are
aggregated. Non-summable metrics (yield, spread) are available per-position
but not aggregated.

Parameters
----------
valuation : PortfolioValuation
    Portfolio valuation results from value_portfolio(). Must include
    position-level metrics in ValuationResult.measures for each position.

Returns
-------
PortfolioMetrics
    Aggregated metrics results containing:
    - Aggregated metrics (portfolio totals and by-entity breakdowns)
    - Per-position metrics (all metrics for each position)

Raises
------
RuntimeError
    If aggregation fails (missing metrics, calculation errors).

Examples
--------
Aggregate metrics for a simple portfolio:

    >>> from datetime import date
    >>> from finstack.core.currency import Currency
    >>> from finstack.core.market_data.context import MarketContext
    >>> from finstack.portfolio import (
    ...     PortfolioBuilder,
    ...     Entity,
    ...     Position,
    ...     PositionUnit,
    ...     value_portfolio,
    ...     aggregate_metrics,
    ... )
    >>> from finstack.valuations.instruments import Equity
    >>> entity = Entity("ACME")
    >>> equity = Equity.create("EQ-ACME", ticker="ACME", currency=Currency("USD"), price=120.0)
    >>> position = Position("POS-1", entity.id, equity.instrument_id, equity, 100.0, PositionUnit.UNITS)
    >>> portfolio = (
    ...     PortfolioBuilder("FUND_A")
    ...     .base_ccy(Currency("USD"))
    ...     .as_of(date(2025, 1, 1))
    ...     .entity(entity)
    ...     .position(position)
    ...     .build()
    ... )
    >>> valuation = value_portfolio(portfolio, MarketContext())
    >>> metrics = aggregate_metrics(valuation)
    >>> metrics.get_total("delta")
    0.0

Notes
-----
- Only summable metrics are aggregated (DV01, CS01, Theta, etc.)
- Non-summable metrics (yield, spread) are available per-position only
- Aggregation sums values across positions (currency conversion handled)
- Entity-level aggregation sums all positions for that entity

See Also
--------
:class:`PortfolioMetrics`: Metrics result structure
:class:`AggregatedMetric`: Aggregated metric structure
:func:`value_portfolio`: Portfolio valuation
"""


def is_summable(metric_id: str) -> bool:
    """Return True if a metric can be meaningfully summed across positions (e.g., dv01)."""
    ...
