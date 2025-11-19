"""Mixed node builder for statements."""

from typing import Any, List, Tuple
from ..types.forecast import ForecastSpec
from ..types.value import AmountOrScalar
from ...core.dates.periods import PeriodId

class MixedNodeBuilder:
    """Builder for mixed nodes with values, forecasts, and formulas.

    Mixed nodes combine explicit values, forecasts, and fallback formulas
    using precedence rules: Value > Forecast > Formula.
    """

    def values(self, values: Any) -> None:
        """Set explicit values for the mixed node.

        Args:
            values: Period values to seed actual periods (list of tuples or dict)
        """
        ...

    def forecast(self, forecast_spec: ForecastSpec) -> None:
        """Set the forecast specification.

        Args:
            forecast_spec: Forecast configuration
        """
        ...

    def formula(self, formula: str) -> None:
        """Set the fallback formula.

        Args:
            formula: DSL expression evaluated when explicit values or forecasts are absent
        """
        ...

    def name(self, name: str) -> None:
        """Set the human-readable name.

        Args:
            name: Display label used in reports or exports
        """
        ...

    def finish(self) -> "ModelBuilder":
        """Finish building the mixed node and return to the parent builder.

        Returns:
            ModelBuilder: Parent model builder instance
        """
        ...

    def __repr__(self) -> str: ...















