"""Statement analysis: sensitivity, variance, scenarios, backtesting, goal seek, DCF, corporate, Monte Carlo, reports, introspection."""

from __future__ import annotations

from typing import Any

__all__ = [
    "run_sensitivity",
    "generate_tornado_entries",
    "run_variance",
    "evaluate_scenario_set",
    "run_monte_carlo",
    "backtest_forecast",
    "goal_seek",
    "evaluate_dcf",
    "run_corporate_analysis",
    "pl_summary_report",
    "credit_assessment_report",
    "trace_dependencies",
    "trace_dependencies_detailed",
    "direct_dependencies",
    "all_dependencies",
    "dependents",
    "explain_formula",
    "explain_formula_text",
    "run_checks",
    "run_three_statement_checks",
    "run_credit_underwriting_checks",
    "render_check_report_text",
    "render_check_report_html",
    "Exposure",
    "classify_stage",
    "compute_ecl",
    "compute_ecl_weighted",
]

def run_sensitivity(model_json: str, config_json: str) -> str:
    """Run sensitivity analysis on a financial model (JSON in/out).

    Args:
        model_json: JSON-serialized ``FinancialModelSpec``.
        config_json: JSON-serialized ``SensitivityConfig``.

    Returns:
        JSON-serialized ``SensitivityResult``.

    Example:
        >>> from finstack.statements_analytics import run_sensitivity
        >>> out = run_sensitivity(model_json, config_json)
    """
    ...

def generate_tornado_entries(
    result_json: str,
    metric_node: str,
    period: str | None = None,
) -> str:
    """Build tornado chart entries from a sensitivity result (JSON in/out).

    Args:
        result_json: JSON-serialized ``SensitivityResult``.
        metric_node: Node ID to extract tornado entries for.
        period: Optional period string to pin the tornado to.

    Returns:
        JSON-serialized list of ``TornadoEntry``.

    Example:
        >>> from finstack.statements_analytics import generate_tornado_entries
        >>> entries_json = generate_tornado_entries(res_json, "ebitda", "2025Q4")
    """
    ...

def run_variance(base_json: str, comparison_json: str, config_json: str) -> str:
    """Run variance analysis comparing two statement results (JSON in/out).

    Args:
        base_json: JSON-serialized baseline ``StatementResult``.
        comparison_json: JSON-serialized comparison ``StatementResult``.
        config_json: JSON-serialized ``VarianceConfig``.

    Returns:
        JSON-serialized variance report.

    Example:
        >>> from finstack.statements_analytics import run_variance
        >>> report_json = run_variance(base_json, cmp_json, cfg_json)
    """
    ...

def evaluate_scenario_set(model_json: str, scenario_set_json: str) -> str:
    """Evaluate every scenario in a scenario set against a model (JSON in/out).

    Args:
        model_json: JSON-serialized ``FinancialModelSpec``.
        scenario_set_json: JSON-serialized ``ScenarioSet``.

    Returns:
        JSON object mapping scenario name to ``StatementResult`` JSON.

    Example:
        >>> from finstack.statements_analytics import evaluate_scenario_set
        >>> results_map_json = evaluate_scenario_set(model_json, set_json)
    """
    ...

def run_monte_carlo(model_json: str, config_json: str) -> str:
    """Run Monte Carlo simulation on a financial model (JSON in/out).

    Args:
        model_json: JSON-serialized ``FinancialModelSpec``.
        config_json: JSON-serialized ``MonteCarloConfig`` (``n_paths``, ``seed``, optional ``percentiles``).

    Returns:
        JSON-serialized ``MonteCarloResults``.

    Example:
        >>> from finstack.statements_analytics import run_monte_carlo
        >>> mc_json = run_monte_carlo(model_json, mc_cfg_json)
    """
    ...

def backtest_forecast(actual: list[float], forecast: list[float]) -> dict[str, float | int]:
    """Compute forecast accuracy metrics (MAE, MAPE, RMSE).

    Args:
        actual: Observed values.
        forecast: Predicted values (same length as ``actual``).

    Returns:
        Dict with keys ``mae``, ``mape``, ``rmse``, and ``n``.

    Example:
        >>> from finstack.statements_analytics import backtest_forecast
        >>> backtest_forecast([1.0, 2.0], [1.1, 1.9])["mae"]
        0.1
    """
    ...

def goal_seek(
    model_json: str,
    target_node: str,
    target_period: str,
    target_value: float,
    driver_node: str,
    driver_period: str,
    update_model: bool = True,
    bounds: tuple[float, float] | None = None,
) -> tuple[float, str]:
    """Find the driver value that makes a target node hit a target value.

    Args:
        model_json: JSON-serialized ``FinancialModelSpec``.
        target_node: Node optimized toward ``target_value``.
        target_period: Period string for the target (e.g. ``"2025Q4"``).
        target_value: Desired value for the target node.
        driver_node: Node adjusted to reach the target.
        driver_period: Period string for the driver.
        update_model: If ``True``, write the solved value back into the returned model JSON.
        bounds: Optional ``(lo, hi)`` search bounds for bisection.

    Returns:
        ``(solved_driver_value, updated_model_json)``.

    Example:
        >>> from finstack.statements_analytics import goal_seek
        >>> solved, new_model = goal_seek(mj, "ni", "2025", 10.0, "rev", "2025")
    """
    ...

def evaluate_dcf(
    model_json: str,
    wacc: float,
    terminal_value_json: str,
    ufcf_node: str = "ufcf",
    net_debt_override: float | None = None,
    mid_year_convention: bool = False,
    shares_outstanding: float | None = None,
    equity_bridge_json: str | None = None,
    valuation_discounts_json: str | None = None,
    market_json: str | None = None,
) -> dict[str, float | str]:
    """Evaluate DCF valuation on a financial model.

    Args:
        model_json: JSON ``FinancialModelSpec`` (metadata must include ``currency``).
        wacc: Weighted average cost of capital as a decimal (``0.10`` = 10%).
        terminal_value_json: JSON ``TerminalValueSpec`` (tagged enum).
        ufcf_node: Node ID for unlevered free cash flow.
        net_debt_override: Optional flat net debt.
        mid_year_convention: Use mid-year discounting when ``True``.
        shares_outstanding: Optional basic shares for per-share equity value.
        equity_bridge_json: Optional JSON ``EquityBridge``.
        valuation_discounts_json: Optional JSON ``ValuationDiscounts`` (DLOM, DLOC).
        market_json: Optional JSON ``MarketContext`` for curve-based discounting.

    Returns:
        Dict with ``equity_value``, ``equity_currency``, ``enterprise_value``, ``net_debt``,
        ``terminal_value_pv``, ``equity_value_per_share``, ``diluted_shares``.

    Example:
        >>> from finstack.statements_analytics import evaluate_dcf
        >>> dcf = evaluate_dcf(mj, 0.09, tv_json)
        >>> float(dcf["equity_value"])
        0.0
    """
    ...

def run_corporate_analysis(
    model_json: str,
    wacc: float | None = None,
    terminal_value_json: str | None = None,
    net_debt_override: float | None = None,
    coverage_node: str = "ebitda",
    market_json: str | None = None,
    as_of: str | None = None,
) -> dict[str, Any]:
    """Run statements plus optional DCF equity and credit context.

    Args:
        model_json: JSON-serialized ``FinancialModelSpec``.
        wacc: If set, enables DCF at this discount rate (decimal).
        terminal_value_json: Required JSON ``TerminalValueSpec`` when ``wacc`` is set.
        net_debt_override: Optional flat net debt for the equity bridge.
        coverage_node: Node for DSCR / interest coverage (default ``ebitda``).
        market_json: Optional JSON ``MarketContext``.
        as_of: Optional ISO 8601 valuation date string.

    Returns:
        Dict with ``statement_json``, optional ``equity`` scalars, and ``credit`` (instrument_id → metrics JSON).

    Example:
        >>> from finstack.statements_analytics import run_corporate_analysis
        >>> out = run_corporate_analysis(model_json, wacc=0.1, terminal_value_json=tv_json)
    """
    ...

def pl_summary_report(
    results_json: str,
    line_items: list[str],
    periods: list[str],
) -> str:
    """Render a P&L summary report as formatted text.

    Args:
        results_json: JSON-serialized ``StatementResult``.
        line_items: Node IDs to include as rows.
        periods: Period strings for columns (e.g. ``["2025Q1", "2025Q2"]``).

    Returns:
        Formatted report text.

    Example:
        >>> from finstack.statements_analytics import pl_summary_report
        >>> text = pl_summary_report(res_json, ["rev", "cogs"], ["2025Q1"])
    """
    ...

def credit_assessment_report(results_json: str, as_of: str) -> str:
    """Render a credit assessment report as formatted text.

    Args:
        results_json: JSON-serialized ``StatementResult``.
        as_of: Period string for the as-of date (e.g. ``"2025Q1"``).

    Returns:
        Formatted credit report text.

    Example:
        >>> from finstack.statements_analytics import credit_assessment_report
        >>> report = credit_assessment_report(res_json, "2025Q1")
    """
    ...

def trace_dependencies(model_json: str, node_id: str) -> str:
    """Return an ASCII dependency tree for a node.

    Args:
        model_json: JSON-serialized ``FinancialModelSpec``.
        node_id: Node to trace.

    Returns:
        ASCII-formatted dependency tree.

    Example:
        >>> from finstack.statements_analytics import trace_dependencies
        >>> tree = trace_dependencies(model_json, "ebitda")
    """
    ...

def trace_dependencies_detailed(
    model_json: str,
    results_json: str,
    node_id: str,
    period: str,
) -> str:
    """Return a detailed ASCII dependency tree with values for one period.

    Args:
        model_json: JSON-serialized ``FinancialModelSpec``.
        results_json: JSON-serialized ``StatementResult``.
        node_id: Node to trace.
        period: Period string (e.g. ``"2025Q1"``).

    Returns:
        ASCII tree including values for the period.

    Example:
        >>> from finstack.statements_analytics import trace_dependencies_detailed
        >>> text = trace_dependencies_detailed(mj, rj, "ebitda", "2025Q1")
    """
    ...

def direct_dependencies(model_json: str, node_id: str) -> list[str]:
    """List immediate dependencies of a node.

    Args:
        model_json: JSON-serialized ``FinancialModelSpec``.
        node_id: Node whose direct dependencies are listed.

    Returns:
        Direct dependency node IDs.

    Example:
        >>> from finstack.statements_analytics import direct_dependencies
        >>> deps = direct_dependencies(model_json, "ebitda")
    """
    ...

def all_dependencies(model_json: str, node_id: str) -> list[str]:
    """List all transitive dependencies of a node in dependency order.

    Args:
        model_json: JSON-serialized ``FinancialModelSpec``.
        node_id: Root node for the dependency walk.

    Returns:
        Transitive dependency node IDs.

    Example:
        >>> from finstack.statements_analytics import all_dependencies
        >>> chain = all_dependencies(model_json, "ni")
    """
    ...

def dependents(model_json: str, node_id: str) -> list[str]:
    """List nodes that depend on the given node (reverse dependencies).

    Args:
        model_json: JSON-serialized ``FinancialModelSpec``.
        node_id: Node whose dependents are listed.

    Returns:
        Dependent node IDs.

    Example:
        >>> from finstack.statements_analytics import dependents
        >>> rev_deps = dependents(model_json, "rev")
    """
    ...

def explain_formula(
    model_json: str,
    results_json: str,
    node_id: str,
    period: str,
) -> dict[str, Any]:
    """Structured formula explanation for a node and period.

    Args:
        model_json: JSON-serialized ``FinancialModelSpec``.
        results_json: JSON-serialized ``StatementResult``.
        node_id: Node to explain.
        period: Period string.

    Returns:
        Dict with ``node_id``, ``period_id``, ``final_value``, ``node_type``, ``formula_text``,
        and ``breakdown`` (list of component dicts: ``component``, ``value``, ``operation``).

    Example:
        >>> from finstack.statements_analytics import explain_formula
        >>> detail = explain_formula(mj, rj, "rev", "2025Q1")
    """
    ...

def explain_formula_text(
    model_json: str,
    results_json: str,
    node_id: str,
    period: str,
) -> str:
    """Human-readable multi-line formula explanation.

    Args:
        model_json: JSON-serialized ``FinancialModelSpec``.
        results_json: JSON-serialized ``StatementResult``.
        node_id: Node to explain.
        period: Period string.

    Returns:
        Detailed text explanation.

    Example:
        >>> from finstack.statements_analytics import explain_formula_text
        >>> text = explain_formula_text(mj, rj, "rev", "2025Q1")
    """
    ...

def run_checks(model_json: str, suite_spec_json: str) -> str:
    """Run checks from a suite spec against a model (JSON in/out).

    Resolves both built-in and formula checks, evaluates the model,
    and returns a full check report.

    Args:
        model_json: JSON-serialized ``FinancialModelSpec``.
        suite_spec_json: JSON-serialized ``CheckSuiteSpec``.

    Returns:
        JSON-serialized ``CheckReport``.

    Example:
        >>> from finstack.statements_analytics import run_checks
        >>> report_json = run_checks(model_json, suite_spec_json)
    """
    ...

def run_three_statement_checks(model_json: str, mapping_json: str) -> str:
    """Run three-statement checks using a node mapping (JSON in/out).

    Args:
        model_json: JSON-serialized ``FinancialModelSpec``.
        mapping_json: JSON-serialized ``ThreeStatementMapping``.

    Returns:
        JSON-serialized ``CheckReport``.

    Example:
        >>> from finstack.statements_analytics import run_three_statement_checks
        >>> report_json = run_three_statement_checks(model_json, mapping_json)
    """
    ...

def run_credit_underwriting_checks(model_json: str, mapping_json: str) -> str:
    """Run credit underwriting checks using a node mapping (JSON in/out).

    Args:
        model_json: JSON-serialized ``FinancialModelSpec``.
        mapping_json: JSON-serialized ``CreditMapping``.

    Returns:
        JSON-serialized ``CheckReport``.

    Example:
        >>> from finstack.statements_analytics import run_credit_underwriting_checks
        >>> report_json = run_credit_underwriting_checks(model_json, mapping_json)
    """
    ...

def render_check_report_text(report_json: str) -> str:
    """Render a check report as plain text.

    Args:
        report_json: JSON-serialized ``CheckReport``.

    Returns:
        Human-readable plain-text report.

    Example:
        >>> from finstack.statements_analytics import render_check_report_text
        >>> text = render_check_report_text(report_json)
    """
    ...

def render_check_report_html(report_json: str) -> str:
    """Render a check report as HTML with inline styles.

    Args:
        report_json: JSON-serialized ``CheckReport``.

    Returns:
        HTML-formatted report suitable for Jupyter notebooks.

    Example:
        >>> from finstack.statements_analytics import render_check_report_html
        >>> html = render_check_report_html(report_json)
    """
    ...

class Exposure:
    """A single credit exposure for ECL / IFRS 9 / CECL computation.

    All monetary fields are in the exposure's base currency; all rates and
    probabilities are expressed as decimals (``0.05`` = 5%).
    """

    id: str
    ead: float
    lgd: float
    eir: float
    remaining_maturity: float
    current_pd: float
    origination_pd: float
    dpd: int

    def __init__(
        self,
        id: str,
        ead: float,
        lgd: float,
        eir: float,
        remaining_maturity: float,
        current_pd: float,
        origination_pd: float,
        dpd: int = 0,
    ) -> None: ...

def classify_stage(
    exposure: Exposure,
    pd_delta_stage2: float = 0.01,
    dpd_30_trigger: bool = True,
    dpd_90_trigger: bool = True,
) -> tuple[str, str]:
    """Classify an exposure into an IFRS 9 stage.

    Args:
        exposure: Credit exposure.
        pd_delta_stage2: Absolute PD increase threshold (decimal) for SICR.
        dpd_30_trigger: Apply the 30-DPD Stage 2 rebuttable backstop.
        dpd_90_trigger: Apply the 90-DPD Stage 3 non-rebuttable backstop.

    Returns:
        ``(stage, trigger_reason)`` where stage is ``"Stage 1"``, ``"Stage 2"``,
        or ``"Stage 3"``.
    """
    ...

def compute_ecl(
    ead: float,
    pd_schedule: list[tuple[float, float]],
    lgd: float,
    eir: float,
    max_horizon_years: float,
    bucket_width_years: float = 0.25,
    stage: str = "stage1",
) -> float:
    """Compute single-scenario ECL for one exposure.

    Args:
        ead: Exposure at default.
        pd_schedule: ``[(time_years, cumulative_pd), ...]`` knots.
        lgd: Loss given default (decimal).
        eir: Effective interest rate (decimal).
        max_horizon_years: Remaining maturity cap.
        bucket_width_years: Time-bucket width (e.g. ``0.25`` for quarterly).
        stage: ``"stage1"``, ``"stage2"``, or ``"stage3"``.

    Returns:
        ECL amount in the exposure's base currency.
    """
    ...

def compute_ecl_weighted(
    ead: float,
    scenarios: list[tuple[float, list[tuple[float, float]]]],
    lgd: float,
    eir: float,
    max_horizon: float,
    stage: str = "stage1",
) -> float:
    """Compute probability-weighted ECL across macro scenarios.

    Args:
        ead: Exposure at default.
        scenarios: List of ``(weight, pd_schedule)``. Weights must sum to 1.0.
        lgd: Loss given default (decimal).
        eir: Effective interest rate (decimal).
        max_horizon: Remaining maturity cap (years).
        stage: ``"stage1"``, ``"stage2"``, or ``"stage3"``.

    Returns:
        Probability-weighted ECL amount.
    """
    ...
