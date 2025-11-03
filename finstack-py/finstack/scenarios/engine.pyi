"""Scenario engine and execution context."""

from typing import List, Optional, Dict, Any
from datetime import date
from ...core.market_data.context import MarketContext
from ...statements.types.model import FinancialModelSpec
from .spec import ScenarioSpec
from .reports import ApplicationReport

class ExecutionContext:
    """Execution context for scenario application.

    Holds all mutable state that a scenario can touch — market data,
    statement models, instruments, and rate bindings — together with
    the current valuation date.

    Args:
        market: Market data context (curves, surfaces, FX, etc.)
        model: Financial statements model
        as_of: Valuation date for context
        instruments: Optional vector of instruments for price/spread shocks and carry calculations
        rate_bindings: Optional mapping from statement node IDs to curve IDs for automatic rate updates

    Examples:
        >>> from finstack.scenarios import ExecutionContext
        >>> from finstack.core import MarketContext
        >>> from finstack.statements import FinancialModelSpec
        >>> from datetime import date
        >>> market = MarketContext()
        >>> model = FinancialModelSpec("demo", [])
        >>> ctx = ExecutionContext(market, model, date(2025, 1, 1))
    """

    def __init__(
        self,
        market: MarketContext,
        model: FinancialModelSpec,
        as_of: date,
        instruments: Optional[List[Any]] = None,
        rate_bindings: Optional[Dict[str, str]] = None,
    ) -> None:
        """Create a new execution context.

        Args:
            market: Market data context
            model: Financial model
            as_of: Valuation date
            instruments: Optional instruments
            rate_bindings: Optional rate bindings
        """
        ...

    @property
    def market(self) -> MarketContext:
        """Get the market context.

        Returns:
            MarketContext: Market data context
        """
        ...

    @property
    def model(self) -> FinancialModelSpec:
        """Get the financial model.

        Returns:
            FinancialModelSpec: Financial model
        """
        ...

    @property
    def as_of(self) -> date:
        """Get the valuation date.

        Returns:
            date: Valuation date
        """
        ...

    @as_of.setter
    def as_of(self, value: date) -> None:
        """Set the valuation date.

        Args:
            value: New valuation date
        """
        ...

    @property
    def instruments(self) -> Optional[List[Any]]:
        """Get the instruments list.

        Returns:
            list | None: Instruments if set
        """
        ...

    @instruments.setter
    def instruments(self, value: Optional[List[Any]]) -> None:
        """Set the instruments list.

        Args:
            value: New instruments list
        """
        ...

    @property
    def rate_bindings(self) -> Optional[Dict[str, str]]:
        """Get the rate bindings.

        Returns:
            dict[str, str] | None: Rate bindings if set
        """
        ...

    @rate_bindings.setter
    def rate_bindings(self, value: Optional[Dict[str, str]]) -> None:
        """Set the rate bindings.

        Args:
            value: New rate bindings
        """
        ...

    def __repr__(self) -> str: ...

class ScenarioEngine:
    """Orchestrates the reproducible application of a ScenarioSpec with stable ordering."""

    def __init__(self) -> None:
        """Create a new scenario engine with default settings.

        Returns:
            ScenarioEngine: New engine instance
        """
        ...

    def compose(self, scenarios: List[ScenarioSpec]) -> ScenarioSpec:
        """Compose multiple scenarios into a single deterministic spec.

        Operations are sorted by (priority, declaration_index); conflicts use last-wins.

        Args:
            scenarios: Collection of scenario specifications to combine

        Returns:
            ScenarioSpec: Combined scenario containing all operations with deterministic ordering
        """
        ...

    def apply(self, scenario: ScenarioSpec, context: ExecutionContext) -> ApplicationReport:
        """Apply a scenario specification to the execution context.

        Operations are applied in this order:
        1. Market data (FX, equities, vol surfaces, curves, base correlation)
        2. Rate bindings update (if configured)
        3. Statement forecast adjustments
        4. Statement re-evaluation

        Args:
            scenario: Scenario specification to apply
            context: Mutable execution context that supplies market data, statements,
                    instruments, and rate bindings

        Returns:
            ApplicationReport: Summary of how many operations were applied and any warnings

        Raises:
            ValueError: If operation cannot be completed (e.g., missing market data,
                       unsupported operation, or invalid tenor strings)
        """
        ...

    def __repr__(self) -> str: ...
