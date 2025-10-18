"""Builder API for financial models."""

from typing import Any, List
from ..types.model import FinancialModelSpec
from ..types.forecast import ForecastSpec
from ..types.value import AmountOrScalar
from ...core.dates.periods import Period, PeriodId

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

    def build(self) -> FinancialModelSpec:
        """Build the final model specification.

        Returns:
            FinancialModelSpec: Complete model specification
        """
        ...

    def __repr__(self) -> str: ...
