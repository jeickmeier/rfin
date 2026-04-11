"""Statement analysis: sensitivity, variance, scenarios, backtesting, and more.

Goal seek, DCF, corporate analysis, Monte Carlo, reports, and introspection.
Bindings for the ``finstack-statements-analytics`` Rust crate.
"""

from __future__ import annotations

from finstack.finstack import statements_analytics as _sa

run_sensitivity = _sa.run_sensitivity
generate_tornado_entries = _sa.generate_tornado_entries
run_variance = _sa.run_variance
evaluate_scenario_set = _sa.evaluate_scenario_set
run_monte_carlo = _sa.run_monte_carlo
backtest_forecast = _sa.backtest_forecast
goal_seek = _sa.goal_seek
evaluate_dcf = _sa.evaluate_dcf
run_corporate_analysis = _sa.run_corporate_analysis
pl_summary_report = _sa.pl_summary_report
credit_assessment_report = _sa.credit_assessment_report
trace_dependencies = _sa.trace_dependencies
trace_dependencies_detailed = _sa.trace_dependencies_detailed
direct_dependencies = _sa.direct_dependencies
all_dependencies = _sa.all_dependencies
dependents = _sa.dependents
explain_formula = _sa.explain_formula
explain_formula_text = _sa.explain_formula_text

__all__: list[str] = [
    "all_dependencies",
    "backtest_forecast",
    "credit_assessment_report",
    "dependents",
    "direct_dependencies",
    "evaluate_dcf",
    "evaluate_scenario_set",
    "explain_formula",
    "explain_formula_text",
    "generate_tornado_entries",
    "goal_seek",
    "pl_summary_report",
    "run_corporate_analysis",
    "run_monte_carlo",
    "run_sensitivity",
    "run_variance",
    "trace_dependencies",
    "trace_dependencies_detailed",
]
