"""Analysis tools for financial statement models.

This module provides tools for:
- Sensitivity analysis - Parameter sweeps and tornado charts
- Dependency tracing - Identify direct and transitive dependencies
- Formula explanation - Break down calculations step-by-step
- Reports - Formatted output for P&L summaries and credit assessment
"""

from typing import Any, Dict, List, Optional
from ..evaluator import Results, DependencyGraph
from ..types import FinancialModelSpec, NodeType
from ...core.dates.periods import PeriodId

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
    def results(self) -> Results: ...
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
    """Variance analyzer between two evaluated Results."""

    def __init__(
        self,
        baseline: Results,
        comparison: Results,
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
        periods: Optional[List[PeriodId]] = None,
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
        period: Optional[PeriodId] = None,
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
        parent: Optional[str] = None,
        overrides: Optional[Dict[str, float]] = None,
        model_id: Optional[str] = None,
    ) -> None:
        """Create a new scenario definition.

        Args:
            parent: Optional parent scenario to inherit overrides from.
            overrides: Map of node_id → scalar overrides applied to all periods.
            model_id: Optional identifier of the underlying financial model.
        """
        ...

    @property
    def parent(self) -> Optional[str]: ...

    @property
    def model_id(self) -> Optional[str]: ...

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

    def get(self, name: str) -> Results:
        """Get the Results for a given scenario."""
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
# Dependency Tracing & Formula Explanation
# =============================================================================

class ExplanationStep:
    """Step in a formula calculation breakdown."""

    def __init__(self, component: str, value: float, operation: Optional[str] = None) -> None:
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
    def operation(self) -> Optional[str]: ...
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
    def formula_text(self) -> Optional[str]: ...
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

    def __init__(self, model: FinancialModelSpec, results: Results) -> None:
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
    def formula(self) -> Optional[str]: ...
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

def render_tree_detailed(tree: DependencyTree, results: Results, period: PeriodId) -> str:
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

    def __init__(self, results: Results, line_items: List[str], periods: List[PeriodId]) -> None:
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

    def __init__(self, results: Results, as_of: PeriodId) -> None:
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

    def __init__(self, model: FinancialModelSpec, results: Results, as_of: PeriodId) -> None:
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

def print_debt_summary(model: FinancialModelSpec, results: Results, as_of: PeriodId) -> None:
    """Convenience function to print debt summary.

    Args:
        model: Financial model
        results: Evaluation results
        as_of: Period for report
    """
    ...

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
]
