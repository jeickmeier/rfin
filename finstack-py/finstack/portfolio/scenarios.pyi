"""Portfolio scenario integration."""

from typing import Optional
from finstack.core.config import FinstackConfig
from finstack.core.market_data.context import MarketContext
from finstack.scenarios.spec import ScenarioSpec
from .portfolio import Portfolio
from .valuation import PortfolioValuation

def apply_scenario(
    portfolio: Portfolio,
    scenario: ScenarioSpec,
    market_context: MarketContext,
) -> Portfolio:
    """Apply a scenario to a portfolio.

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

    Examples
    --------
    Typical usage builds a :class:`ScenarioSpec`, applies it to a portfolio,
    and then values the transformed portfolio with :func:`value_portfolio`.
    """
    ...

def apply_and_revalue(
    portfolio: Portfolio,
    scenario: ScenarioSpec,
    market_context: MarketContext,
    config: Optional[FinstackConfig] = None,
) -> PortfolioValuation:
    """Apply a scenario to a portfolio and revalue it.

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

    Notes
    -----
    This helper simply composes :func:`apply_scenario` and
    :func:`value_portfolio`, returning the resulting
    :class:`PortfolioValuation`.
    """
    ...
