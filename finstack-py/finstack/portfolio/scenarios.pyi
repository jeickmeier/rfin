"""Portfolio scenario integration."""

from __future__ import annotations
from finstack.core.config import FinstackConfig
from finstack.core.market_data.context import MarketContext
from finstack.scenarios import ApplicationReport, ScenarioSpec
from .portfolio import Portfolio
from .valuation import PortfolioValuation

def apply_scenario(
    portfolio: Portfolio,
    scenario: ScenarioSpec,
    market_context: MarketContext,
) -> tuple[Portfolio, MarketContext, ApplicationReport]:
    """Apply a scenario to a portfolio.

    Transforms the portfolio by applying scenario operations. The original portfolio
    is not modified; a new portfolio with transformed positions is returned along
    with the stressed market context and an application report.

    Args:
        portfolio: Portfolio to transform.
        scenario: Scenario specification to apply.
        market_context: Market data context.

    Returns:
        tuple[Portfolio, MarketContext, ApplicationReport]: Transformed portfolio,
            stressed market context, and application report.

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
    config: FinstackConfig | None = None,
) -> tuple[PortfolioValuation, ApplicationReport]:
    """Apply a scenario to a portfolio and revalue it.

    Convenience function that applies a scenario and then values the resulting portfolio.
    Equivalent to calling apply_scenario followed by value_portfolio.

    Args:
        portfolio: Portfolio to transform and value.
        scenario: Scenario specification to apply.
        market_context: Market data context.
        config: Finstack configuration (optional, uses default if not provided).

    Returns:
        tuple[PortfolioValuation, ApplicationReport]: Portfolio valuation results
            and application report.

    Raises:
        RuntimeError: If scenario application or valuation fails.

    Notes
    -----
    This helper simply composes :func:`apply_scenario` and
    :func:`value_portfolio`, returning the resulting
    :class:`PortfolioValuation` and :class:`ApplicationReport`.
    """
    ...
