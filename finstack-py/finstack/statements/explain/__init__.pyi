"""Node explanation and dependency tracing."""

from typing import Any, List, Optional
from ..evaluator import Results, DependencyGraph
from ..types import FinancialModelSpec, NodeType
from ...core.dates.periods import PeriodId

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

__all__ = [
    "ExplanationStep",
    "Explanation",
    "FormulaExplainer",
    "DependencyTree",
    "DependencyTracer",
    "render_tree_ascii",
    "render_tree_detailed",
]














