"""Registry system for dynamic metrics."""

from __future__ import annotations
from typing import List
from .schema import MetricDefinition, MetricRegistry

class Registry:
    """Dynamic metric registry.

    Allows loading reusable metric definitions from JSON files,
    enabling analysts to define standard financial metrics without recompiling.
    """

    @classmethod
    def new(cls) -> Registry:
        """Create a new registry.

        Returns:
            Registry: Registry instance
        """
        ...

    def load_builtins(self) -> None:
        """Load built-in metrics (fin.* namespace).

        Returns:
            None
        """
        ...

    def load_from_json(self, path: str) -> None:
        """Load metrics from a JSON file.

        Args:
            path: Path to JSON registry file

        Returns:
            None
        """
        ...

    def load_from_json_str(self, json_str: str) -> MetricRegistry:
        """Load metrics from a JSON string.

        Args:
            json_str: JSON string containing metric registry

        Returns:
            MetricRegistry: Loaded registry
        """
        ...

    def get(self, metric_id: str) -> MetricDefinition:
        """Get a metric definition by ID.

        Args:
            metric_id: Metric identifier (e.g., "fin.gross_margin")

        Returns:
            MetricDefinition: Metric definition
        """
        ...

    def list_metrics(self, namespace: str | None = None) -> List[str]:
        """List available metrics.

        Args:
            namespace: Filter by namespace (e.g., "fin")

        Returns:
            list[str]: List of metric IDs
        """
        ...

    def has_metric(self, metric_id: str) -> bool:
        """Check if a metric exists.

        Args:
            metric_id: Metric identifier

        Returns:
            bool: True if metric exists
        """
        ...

    def __repr__(self) -> str: ...
