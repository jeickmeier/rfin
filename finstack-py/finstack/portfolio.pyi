# flake8: noqa: PYI021
def py_aggregate_by_attribute(valuation, portfolio, attribute_key):
    """
    Aggregate portfolio valuation by an attribute.

    Sums position values within each attribute group. Only positions with the
    specified attribute key in their tags are included. Values are converted
    to the portfolio base currency before aggregation.

    Args:
        valuation: Portfolio valuation results.
        portfolio: Portfolio containing positions.
        attribute_key: Tag key to group by (e.g., "sector", "rating").

    Returns:
        dict[str, Money]: Mapping of attribute values to aggregated amounts.

    Raises:
        RuntimeError: If aggregation fails.

    Examples:
        >>> from finstack.portfolio import aggregate_by_attribute
        >>> by_sector = aggregate_by_attribute(valuation, portfolio, "sector")
        >>> by_sector["Technology"]
        Money(USD, 5000000.0)
    """

def py_aggregate_metrics(valuation):
    """
    Aggregate metrics from portfolio valuation.

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

def py_apply_and_revalue(portfolio, scenario, market_context, config=None):
    """
    Apply a scenario to a portfolio and revalue it.

    Convenience function that applies a scenario and then values the resulting portfolio.
    Equivalent to calling apply_scenario followed by value_portfolio.

    Args:
        portfolio: Portfolio to transform and value.
        scenario: Scenario specification to apply.
        market_context: Market data context.
        config: Finstack configuration (optional, uses default if not provided).

    Returns:
        PortfolioValuation: Portfolio valuation results.

    Raises:
        RuntimeError: If scenario application or valuation fails.

    Examples:
        >>> from finstack.portfolio import apply_and_revalue
        >>> from finstack.scenarios import ScenarioSpec
        >>> valuation = apply_and_revalue(portfolio, scenario, market_context)
        >>> valuation.total_base_ccy
        Money(USD, 9500000.0)
    """

def py_apply_scenario(portfolio, scenario, market_context):
    """
    Apply a scenario to a portfolio.

    Transforms the portfolio by applying scenario operations. The original portfolio
    is not modified; a new portfolio with transformed positions is returned.

    Args:
        portfolio: Portfolio to transform.
        scenario: Scenario specification to apply.
        market_context: Market data context.

    Returns:
        Portfolio: Transformed portfolio.

    Raises:
        RuntimeError: If scenario application fails.

    Examples:
        >>> from finstack.portfolio import apply_scenario
        >>> from finstack.scenarios import ScenarioSpec
        >>> transformed = apply_scenario(portfolio, scenario, market_context)
    """

def py_group_by_attribute(portfolio, attribute_key):
    """
    Group portfolio positions by an attribute.

    Returns a dictionary mapping attribute values to lists of positions.
    The attribute key must exist in position tags for positions to be included.

    Args:
        portfolio: Portfolio to group.
        attribute_key: Tag key to group by (e.g., "sector", "rating").

    Returns:
        dict[str, list[Position]]: Mapping of attribute values to position lists.

    Raises:
        RuntimeError: If grouping fails.

    Examples:
        >>> from finstack.portfolio import group_by_attribute
        >>> by_sector = group_by_attribute(portfolio, "sector")
        >>> by_sector["Technology"]
        [Position(...), Position(...)]
    """

def py_value_portfolio(portfolio, market_context, config=None):
    """
    Value a complete portfolio.

    Args:
        portfolio: Portfolio to value.
        market_context: Market data context.
        config: Finstack configuration (optional, uses default if not provided).

    Returns:
        PortfolioValuation: Complete valuation results.

    Raises:
        RuntimeError: If valuation fails.

    Examples:
        >>> from finstack.portfolio import value_portfolio
        >>> from finstack.core import FinstackConfig
        >>> valuation = value_portfolio(portfolio, market_context, FinstackConfig())
    """
