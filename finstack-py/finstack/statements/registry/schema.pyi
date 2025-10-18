"""Schema types for metric definitions."""

from typing import Optional, List, Dict, Any

class UnitType:
    """Unit type for metric values."""

    # Class attributes
    PERCENTAGE: UnitType
    CURRENCY: UnitType
    RATIO: UnitType
    COUNT: UnitType
    TIME_PERIOD: UnitType

    def __repr__(self) -> str: ...

class MetricDefinition:
    """Individual metric definition."""

    def __init__(
        self,
        id: str,
        name: str,
        formula: str,
        description: Optional[str] = None,
        category: Optional[str] = None,
        unit_type: Optional[UnitType] = None,
        requires: Optional[List[str]] = None,
        tags: Optional[List[str]] = None
    ) -> None:
        """Create a metric definition.

        Args:
            id: Unique identifier within namespace
            name: Human-readable name
            formula: Formula text in statement DSL
            description: Description of what this metric represents
            category: Category for grouping (e.g., "margins", "returns")
            unit_type: Unit type (percentage, currency, ratio, etc.)
            requires: List of required node dependencies
            tags: Tags for filtering/searching

        Returns:
            MetricDefinition: Metric definition
        """
        ...

    @property
    def id(self) -> str: ...

    @property
    def name(self) -> str: ...

    @property
    def formula(self) -> str: ...

    @property
    def description(self) -> Optional[str]: ...

    @property
    def category(self) -> Optional[str]: ...

    @property
    def unit_type(self) -> Optional[UnitType]: ...

    @property
    def requires(self) -> List[str]: ...

    @property
    def tags(self) -> List[str]: ...

    @property
    def meta(self) -> Dict[str, Any]: ...

    def to_json(self) -> str:
        """Convert to JSON string.

        Returns:
            str: JSON representation
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> MetricDefinition:
        """Create from JSON string.

        Args:
            json_str: JSON string

        Returns:
            MetricDefinition: Deserialized metric definition
        """
        ...

    def __repr__(self) -> str: ...

class MetricRegistry:
    """Top-level metric registry schema."""

    def __init__(
        self,
        namespace: str,
        metrics: List[MetricDefinition],
        schema_version: Optional[int] = None
    ) -> None:
        """Create a metric registry.

        Args:
            namespace: Namespace for all metrics (e.g., "fin", "custom")
            metrics: List of metric definitions
            schema_version: Schema version (default: 1)

        Returns:
            MetricRegistry: Registry
        """
        ...

    @property
    def namespace(self) -> str: ...

    @property
    def schema_version(self) -> int: ...

    @property
    def metrics(self) -> List[MetricDefinition]: ...

    @property
    def meta(self) -> Dict[str, Any]: ...

    def to_json(self) -> str:
        """Convert to JSON string.

        Returns:
            str: JSON representation
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> MetricRegistry:
        """Create from JSON string.

        Args:
            json_str: JSON string

        Returns:
            MetricRegistry: Deserialized registry
        """
        ...

    def __repr__(self) -> str: ...
