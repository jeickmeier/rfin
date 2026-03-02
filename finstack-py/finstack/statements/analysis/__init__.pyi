"""Analysis tools for financial statement models.

This module provides tools for:
- Sensitivity analysis - Parameter sweeps and tornado charts
- Dependency tracing - Identify direct and transitive dependencies
- Formula explanation - Break down calculations step-by-step
- Reports - Formatted output for P&L summaries and credit assessment
"""

from __future__ import annotations
from datetime import date
from typing import Any, Dict, List
from ..evaluator import StatementResult, DependencyGraph
from ..types import FinancialModelSpec, NodeType
from ..capital_structure import CapitalStructureCashflows
from ...core.dates.periods import Period, PeriodId
from ...core.money import Money
from ...core.market_data.context import MarketContext
from ...valuations.covenants import CovenantSpec, CovenantForecast, CovenantForecastConfig, FutureBreach
from ...valuations.instruments.equity.dcf import TerminalValueSpec

# =============================================================================
# Sensitivity Analysis
# =============================================================================

class SensitivityMode:
    """Sensitivity analysis mode."""

    DIAGONAL: SensitivityMode
    FULL_GRID: SensitivityMode
    TORNADO: SensitivityMode

    def __repr__(self) -> str: ...

class ParameterSpec:
    """Parameter specification for sensitivity analysis."""

    def __init__(self, node_id: str, period_id: PeriodId, base_value: float, perturbations: List[float]) -> None:
        """Create a new parameter specification.

        Args:
            node_id: Node identifier
            period_id: Period to vary
            base_value: Base value
            perturbations: Perturbations to apply
        """
        ...

    @staticmethod
    def with_percentages(node_id: str, period_id: PeriodId, base_value: float, pct_range: List[float]) -> ParameterSpec:
        """Create a parameter spec with percentage perturbations.

        Args:
            node_id: Node identifier
            period_id: Period to vary
            base_value: Base value
            pct_range: Percentage range (e.g., [-10.0, 0.0, 10.0] for ±10%)

        Returns:
            ParameterSpec: Parameter specification
        """
        ...

    @property
    def node_id(self) -> str: ...
    @property
    def period_id(self) -> PeriodId: ...
    @property
    def base_value(self) -> float: ...
    @property
    def perturbations(self) -> List[float]: ...
    def __repr__(self) -> str: ...

class SensitivityConfig:
    """Sensitivity analysis configuration."""

    def __init__(self, mode: SensitivityMode) -> None:
        """Create a new sensitivity configuration.

        Args:
            mode: Analysis mode
        """
        ...

    def add_parameter(self, param: ParameterSpec) -> None:
        """Add a parameter to vary.

        Args:
            param: Parameter specification
        """
        ...

    def add_target_metric(self, metric: str) -> None:
        """Add a target metric to track.

        Args:
            metric: Metric identifier
        """
        ...

    @property
    def mode(self) -> SensitivityMode: ...
    @property
    def parameters(self) -> List[ParameterSpec]: ...
    @property
    def target_metrics(self) -> List[str]: ...
    def __repr__(self) -> str: ...

class SensitivityScenario:
    """Result of a single sensitivity scenario."""

    @property
    def parameter_values(self) -> Dict[str, float]: ...
    @property
    def results(self) -> StatementResult: ...
    def __repr__(self) -> str: ...

class SensitivityResult:
    """Results of sensitivity analysis."""

    @property
    def config(self) -> SensitivityConfig: ...
    @property
    def scenarios(self) -> List[SensitivityScenario]: ...
    def __len__(self) -> int: ...
    def __repr__(self) -> str: ...

class SensitivityAnalyzer:
    """Sensitivity analyzer for financial statement models."""

    def __init__(self, model: FinancialModelSpec) -> None:
        """Create a new sensitivity analyzer.

        Args:
            model: Financial model to analyze
        """
        ...

    def run(self, config: SensitivityConfig) -> SensitivityResult:
        """Run sensitivity analysis.

        Args:
            config: Analysis configuration

        Returns:
            SensitivityResult: Analysis results
        """
        ...

    def __repr__(self) -> str: ...

# =============================================================================
# Variance Analysis
# =============================================================================

class VarianceConfig:
    """Configuration for variance analysis."""

    def __init__(
        self,
        baseline_label: str,
        comparison_label: str,
        periods: List[PeriodId],
        metrics: List[str],
    ) -> None:
        """Create a new variance configuration.

        Args:
            baseline_label: Human-readable label for the baseline scenario.
            comparison_label: Human-readable label for the comparison scenario.
            periods: Periods to include in the variance report.
            metrics: Node identifiers to compare.
        """
        ...

    @property
    def baseline_label(self) -> str: ...
    @property
    def comparison_label(self) -> str: ...
    @property
    def metrics(self) -> List[str]: ...
    @property
    def periods(self) -> List[PeriodId]: ...
    def __repr__(self) -> str: ...

class VarianceRow:
    """One row of variance output."""

    @property
    def period(self) -> PeriodId: ...
    @property
    def metric(self) -> str: ...
    @property
    def baseline(self) -> float: ...
    @property
    def comparison(self) -> float: ...
    @property
    def abs_var(self) -> float: ...
    @property
    def pct_var(self) -> float: ...
    @property
    def driver_contribution(self) -> List[tuple[str, float]]: ...
    def __repr__(self) -> str: ...

class VarianceReport:
    """Variance report between a baseline and comparison scenario."""

    @property
    def baseline_label(self) -> str: ...
    @property
    def comparison_label(self) -> str: ...
    @property
    def rows(self) -> List[VarianceRow]: ...
    def to_polars(self) -> Any:
        """Export variance rows to a Polars DataFrame.

        Returns:
            polars.DataFrame: DataFrame with period, metric, baseline, comparison, abs_var, pct_var, driver_contribution.
        """
        ...

    def __repr__(self) -> str: ...

class BridgeStep:
    """Single driver contribution in a bridge chart."""

    @property
    def driver(self) -> str: ...
    @property
    def contribution(self) -> float: ...
    def __repr__(self) -> str: ...

class BridgeChart:
    """Bridge chart for a single metric and period."""

    @property
    def target_metric(self) -> str: ...
    @property
    def period(self) -> PeriodId: ...
    @property
    def baseline_label(self) -> str: ...
    @property
    def comparison_label(self) -> str: ...
    @property
    def baseline_value(self) -> float: ...
    @property
    def comparison_value(self) -> float: ...
    @property
    def steps(self) -> List[BridgeStep]: ...
    def __repr__(self) -> str: ...

class VarianceAnalyzer:
    """Variance analyzer between two evaluated StatementResult."""

    def __init__(
        self,
        baseline: StatementResult,
        comparison: StatementResult,
        baseline_label: str = "baseline",
        comparison_label: str = "comparison",
    ) -> None:
        """Create a new variance analyzer.

        Args:
            baseline: Baseline evaluation results.
            comparison: Comparison evaluation results.
            baseline_label: Label for the baseline scenario.
            comparison_label: Label for the comparison scenario.
        """
        ...

    def compute(
        self,
        metrics: List[str],
        periods: List[PeriodId] | None = None,
    ) -> VarianceReport:
        """Compute variance between baseline and comparison.

        Args:
            metrics: Node identifiers to compare.
            periods: Periods to include (if None, infers from baseline results).

        Returns:
            VarianceReport: Structured variance report.
        """
        ...

    def bridge(
        self,
        target_metric: str,
        drivers: List[str],
        period: PeriodId | None = None,
    ) -> BridgeChart:
        """Compute a simple bridge decomposition for a target metric.

        Args:
            target_metric: Target metric identifier (e.g. "ebitda").
            drivers: Driver node identifiers.
            period: Period to analyze (if None, uses latest period).

        Returns:
            BridgeChart: Bridge chart with driver contributions.
        """
        ...

    def __repr__(self) -> str: ...

# =============================================================================
# Scenario Management
# =============================================================================

class ScenarioDefinition:
    """Definition for a single named scenario."""

    def __init__(
        self,
        parent: str | None = None,
        overrides: Dict[str, float] | None = None,
        model_id: str | None = None,
    ) -> None:
        """Create a new scenario definition.

        Args:
            parent: Optional parent scenario to inherit overrides from.
            overrides: Map of node_id → scalar overrides applied to all periods.
            model_id: Optional identifier of the underlying financial model.
        """
        ...

    @property
    def parent(self) -> str | None: ...
    @property
    def model_id(self) -> str | None: ...
    @property
    def overrides(self) -> Dict[str, float]: ...
    def __repr__(self) -> str: ...

class ScenarioSet:
    """Named scenario registry for financial models."""

    def __init__(self) -> None: ...
    @staticmethod
    def from_mapping(mapping: Dict[str, Dict[str, Any]]) -> ScenarioSet:
        """Create a scenario set from a mapping of name → definition dict."""
        ...

    @staticmethod
    def from_json(path: str) -> ScenarioSet:
        """Load a scenario set from a JSON file."""
        ...

    def add_scenario(self, name: str, definition: ScenarioDefinition) -> None:
        """Add or replace a scenario by name."""
        ...

    def remove_scenario(self, name: str) -> bool:
        """Remove a scenario by name, returning True if it existed."""
        ...

    @property
    def scenario_names(self) -> List[str]:
        """Scenario names in insertion order."""
        ...

    def evaluate_all(self, base_model: FinancialModelSpec) -> "ScenarioResults":
        """Evaluate all scenarios using a base model."""
        ...

    def diff(
        self,
        results: "ScenarioResults",
        baseline: str,
        comparison: str,
        metrics: List[str],
        periods: List[PeriodId],
    ) -> "ScenarioDiff":
        """Compute a variance-style diff between two scenarios."""
        ...

    def trace(self, scenario: str) -> List[str]:
        """Return the lineage for a scenario (from root ancestor to the given name)."""
        ...

    def __repr__(self) -> str: ...

class ScenarioResults:
    """Evaluated results for all scenarios in a ScenarioSet."""

    @property
    def scenario_names(self) -> List[str]: ...
    def __len__(self) -> int: ...
    def get(self, name: str) -> StatementResult:
        """Get the StatementResult for a given scenario."""
        ...

    def to_comparison_df(self, metrics: List[str]) -> Any:
        """Export a wide comparison table as a Polars DataFrame."""
        ...

    def __repr__(self) -> str: ...

class ScenarioDiff:
    """Variance-style diff between two evaluated scenarios."""

    @property
    def baseline(self) -> str: ...
    @property
    def comparison(self) -> str: ...
    @property
    def variance(self) -> VarianceReport: ...
    def __repr__(self) -> str: ...

class TornadoEntry:
    """Entry in a tornado chart."""

    def __init__(self, parameter_id: str, downside_impact: float, upside_impact: float) -> None:
        """Create a tornado entry.

        Args:
            parameter_id: Parameter identifier
            downside_impact: Impact of low value
            upside_impact: Impact of high value
        """
        ...

    @property
    def parameter_id(self) -> str: ...
    @property
    def downside_impact(self) -> float: ...
    @property
    def upside_impact(self) -> float: ...
    @property
    def swing(self) -> float: ...
    def __repr__(self) -> str: ...

def generate_tornado_chart(result: SensitivityResult, metric: str) -> List[TornadoEntry]:
    """Generate tornado chart data from sensitivity analysis results.

    Args:
        result: Results from sensitivity analysis
        metric: Target metric identifier to analyze

    Returns:
        List[TornadoEntry]: List of tornado entries sorted by swing magnitude
    """
    ...

# =============================================================================
# Monte Carlo Configuration
# =============================================================================

class MonteCarloConfig:
    """Monte Carlo simulation configuration.

    Specifies the number of paths, the random seed, and which percentiles
    to compute from the simulated distribution.
    """

    def __init__(self, n_paths: int, seed: int) -> None:
        """Create a new Monte Carlo configuration.

        Default percentiles are ``[0.05, 0.5, 0.95]``.

        Args:
            n_paths: Number of Monte Carlo paths to simulate
            seed: Random seed for reproducibility
        """
        ...

    def with_percentiles(self, percentiles: List[float]) -> MonteCarloConfig:
        """Return a new config with the given percentiles.

        Args:
            percentiles: Percentile values in [0.0, 1.0]

        Returns:
            MonteCarloConfig: New configuration with updated percentiles
        """
        ...

    @property
    def n_paths(self) -> int: ...
    @property
    def seed(self) -> int: ...
    @property
    def percentiles(self) -> List[float]: ...
    def __repr__(self) -> str: ...

# =============================================================================
# Dependency Tracing & Formula Explanation
# =============================================================================

class ExplanationStep:
    """Step in a formula calculation breakdown."""

    def __init__(self, component: str, value: float, operation: str | None = None) -> None:
        """Create an explanation step.

        Args:
            component: Component identifier (e.g., "revenue")
            value: Value of the component
            operation: Operation applied (e.g., "+", "-", "*", "/")
        """
        ...

    @property
    def component(self) -> str: ...
    @property
    def value(self) -> float: ...
    @property
    def operation(self) -> str | None: ...
    def __repr__(self) -> str: ...

class Explanation:
    """Detailed explanation of a node's calculation."""

    @property
    def node_id(self) -> str: ...
    @property
    def period_id(self) -> PeriodId: ...
    @property
    def final_value(self) -> float: ...
    @property
    def node_type(self) -> NodeType: ...
    @property
    def formula_text(self) -> str | None: ...
    @property
    def breakdown(self) -> List[ExplanationStep]: ...
    def to_string_detailed(self) -> str:
        """Convert explanation to detailed string format.

        Returns:
            str: Human-readable explanation of the calculation
        """
        ...

    def to_string_compact(self) -> str:
        """Convert explanation to compact string format.

        Returns:
            str: Compact single-line summary
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class FormulaExplainer:
    """Formula explainer for financial models."""

    def __init__(self, model: FinancialModelSpec, results: StatementResult) -> None:
        """Create a new formula explainer.

        Args:
            model: Financial model specification
            results: Evaluation results
        """
        ...

    def explain(self, node_id: str, period: PeriodId) -> Explanation:
        """Explain how a node's value was calculated for a specific period.

        Args:
            node_id: Node identifier
            period: Period to explain

        Returns:
            Explanation: Detailed explanation of the calculation
        """
        ...

    def __repr__(self) -> str: ...

class DependencyTree:
    """Hierarchical dependency tree structure."""

    @property
    def node_id(self) -> str: ...
    @property
    def formula(self) -> str | None: ...
    @property
    def children(self) -> List["DependencyTree"]: ...
    def depth(self) -> int:
        """Get the maximum depth of the tree.

        Returns:
            int: Maximum depth (0 for a leaf node, 1 for a node with children, etc.)
        """
        ...

    def to_string_ascii(self) -> str:
        """Convert tree to ASCII string representation.

        Returns:
            str: ASCII tree visualization
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class DependencyTracer:
    """Dependency tracer for financial models."""

    def __init__(self, model: FinancialModelSpec, graph: DependencyGraph) -> None:
        """Create a new dependency tracer.

        Args:
            model: Financial model specification
            graph: Pre-built dependency graph
        """
        ...

    def direct_dependencies(self, node_id: str) -> List[str]:
        """Get all direct dependencies for a node.

        Args:
            node_id: Node identifier to inspect

        Returns:
            list[str]: Node IDs that are direct dependencies
        """
        ...

    def all_dependencies(self, node_id: str) -> List[str]:
        """Get all transitive dependencies (recursive).

        Args:
            node_id: Node identifier to inspect

        Returns:
            list[str]: All node IDs in dependency order
        """
        ...

    def dependency_tree(self, node_id: str) -> DependencyTree:
        """Get dependency tree as hierarchical structure.

        Args:
            node_id: Root node for the dependency tree

        Returns:
            DependencyTree: Hierarchical dependency structure
        """
        ...

    def dependents(self, node_id: str) -> List[str]:
        """Get nodes that depend on this node (reverse dependencies).

        Args:
            node_id: Node identifier to inspect

        Returns:
            list[str]: Node IDs that depend on this node
        """
        ...

    def __repr__(self) -> str: ...

def render_tree_ascii(tree: DependencyTree) -> str:
    """Render dependency tree as ASCII art.

    Args:
        tree: Dependency tree to render

    Returns:
        str: ASCII representation
    """
    ...

def render_tree_detailed(tree: DependencyTree, results: StatementResult, period: PeriodId) -> str:
    """Render dependency tree with values from results.

    Args:
        tree: Dependency tree to render
        results: Evaluation results containing node values
        period: Period to display values for

    Returns:
        str: ASCII tree with values
    """
    ...

# =============================================================================
# Reports
# =============================================================================

class Alignment:
    """Alignment options for table columns."""

    LEFT: Alignment
    RIGHT: Alignment
    CENTER: Alignment

    def __repr__(self) -> str: ...

class TableBuilder:
    """Builder for ASCII and Markdown tables."""

    def __init__(self) -> None:
        """Create a new table builder."""
        ...

    def add_header(self, name: str) -> None:
        """Add a column header.

        Args:
            name: Column header text
        """
        ...

    def add_header_with_alignment(self, name: str, alignment: Alignment) -> None:
        """Add a column header with specific alignment.

        Args:
            name: Column header text
            alignment: Column alignment
        """
        ...

    def add_row(self, cells: List[str]) -> None:
        """Add a data row.

        Args:
            cells: List of cell values
        """
        ...

    def build(self) -> str:
        """Build ASCII table.

        Returns:
            str: Formatted ASCII table with box-drawing characters
        """
        ...

    def build_markdown(self) -> str:
        """Build Markdown table.

        Returns:
            str: Formatted Markdown table
        """
        ...

    def __repr__(self) -> str: ...

class PLSummaryReport:
    """P&L summary report."""

    def __init__(self, results: StatementResult, line_items: List[str], periods: List[PeriodId]) -> None:
        """Create a new P&L summary report.

        Args:
            results: Evaluation results
            line_items: Node IDs to include
            periods: Periods to display
        """
        ...

    def to_string(self) -> str:
        """Convert report to string format.

        Returns:
            str: Formatted report
        """
        ...

    def to_markdown(self) -> str:
        """Convert report to Markdown format.

        Returns:
            str: Markdown formatted report
        """
        ...

    def print(self) -> None:
        """Print report to stdout."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class CreditAssessmentReport:
    """Credit assessment report."""

    def __init__(self, results: StatementResult, as_of: PeriodId) -> None:
        """Create a new credit assessment report.

        Args:
            results: Evaluation results
            as_of: Period for assessment
        """
        ...

    def to_string(self) -> str:
        """Convert report to string format.

        Returns:
            str: Formatted report
        """
        ...

    def to_markdown(self) -> str:
        """Convert report to Markdown format.

        Returns:
            str: Markdown formatted report
        """
        ...

    def print(self) -> None:
        """Print report to stdout."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class DebtSummaryReport:
    """Debt summary report."""

    def __init__(self, model: FinancialModelSpec, results: StatementResult, as_of: PeriodId) -> None:
        """Create a new debt summary report.

        Args:
            model: Financial model
            results: Evaluation results
            as_of: Period for report
        """
        ...

    def to_string(self) -> str:
        """Convert report to string format.

        Returns:
            str: Formatted report
        """
        ...

    def to_markdown(self) -> str:
        """Convert report to Markdown format.

        Returns:
            str: Markdown formatted report
        """
        ...

    def print(self) -> None:
        """Print report to stdout."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

def print_debt_summary(model: FinancialModelSpec, results: StatementResult, as_of: PeriodId) -> None:
    """Convenience function to print debt summary.

    Args:
        model: Financial model
        results: Evaluation results
        as_of: Period for report
    """
    ...

# =============================================================================
# Backtesting
# =============================================================================

class ForecastMetrics:
    """Forecast accuracy metrics.

    Contains MAE, MAPE, RMSE, and the number of data points used.
    """

    @property
    def mae(self) -> float:
        """Mean Absolute Error."""
        ...

    @property
    def mape(self) -> float:
        """Mean Absolute Percentage Error."""
        ...

    @property
    def rmse(self) -> float:
        """Root Mean Squared Error."""
        ...

    @property
    def n(self) -> int:
        """Number of data points."""
        ...

    def summary(self) -> str:
        """Format metrics as a human-readable summary.

        Returns:
            str: Summary string
        """
        ...

    def __repr__(self) -> str: ...

def backtest_forecast(actual: list[float], forecast: list[float]) -> ForecastMetrics:
    """Compute forecast error metrics by comparing actual vs forecast values.

    Args:
        actual: Actual observed values
        forecast: Forecasted/predicted values

    Returns:
        ForecastMetrics: Metrics containing MAE, MAPE, and RMSE

    Raises:
        RuntimeError: If arrays have different lengths
    """
    ...

# =============================================================================
# Credit Context
# =============================================================================

class CreditContextMetrics:
    """Per-instrument credit context metrics.

    Contains DSCR, interest coverage, and LTV time series with summary
    statistics.
    """

    @property
    def dscr(self) -> list[tuple[PeriodId, float]]:
        """DSCR by period."""
        ...

    @property
    def interest_coverage(self) -> list[tuple[PeriodId, float]]:
        """Interest coverage by period."""
        ...

    @property
    def ltv(self) -> list[tuple[PeriodId, float]]:
        """LTV by period."""
        ...

    @property
    def dscr_min(self) -> float | None:
        """Minimum DSCR across all periods."""
        ...

    @property
    def interest_coverage_min(self) -> float | None:
        """Minimum interest coverage across all periods."""
        ...

    def __repr__(self) -> str: ...

def compute_credit_context(
    statement: StatementResult,
    cs_cashflows: CapitalStructureCashflows,
    instrument_id: str,
    coverage_node: str,
    periods: list[Period],
    reference_value: float | None = None,
) -> CreditContextMetrics:
    """Compute credit context metrics for a specific instrument.

    Args:
        statement: Evaluated statement results
        cs_cashflows: Capital structure cashflows
        instrument_id: Instrument to compute metrics for
        coverage_node: Statement node for coverage numerator (e.g. "ebitda")
        periods: Periods over which to compute metrics
        reference_value: Optional reference value for LTV (e.g. enterprise value)

    Returns:
        CreditContextMetrics: Credit metrics (DSCR, interest coverage, LTV)
    """
    ...

# =============================================================================
# Corporate DCF Valuation
# =============================================================================

class DcfOptions:
    """Optional configuration for DCF valuation."""

    def __init__(
        self,
        *,
        mid_year_convention: bool = False,
        shares_outstanding: float | None = None,
    ) -> None:
        """Create DCF options.

        Args:
            mid_year_convention: Enable mid-year discounting convention
            shares_outstanding: Basic shares outstanding for per-share calculation
        """
        ...

    @property
    def mid_year_convention(self) -> bool: ...
    @property
    def shares_outstanding(self) -> float | None: ...
    def __repr__(self) -> str: ...

class CorporateValuationResult:
    """Corporate valuation result from DCF analysis."""

    @property
    def equity_value(self) -> Money:
        """Equity value."""
        ...

    @property
    def enterprise_value(self) -> Money:
        """Enterprise value."""
        ...

    @property
    def net_debt(self) -> Money:
        """Net debt."""
        ...

    @property
    def terminal_value_pv(self) -> Money:
        """Terminal value (present value)."""
        ...

    @property
    def equity_value_per_share(self) -> float | None:
        """Per-share equity value, if shares_outstanding was set."""
        ...

    @property
    def diluted_shares(self) -> float | None:
        """Diluted share count, if shares_outstanding was set."""
        ...

    def __repr__(self) -> str: ...

def evaluate_dcf(
    model: FinancialModelSpec,
    wacc: float,
    terminal_value: TerminalValueSpec,
    ufcf_node: str = "ufcf",
    net_debt_override: float | None = None,
) -> CorporateValuationResult:
    """Evaluate a financial model using DCF methodology.

    Args:
        model: Financial model with forecast periods
        wacc: Weighted average cost of capital (decimal, e.g., 0.10)
        terminal_value: Terminal value specification
        ufcf_node: Node ID containing UFCF values (default: "ufcf")
        net_debt_override: Optional fixed net debt value

    Returns:
        CorporateValuationResult: DCF valuation results
    """
    ...

def evaluate_dcf_with_options(
    model: FinancialModelSpec,
    wacc: float,
    terminal_value: TerminalValueSpec,
    ufcf_node: str = "ufcf",
    net_debt_override: float | None = None,
    options: DcfOptions | None = None,
) -> CorporateValuationResult:
    """Evaluate DCF with additional options.

    Args:
        model: Financial model with forecast periods
        wacc: Weighted average cost of capital
        terminal_value: Terminal value specification
        ufcf_node: Node ID containing UFCF values
        net_debt_override: Optional fixed net debt value
        options: DCF options (mid-year convention, shares, etc.)

    Returns:
        CorporateValuationResult: DCF valuation results
    """
    ...

def evaluate_dcf_with_market(
    model: FinancialModelSpec,
    wacc: float,
    terminal_value: TerminalValueSpec,
    ufcf_node: str = "ufcf",
    net_debt_override: float | None = None,
    options: DcfOptions | None = None,
    market: MarketContext | None = None,
) -> CorporateValuationResult:
    """Evaluate DCF with market context.

    Args:
        model: Financial model with forecast periods
        wacc: Weighted average cost of capital
        terminal_value: Terminal value specification
        ufcf_node: Node ID containing UFCF values
        net_debt_override: Optional fixed net debt value
        options: DCF options
        market: Market context for curve-based discounting

    Returns:
        CorporateValuationResult: DCF valuation results
    """
    ...

# =============================================================================
# Covenant Analysis
# =============================================================================

def forecast_covenant(
    covenant: CovenantSpec,
    model: FinancialModelSpec,
    base_case: StatementResult,
    periods: list[PeriodId],
    config: CovenantForecastConfig | None = None,
) -> CovenantForecast:
    """Forecast a single covenant's future compliance using statement results.

    Args:
        covenant: Covenant specification to forecast
        model: Financial model with period definitions
        base_case: Evaluated statement results (time-series of metrics)
        periods: Periods to project over
        config: Forecasting configuration (uses defaults when omitted)

    Returns:
        CovenantForecast: Projected covenant compliance
    """
    ...

def forecast_covenants(
    covenants: list[CovenantSpec],
    model: FinancialModelSpec,
    base_case: StatementResult,
    periods: list[PeriodId],
    config: CovenantForecastConfig | None = None,
) -> list[CovenantForecast]:
    """Forecast multiple covenants with shared statement inputs.

    Args:
        covenants: Covenant specifications to forecast
        model: Financial model with period definitions
        base_case: Evaluated statement results (time-series of metrics)
        periods: Periods to project over
        config: Forecasting configuration (uses defaults when omitted)

    Returns:
        list[CovenantForecast]: Projected covenant compliance for each specification
    """
    ...

def forecast_breaches(
    results: StatementResult,
    covenant_specs: list[CovenantSpec],
    model: FinancialModelSpec | None = None,
    config: CovenantForecastConfig | None = None,
) -> list[FutureBreach]:
    """Forecast covenant breaches based on statement results.

    Args:
        results: Evaluated statement results containing metric time-series
        covenant_specs: Covenant specifications to check for breaches
        model: Financial model for precise period end dates (optional)
        config: Forecasting configuration (uses defaults when omitted)

    Returns:
        list[FutureBreach]: Projected breaches across all covenants and periods
    """
    ...

# =============================================================================
# Corporate Analysis Orchestrator
# =============================================================================

class CreditInstrumentAnalysis:
    """Credit analysis for a single instrument."""

    @property
    def coverage(self) -> CreditContextMetrics:
        """Coverage and leverage metrics computed from the statement context."""
        ...

    def __repr__(self) -> str: ...

class CorporateAnalysis:
    """Unified corporate analysis result combining statement, equity, and credit
    perspectives.
    """

    @property
    def statement(self) -> StatementResult:
        """Full statement evaluation result (all nodes, all periods)."""
        ...

    @property
    def equity(self) -> CorporateValuationResult | None:
        """Equity valuation result (None when no DCF was configured)."""
        ...

    @property
    def credit(self) -> Dict[str, CreditInstrumentAnalysis]:
        """Per-instrument credit analysis keyed by instrument id."""
        ...

    def __repr__(self) -> str: ...

class CorporateAnalysisBuilder:
    """Builder for the corporate analysis pipeline.

    The builder uses move semantics internally. Calling :meth:`analyze`
    consumes the builder; a second call raises ``RuntimeError``.

    Example:
        >>> builder = CorporateAnalysisBuilder(model)
        >>> result = builder.dcf(0.10, tv).analyze()
        >>> result.statement
        StatementResult(...)
    """

    def __init__(self, model: FinancialModelSpec) -> None:
        """Create a new builder for the given financial model.

        Args:
            model: Financial model specification
        """
        ...

    def market(self, ctx: MarketContext) -> CorporateAnalysisBuilder:
        """Set the market context for curve-based discounting.

        Args:
            ctx: Market context

        Returns:
            CorporateAnalysisBuilder: self (for chaining)
        """
        ...

    def as_of(self, date: date) -> CorporateAnalysisBuilder:
        """Set the as-of date for valuation.

        Args:
            date: Valuation date

        Returns:
            CorporateAnalysisBuilder: self (for chaining)
        """
        ...

    def dcf(self, wacc: float, terminal_value: TerminalValueSpec) -> CorporateAnalysisBuilder:
        """Configure DCF equity valuation with default options.

        Args:
            wacc: Weighted average cost of capital
            terminal_value: Terminal value specification

        Returns:
            CorporateAnalysisBuilder: self (for chaining)
        """
        ...

    def dcf_with_options(
        self,
        wacc: float,
        terminal_value: TerminalValueSpec,
        options: DcfOptions,
    ) -> CorporateAnalysisBuilder:
        """Configure DCF equity valuation with custom options.

        Args:
            wacc: Weighted average cost of capital
            terminal_value: Terminal value specification
            options: DCF options

        Returns:
            CorporateAnalysisBuilder: self (for chaining)
        """
        ...

    def dcf_node(self, node: str) -> CorporateAnalysisBuilder:
        """Override the UFCF node name (default: "ufcf").

        Args:
            node: Node ID for unlevered free cashflow

        Returns:
            CorporateAnalysisBuilder: self (for chaining)
        """
        ...

    def net_debt_override(self, net_debt: float) -> CorporateAnalysisBuilder:
        """Override net debt for the equity bridge calculation.

        Args:
            net_debt: Fixed net debt value

        Returns:
            CorporateAnalysisBuilder: self (for chaining)
        """
        ...

    def coverage_node(self, node: str) -> CorporateAnalysisBuilder:
        """Set the coverage node for credit metrics (default: "ebitda").

        Args:
            node: Node ID for coverage numerator

        Returns:
            CorporateAnalysisBuilder: self (for chaining)
        """
        ...

    def analyze(self) -> CorporateAnalysis:
        """Execute the analysis pipeline and return the combined result.

        The builder is consumed; calling this a second time raises
        ``RuntimeError``.

        Returns:
            CorporateAnalysis: Combined analysis result
        """
        ...

    def __repr__(self) -> str: ...

# =============================================================================
# Exports
# =============================================================================

__all__ = [
    # Sensitivity Analysis
    "ParameterSpec",
    "SensitivityMode",
    "SensitivityConfig",
    "SensitivityScenario",
    "SensitivityResult",
    "SensitivityAnalyzer",
    "TornadoEntry",
    "generate_tornado_chart",
    # Dependency Tracing & Formula Explanation
    "ExplanationStep",
    "Explanation",
    "FormulaExplainer",
    "DependencyTree",
    "DependencyTracer",
    "render_tree_ascii",
    "render_tree_detailed",
    # Reports
    "Alignment",
    "TableBuilder",
    "PLSummaryReport",
    "CreditAssessmentReport",
    "DebtSummaryReport",
    "print_debt_summary",
    # Variance
    "VarianceConfig",
    "VarianceRow",
    "VarianceReport",
    "BridgeStep",
    "BridgeChart",
    "VarianceAnalyzer",
    # Scenario Management
    "ScenarioDefinition",
    "ScenarioSet",
    "ScenarioResults",
    "ScenarioDiff",
    # Monte Carlo
    "MonteCarloConfig",
    # Backtesting
    "ForecastMetrics",
    "backtest_forecast",
    # Credit Context
    "CreditContextMetrics",
    "compute_credit_context",
    # Corporate DCF Valuation
    "DcfOptions",
    "CorporateValuationResult",
    "evaluate_dcf",
    "evaluate_dcf_with_options",
    "evaluate_dcf_with_market",
    # Covenant Analysis
    "forecast_covenant",
    "forecast_covenants",
    "forecast_breaches",
    # Corporate Analysis Orchestrator
    "CreditInstrumentAnalysis",
    "CorporateAnalysis",
    "CorporateAnalysisBuilder",
]
