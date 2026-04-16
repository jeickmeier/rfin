"""Portfolio construction, valuation, optimization, cashflows, scenarios, and metrics.

Bindings for the ``finstack-portfolio`` Rust crate.
"""

from __future__ import annotations

from finstack.finstack import portfolio as _portfolio

parse_portfolio_spec = _portfolio.parse_portfolio_spec
build_portfolio_from_spec = _portfolio.build_portfolio_from_spec
portfolio_result_total_value = _portfolio.portfolio_result_total_value
portfolio_result_get_metric = _portfolio.portfolio_result_get_metric
aggregate_metrics = _portfolio.aggregate_metrics
value_portfolio = _portfolio.value_portfolio
aggregate_cashflows = _portfolio.aggregate_cashflows
apply_scenario_and_revalue = _portfolio.apply_scenario_and_revalue
optimize_portfolio = _portfolio.optimize_portfolio
replay_portfolio = _portfolio.replay_portfolio
parametric_var_decomposition = _portfolio.parametric_var_decomposition
parametric_es_decomposition = _portfolio.parametric_es_decomposition
historical_var_decomposition = _portfolio.historical_var_decomposition
evaluate_risk_budget = _portfolio.evaluate_risk_budget

__all__: list[str] = [
    "aggregate_cashflows",
    "aggregate_metrics",
    "apply_scenario_and_revalue",
    "build_portfolio_from_spec",
    "evaluate_risk_budget",
    "historical_var_decomposition",
    "optimize_portfolio",
    "parametric_es_decomposition",
    "parametric_var_decomposition",
    "parse_portfolio_spec",
    "portfolio_result_get_metric",
    "portfolio_result_total_value",
    "replay_portfolio",
    "value_portfolio",
]
