"""Portfolio construction, valuation, optimization, cashflows, scenarios, and metrics.

Bindings for the ``finstack-portfolio`` Rust crate.
"""

from __future__ import annotations

from finstack.finstack import portfolio as _portfolio

Portfolio = _portfolio.Portfolio
PortfolioValuation = _portfolio.PortfolioValuation
PortfolioResult = _portfolio.PortfolioResult

parse_portfolio_spec = _portfolio.parse_portfolio_spec
build_portfolio_from_spec = _portfolio.build_portfolio_from_spec
portfolio_result_total_value = _portfolio.portfolio_result_total_value
portfolio_result_get_metric = _portfolio.portfolio_result_get_metric
aggregate_metrics = _portfolio.aggregate_metrics
value_portfolio = _portfolio.value_portfolio
aggregate_full_cashflows = _portfolio.aggregate_full_cashflows
apply_scenario_and_revalue = _portfolio.apply_scenario_and_revalue
optimize_portfolio = _portfolio.optimize_portfolio
replay_portfolio = _portfolio.replay_portfolio
parametric_var_decomposition = _portfolio.parametric_var_decomposition
parametric_es_decomposition = _portfolio.parametric_es_decomposition
historical_var_decomposition = _portfolio.historical_var_decomposition
evaluate_risk_budget = _portfolio.evaluate_risk_budget
roll_effective_spread = _portfolio.roll_effective_spread
amihud_illiquidity = _portfolio.amihud_illiquidity
days_to_liquidate = _portfolio.days_to_liquidate
liquidity_tier = _portfolio.liquidity_tier
lvar_bangia = _portfolio.lvar_bangia
almgren_chriss_impact = _portfolio.almgren_chriss_impact
kyle_lambda = _portfolio.kyle_lambda

__all__: list[str] = [
    "Portfolio",
    "PortfolioResult",
    "PortfolioValuation",
    "aggregate_full_cashflows",
    "aggregate_metrics",
    "almgren_chriss_impact",
    "amihud_illiquidity",
    "apply_scenario_and_revalue",
    "build_portfolio_from_spec",
    "days_to_liquidate",
    "evaluate_risk_budget",
    "historical_var_decomposition",
    "kyle_lambda",
    "liquidity_tier",
    "lvar_bangia",
    "optimize_portfolio",
    "parametric_es_decomposition",
    "parametric_var_decomposition",
    "parse_portfolio_spec",
    "portfolio_result_get_metric",
    "portfolio_result_total_value",
    "replay_portfolio",
    "roll_effective_spread",
    "value_portfolio",
]
