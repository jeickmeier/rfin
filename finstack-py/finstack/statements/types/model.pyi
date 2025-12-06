"""Financial model specification bindings."""

from typing import Optional, List, Dict, Any
from finstack.core.dates.periods import Period
from .node import NodeSpec
from .waterfall import WaterfallSpec

class CapitalStructureSpec:
    """Capital structure specification.

    Defines debt and equity instruments in a model.
    """

    def __init__(
        self,
        debt_instruments: Optional[List[DebtInstrumentSpec]] = None,
        equity_instruments: Optional[List[Any]] = None,
        waterfall: Optional[WaterfallSpec] = None,
    ) -> None:
        """Create a capital structure specification.

        Args:
            debt_instruments: Debt instruments
            equity_instruments: Equity instruments (future expansion)
            waterfall: Waterfall configuration for dynamic cash flow allocation

        Returns:
            CapitalStructureSpec: Capital structure spec
        """
        ...

    @property
    def debt_instruments(self) -> List[DebtInstrumentSpec]:
        """Get debt instruments.

        Returns:
            list[DebtInstrumentSpec]: Debt instruments
        """
        ...

    @property
    def waterfall(self) -> Optional[WaterfallSpec]:
        """Get waterfall spec.

        Returns:
            WaterfallSpec | None: Waterfall configuration if set
        """
        ...

    def to_json(self) -> str:
        """Convert to JSON string.

        Returns:
            str: JSON representation
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> CapitalStructureSpec:
        """Create from JSON string.

        Args:
            json_str: JSON string

        Returns:
            CapitalStructureSpec: Deserialized spec
        """
        ...

    def __repr__(self) -> str: ...

class DebtInstrumentSpec:
    """Debt instrument specification.

    Represents a debt instrument in a capital structure.
    """

    @staticmethod
    def bond(id: str, spec: Dict[str, Any]) -> DebtInstrumentSpec:
        """Create a bond instrument.

        Args:
            id: Instrument identifier
            spec: Instrument specification

        Returns:
            DebtInstrumentSpec: Bond instrument spec
        """
        ...

    @staticmethod
    def swap(id: str, spec: Dict[str, Any]) -> DebtInstrumentSpec:
        """Create a swap instrument.

        Args:
            id: Instrument identifier
            spec: Instrument specification

        Returns:
            DebtInstrumentSpec: Swap instrument spec
        """
        ...

    @staticmethod
    def generic(id: str, spec: Dict[str, Any]) -> DebtInstrumentSpec:
        """Create a generic debt instrument.

        Args:
            id: Instrument identifier
            spec: Instrument specification

        Returns:
            DebtInstrumentSpec: Generic instrument spec
        """
        ...

    def to_json(self) -> str:
        """Convert to JSON string.

        Returns:
            str: JSON representation
        """
        ...

    def __repr__(self) -> str: ...

class FinancialModelSpec:
    """Financial model specification for statement modeling.

    FinancialModelSpec is the top-level container for a complete financial
    statement model. It contains nodes (value, forecast, formula), periods,
    optional capital structure, and metadata. Models are evaluated period-by-period
    with deterministic precedence rules (Value > Forecast > Formula).

    Models can be built using ModelBuilder or constructed programmatically.
    They support serialization to/from JSON for persistence and sharing.

    Examples
    --------
    Build a model via :class:`ModelBuilder`:

        >>> from finstack.core.dates.periods import PeriodId
        >>> from finstack.statements.builder import ModelBuilder
        >>> from finstack.statements.types import AmountOrScalar, FinancialModelSpec
        >>> builder = ModelBuilder.new("DocCo")
        >>> builder.periods("2025Q1..Q2", None)
        >>> builder.value(
        ...     "revenue",
        ...     [
        ...         (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100.0)),
        ...         (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(110.0)),
        ...     ],
        ... )
        >>> builder.compute("gross_profit", "revenue * 0.4")
        >>> model = builder.build()
        >>> restored = FinancialModelSpec.from_json(model.to_json())
        >>> print(restored.id, len(model.periods), restored.has_node("gross_profit"))
        DocCo 2 True

    Notes
    -----
    - Models are immutable after construction
    - Nodes are evaluated in dependency order
    - Capital structure enables cs.* references in formulas
    - Models can be serialized to JSON for persistence

    See Also
    --------
    :class:`ModelBuilder`: Fluent builder for models
    :class:`Evaluator`: Model evaluation engine
    :class:`NodeSpec`: Individual node specifications
    """

    def __init__(self, id: str, periods: List[Period]) -> None:
        """Create a new financial model specification.

        Args:
            id: Unique model identifier
            periods: Ordered list of periods

        Returns:
            FinancialModelSpec: Model specification
        """
        ...

    def add_node(self, node: NodeSpec) -> None:
        """Add a node to the model.

        Args:
            node: Node specification to add
        """
        ...

    def get_node(self, node_id: str) -> Optional[NodeSpec]:
        """Get a node by ID.

        Args:
            node_id: Node identifier

        Returns:
            NodeSpec | None: Node spec if found
        """
        ...

    def has_node(self, node_id: str) -> bool:
        """Check if a node exists.

        Args:
            node_id: Node identifier

        Returns:
            bool: True if node exists
        """
        ...

    @property
    def id(self) -> str:
        """Get model ID.

        Returns:
            str: Model ID
        """
        ...

    @property
    def periods(self) -> List[Period]:
        """Get periods.

        Returns:
            list[Period]: Ordered periods
        """
        ...

    @property
    def nodes(self) -> Dict[str, NodeSpec]:
        """Get all nodes.

        Returns:
            dict[str, NodeSpec]: Map of node_id to NodeSpec
        """
        ...

    @property
    def capital_structure(self) -> Optional[CapitalStructureSpec]:
        """Get capital structure.

        Returns:
            CapitalStructureSpec | None: Capital structure if set
        """
        ...

    @property
    def meta(self) -> Dict[str, Any]:
        """Get metadata.

        Returns:
            dict: Metadata dictionary
        """
        ...

    @property
    def schema_version(self) -> int:
        """Get schema version.

        Returns:
            int: Schema version
        """
        ...

    def to_json(self) -> str:
        """Convert to JSON string.

        Returns:
            str: JSON representation
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> FinancialModelSpec:
        """Create from JSON string.

        Args:
            json_str: JSON string

        Returns:
            FinancialModelSpec: Deserialized model spec
        """
        ...

    def __repr__(self) -> str: ...
