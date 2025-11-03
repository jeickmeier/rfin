"""Builder API for financial models."""

from typing import Any, List, Dict
from ..types.model import FinancialModelSpec
from ..types.forecast import ForecastSpec
from ..types.value import AmountOrScalar
from ...core.dates.periods import Period, PeriodId
from .mixed_builder import MixedNodeBuilder

class ModelBuilder:
    """Builder for financial models.

    Provides a fluent API for building financial statement models with
    type-safe construction.

    Examples:
        >>> builder = ModelBuilder.new("Acme Corp")
        >>> builder = builder.periods("2025Q1..Q4", "2025Q2")
        >>> builder = builder.value("revenue", [(PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100))])
        >>> builder = builder.compute("gross_profit", "revenue * 0.4")
        >>> model = builder.build()
    """

    @classmethod
    def new(cls, id: str) -> ModelBuilder:
        """Create a new model builder.

        You must call `periods()` before adding nodes.

        Args:
            id: Unique model identifier

        Returns:
            ModelBuilder: Model builder instance
        """
        ...

    def periods(self, range: str, actuals_until: Optional[str] = None) -> None:
        """Define periods using a range expression.

        Args:
            range: Period range (e.g., "2025Q1..Q4", "2025Q1..2026Q2")
            actuals_until: Optional cutoff for actuals (e.g., "2025Q2")

        Returns:
            ModelBuilder: Builder instance ready for node definitions
        """
        ...

    def periods_explicit(self, periods: List[Period]) -> None:
        """Define periods explicitly.

        Args:
            periods: Explicit list of periods

        Returns:
            ModelBuilder: Builder instance ready for node definitions
        """
        ...

    def value(self, node_id: str, values: Any) -> None:
        """Add a value node with explicit period values.

        Value nodes contain only explicit data (actuals or assumptions).

        Args:
            node_id: Node identifier
            values: Period values (list of tuples or dict)

        Returns:
            ModelBuilder: Builder instance for chaining
        """
        ...

    def value_money(self, node_id: str, values: Any) -> None:
        """Add a monetary value node.

        This is a convenience method for creating value nodes that represent
        monetary amounts (Money type).

        Args:
            node_id: Node identifier
            values: Period values as Money objects (list of tuples or dict)

        Returns:
            ModelBuilder: Builder instance for chaining
        """
        ...

    def value_scalar(self, node_id: str, values: Any) -> None:
        """Add a scalar value node.

        This is a convenience method for creating value nodes that represent
        non-monetary scalars (ratios, percentages, counts, etc.).

        Args:
            node_id: Node identifier
            values: Period values as floats (list of tuples or dict)

        Returns:
            ModelBuilder: Builder instance for chaining
        """
        ...

    def compute(self, node_id: str, formula: str) -> None:
        """Add a calculated node with a formula.

        Calculated nodes derive their values from formulas only.

        Args:
            node_id: Node identifier
            formula: Formula text in statement DSL

        Returns:
            ModelBuilder: Builder instance for chaining
        """
        ...

    def mixed(self, node_id: str) -> MixedNodeBuilder:
        """Create a mixed node with values, forecasts, and formulas.

        Returns a MixedNodeBuilder for chaining method calls.

        Args:
            node_id: Node identifier

        Returns:
            MixedNodeBuilder: Mixed node builder instance
        """
        ...

    def forecast(self, node_id: str, forecast_spec: ForecastSpec) -> None:
        """Add a forecast specification to an existing node.

        This allows forecasting values into future periods using various methods.

        Args:
            node_id: Node identifier
            forecast_spec: Forecast specification

        Returns:
            ModelBuilder: Builder instance for chaining
        """
        ...

    def with_meta(self, key: str, value: Any) -> None:
        """Add metadata to the model.

        Args:
            key: Metadata key
            value: Metadata value (must be JSON-serializable)

        Returns:
            ModelBuilder: Builder instance for chaining
        """
        ...

    def add_bond(
        self,
        id: str,
        notional: Any,  # Money
        coupon_rate: float,
        issue_date: Any,  # date
        maturity_date: Any,  # date
        discount_curve_id: str,
    ) -> None:
        """Add a bond instrument to the capital structure.

        Args:
            id: Unique instrument identifier
            notional: Principal amount (Money)
            coupon_rate: Annual coupon rate (e.g., 0.05 for 5%)
            issue_date: Bond issue date
            maturity_date: Bond maturity date
            discount_curve_id: Discount curve ID for pricing
        """
        ...

    def add_swap(
        self,
        id: str,
        notional: Any,  # Money
        fixed_rate: float,
        start_date: Any,  # date
        maturity_date: Any,  # date
        discount_curve_id: str,
        forward_curve_id: str,
    ) -> None:
        """Add an interest rate swap to the capital structure.

        Args:
            id: Unique instrument identifier
            notional: Notional amount (Money)
            fixed_rate: Fixed rate (e.g., 0.04 for 4%)
            start_date: Swap start date
            maturity_date: Swap maturity date
            discount_curve_id: Discount curve ID
            forward_curve_id: Forward curve ID for floating leg
        """
        ...

    def add_custom_debt(self, id: str, spec: Dict[str, Any]) -> None:
        """Add a generic debt instrument via JSON specification.

        Args:
            id: Unique instrument identifier
            spec: JSON specification for the debt instrument
        """
        ...

    def with_builtin_metrics(self) -> None:
        """Load built-in metrics (fin.* namespace) and add them to the model.

        This adds all standard financial metrics from the built-in registry.
        """
        ...

    def with_metrics(self, path: str) -> None:
        """Load metrics from a JSON file and add them to the model.

        Args:
            path: Path to a metrics JSON definition file
        """
        ...

    def add_metric(self, qualified_id: str) -> None:
        """Add a specific metric from the built-in registry.

        Args:
            qualified_id: Fully qualified metric identifier (e.g., "fin.gross_margin")
        """
        ...

    def add_metric_from_registry(self, qualified_id: str, registry: Any) -> None:
        """Add a specific metric from a registry.

        Args:
            qualified_id: Fully qualified metric identifier to add
            registry: Registry loaded by the caller (allows reuse across builders)
        """
        ...

    def build(self) -> FinancialModelSpec:
        """Build the final model specification.

        Returns:
            FinancialModelSpec: Complete model specification
        """
        ...

    def __repr__(self) -> str: ...
