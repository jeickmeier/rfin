"""Node specification bindings."""

from __future__ import annotations
from typing import Dict, Any, List
from finstack.core.dates.periods import PeriodId
from finstack.core.currency import Currency
from .forecast import ForecastSpec
from .value import AmountOrScalar

class NodeValueType:
    """Node value type classification.

    Determines whether a node represents monetary values (with a specific
    currency) or unitless scalar values.
    """

    SCALAR: NodeValueType

    @staticmethod
    def monetary(currency: Currency) -> NodeValueType:
        """Create a monetary value type with the given currency."""
        ...

    @property
    def currency(self) -> Currency | None:
        """Get the currency if this is a monetary type, None if scalar."""
        ...

    def __repr__(self) -> str: ...

class NodeType:
    """Node computation type.

    Determines how a node's value is computed.
    """

    # Class attributes
    VALUE: NodeType
    CALCULATED: NodeType
    MIXED: NodeType

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class NodeSpec:
    """Node specification.

    Specifies a single node (metric/line item) in a financial model.
    """

    def __init__(self, node_id: str, node_type: NodeType) -> None:
        """Create a new node specification.

        Args:
            node_id: Unique identifier for this node
            node_type: Node computation type

        Returns:
            NodeSpec: Node specification
        """
        ...

    def with_name(self, name: str) -> NodeSpec:
        """Set the human-readable name.

        Args:
            name: Human-readable name

        Returns:
            NodeSpec: Updated node spec
        """
        ...

    def with_values(self, values: Any) -> NodeSpec:
        """Add explicit values per period.

        Args:
            values: Period values (dict or list of tuples)

        Returns:
            NodeSpec: Updated node spec
        """
        ...

    def with_formula(self, formula: str) -> NodeSpec:
        """Set the formula text.

        Args:
            formula: Formula text in statement DSL

        Returns:
            NodeSpec: Updated node spec
        """
        ...

    def with_forecast(self, forecast_spec: ForecastSpec) -> NodeSpec:
        """Set the forecast specification.

        Args:
            forecast_spec: Forecast specification

        Returns:
            NodeSpec: Updated node spec
        """
        ...

    def with_tags(self, tags: List[str]) -> NodeSpec:
        """Add tags for grouping/filtering.

        Args:
            tags: Tags

        Returns:
            NodeSpec: Updated node spec
        """
        ...

    @property
    def node_id(self) -> str:
        """Get the node ID.

        Returns:
            str: Node ID
        """
        ...

    @property
    def name(self) -> str | None:
        """Get the human-readable name.

        Returns:
            str | None: Name if set
        """
        ...

    @property
    def node_type(self) -> NodeType:
        """Get the node type.

        Returns:
            NodeType: Node computation type
        """
        ...

    @property
    def values(self) -> Dict[PeriodId, AmountOrScalar] | None:
        """Get explicit period values.

        Returns:
            dict[PeriodId, AmountOrScalar] | None: Period values if set
        """
        ...

    @property
    def forecast(self) -> ForecastSpec | None:
        """Get the forecast specification.

        Returns:
            ForecastSpec | None: Forecast spec if set
        """
        ...

    @property
    def formula_text(self) -> str | None:
        """Get the formula text.

        Returns:
            str | None: Formula text if set
        """
        ...

    @property
    def where_text(self) -> str | None:
        """Get the where clause.

        Returns:
            str | None: Where clause if set
        """
        ...

    @property
    def tags(self) -> List[str]:
        """Get tags.

        Returns:
            list[str]: Tags
        """
        ...

    @property
    def meta(self) -> Dict[str, Any]:
        """Get metadata.

        Returns:
            dict: Metadata dictionary
        """
        ...

    def to_json(self) -> str:
        """Convert to JSON string.

        Returns:
            str: JSON representation
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> NodeSpec:
        """Create from JSON string.

        Args:
            json_str: JSON string

        Returns:
            NodeSpec: Deserialized node spec
        """
        ...

    def __repr__(self) -> str: ...
