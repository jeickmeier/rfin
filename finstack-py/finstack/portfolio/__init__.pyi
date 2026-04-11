"""Portfolio construction, valuation, cashflows, scenarios, and metrics."""

from __future__ import annotations

__all__ = [
    "parse_portfolio_spec",
    "build_portfolio_from_spec",
    "portfolio_result_total_value",
    "portfolio_result_get_metric",
    "aggregate_metrics",
    "value_portfolio",
    "aggregate_cashflows",
    "apply_scenario_and_revalue",
]

def parse_portfolio_spec(json_str: str) -> str:
    """Parse and canonicalize a ``PortfolioSpec`` from JSON.

    Args:
        json_str: JSON-serialized ``PortfolioSpec``.

    Returns:
        Canonical JSON string for the spec.

    Example:
        >>> from finstack.portfolio import parse_portfolio_spec
        >>> canonical_json = parse_portfolio_spec(spec_json)
    """
    ...

def build_portfolio_from_spec(spec_json: str) -> str:
    """Build a runtime portfolio from JSON and return the round-tripped spec.

    Args:
        spec_json: JSON-serialized ``PortfolioSpec``.

    Returns:
        JSON from ``Portfolio::to_spec`` after ``Portfolio::from_spec``.

    Example:
        >>> from finstack.portfolio import build_portfolio_from_spec
        >>> round_tripped = build_portfolio_from_spec(spec_json)
    """
    ...

def portfolio_result_total_value(result_json: str) -> float:
    """Read total portfolio value from a ``PortfolioResult`` JSON envelope.

    Args:
        result_json: JSON-serialized ``PortfolioResult``.

    Returns:
        Total value amount in the result's base currency.

    Example:
        >>> from finstack.portfolio import portfolio_result_total_value
        >>> portfolio_result_total_value(result_json)
        0.0
    """
    ...

def portfolio_result_get_metric(result_json: str, metric_id: str) -> float | None:
    """Read one metric from a ``PortfolioResult`` JSON envelope.

    Args:
        result_json: JSON-serialized ``PortfolioResult``.
        metric_id: Metric key present in the result.

    Returns:
        Metric value, or ``None`` if absent.

    Example:
        >>> from finstack.portfolio import portfolio_result_get_metric
        >>> portfolio_result_get_metric(result_json, "pv")
    """
    ...

def aggregate_metrics(
    valuation_json: str,
    base_ccy: str,
    market_json: str,
    as_of: str,
) -> str:
    """Aggregate portfolio metrics from a valuation JSON snapshot.

    Args:
        valuation_json: JSON-serialized ``PortfolioValuation``.
        base_ccy: Aggregation currency code (e.g. ``"USD"``).
        market_json: JSON-serialized ``MarketContext``.
        as_of: Valuation date in ISO 8601 format.

    Returns:
        JSON-serialized aggregated metrics structure.

    Example:
        >>> from finstack.portfolio import aggregate_metrics
        >>> aggregate_metrics(val_json, "USD", mkt_json, "2025-01-15")
        '{}'
    """
    ...

def value_portfolio(
    spec_json: str,
    market_json: str,
    strict_risk: bool = False,
) -> str:
    """Value a portfolio from its spec and market context.

    Args:
        spec_json: JSON-serialized ``PortfolioSpec``.
        market_json: JSON-serialized ``MarketContext``.
        strict_risk: When ``True``, abort if any risk metric fails.

    Returns:
        JSON-serialized ``PortfolioValuation``.

    Example:
        >>> from finstack.portfolio import value_portfolio
        >>> value_portfolio(spec_json, market_json)
        '{}'
    """
    ...

def aggregate_cashflows(spec_json: str, market_json: str) -> str:
    """Build a cashflow ladder for the portfolio.

    Args:
        spec_json: JSON-serialized ``PortfolioSpec``.
        market_json: JSON-serialized ``MarketContext``.

    Returns:
        JSON-serialized ``PortfolioCashflows`` ladder.

    Example:
        >>> from finstack.portfolio import aggregate_cashflows
        >>> aggregate_cashflows(spec_json, market_json)
        '{}'
    """
    ...

def apply_scenario_and_revalue(
    spec_json: str,
    scenario_json: str,
    market_json: str,
) -> tuple[str, str]:
    """Apply a scenario and revalue the portfolio.

    Args:
        spec_json: JSON-serialized ``PortfolioSpec``.
        scenario_json: JSON-serialized ``ScenarioSpec``.
        market_json: JSON-serialized ``MarketContext``.

    Returns:
        ``(valuation_json, report_json)`` for the stressed portfolio and application report.

    Example:
        >>> from finstack.portfolio import apply_scenario_and_revalue
        >>> val_j, rep_j = apply_scenario_and_revalue(spec_json, scen_json, mkt_json)
    """
    ...
