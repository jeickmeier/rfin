"""Scenario engine and execution context."""

from __future__ import annotations
from typing import List, Dict, Any
from datetime import date
from finstack.core.market_data.context import MarketContext
from finstack.core.dates.calendar import Calendar
from finstack.statements.types.model import FinancialModelSpec
from .spec import ScenarioSpec, RateBindingSpec
from .reports import ApplicationReport

class ExecutionContext:
    """Execution context for scenario application.

    Holds all mutable state that a scenario can touch — market data,
    statement models, instruments, and rate bindings — together with
    the current valuation date.

    Notes
    -----
    - Instruments should be constructed from ``finstack.valuations.instruments``;
      they are converted to Rust trait objects and mutated in-place.
    - ``calendar`` enables business-day aware tenor alignment for curve shocks.
    - ``as_of`` may advance when ``time_roll_forward`` operations are applied.

    Args:
        market: Market data context (curves, surfaces, FX, etc.)
        model: Financial statements model
        as_of: Valuation date for context
        instruments: Optional vector of instruments for price/spread shocks and carry calculations
        rate_bindings: Optional rate bindings for statement rate updates. Accepts ``dict[str, RateBindingSpec]``, ``list[RateBindingSpec]`` or legacy ``dict[str, str]`` (converted to 1Y continuous bindings).

    Examples:
        >>> from datetime import date
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.term_structures import DiscountCurve
        >>> from finstack.statements.types import FinancialModelSpec
        >>> from finstack.scenarios import ExecutionContext
        >>> market = MarketContext()
        >>> market.insert_discount(DiscountCurve("USD", date(2024, 1, 1), [(0.0, 1.0), (1.0, 0.99)]))
        >>> model = FinancialModelSpec("demo_model", [])
        >>> ctx = ExecutionContext(market, model, date(2025, 1, 1))
        >>> ctx.as_of
        datetime.date(2025, 1, 1)
    """

    def __init__(
        self,
        market: MarketContext,
        model: FinancialModelSpec,
        as_of: date,
        instruments: List[Any] | None = None,
        rate_bindings: Dict[str, RateBindingSpec] | List[RateBindingSpec] | Dict[str, str] | None = None,
        calendar: Calendar | None = None,
    ) -> None:
        """Create a new execution context.

        Args:
            market: Market data context
            model: Financial model
            as_of: Valuation date
            instruments: Optional instruments (mutated in-place during shocks)
            rate_bindings: Optional rate bindings for statement rate updates
            calendar: Optional holiday calendar for tenor alignment and date math
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
    def instruments(self) -> List[Any] | None:
        """Get the instruments list.

        Returns:
            list | None: Instruments if set
        """
        ...

    @instruments.setter
    def instruments(self, value: List[Any] | None) -> None:
        """Set the instruments list.

        Args:
            value: New instruments list
        """
        ...

    @property
    def rate_bindings(self) -> Dict[str, RateBindingSpec] | None:
        """Get the rate bindings.

        Returns:
            dict[str, RateBindingSpec] | None: Rate bindings if set (legacy ``dict[str, str]`` inputs are upgraded to 1Y continuous bindings)
        """
        ...

    @rate_bindings.setter
    def rate_bindings(
        self,
        value: Dict[str, RateBindingSpec] | List[RateBindingSpec] | Dict[str, str] | None,
    ) -> None:
        """Set the rate bindings.

        Args:
            value: New rate bindings
        """
        ...

    @property
    def calendar(self) -> Calendar | None:
        """Get the holiday calendar used for business-day adjustments."""
        ...

    @calendar.setter
    def calendar(self, value: Calendar | None) -> None:
        """Set the holiday calendar for tenor alignment."""
        ...

    def __repr__(self) -> str: ...

class ScenarioEngine:
    """Orchestrates reproducible scenario application with stable ordering.

    ScenarioEngine applies scenario specifications to execution contexts,
    modifying market data, statement models, and instrument prices in a
    deterministic order. Scenarios can be composed from multiple scenario
    specs with conflict resolution.

    Scenarios are used for stress testing, what-if analysis, and sensitivity
    analysis across market data, financial statements, and instrument portfolios.

    Examples
    --------
    Create and apply a simple scenario:

        >>> from datetime import date
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.term_structures import DiscountCurve
        >>> from finstack.statements.types import FinancialModelSpec
        >>> from finstack.scenarios import ScenarioEngine, ScenarioSpec, OperationSpec, CurveKind, ExecutionContext
        >>> # Minimal market context with one curve
        >>> market_ctx = MarketContext()
        >>> market_ctx.insert_discount(DiscountCurve("USD-SOFR", date(2024, 1, 1), [(0.0, 1.0), (1.0, 0.99)]))
        >>> model = FinancialModelSpec("demo", [])
        >>> ctx = ExecutionContext(market_ctx, model, date(2025, 1, 1))
        >>> ops = [OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD-SOFR", 50.0)]
        >>> scenario = ScenarioSpec("rate_shock", ops, name="+50bp Rate Shock")
        >>> engine = ScenarioEngine()
        >>> report = engine.apply(scenario, ctx)
        >>> report.operations_applied
        1

    Compose multiple scenarios:

        >>> from finstack.scenarios import ScenarioSpec, ScenarioEngine, OperationSpec, CurveKind
        >>> from finstack.core.currency import Currency
        >>> base_ops = [OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD-SOFR", 25.0)]
        >>> overlay_ops = [OperationSpec.market_fx_pct(Currency("EUR"), Currency("USD"), -0.02)]
        >>> rate_scenario = ScenarioSpec("rates", base_ops, priority=1)
        >>> fx_scenario = ScenarioSpec("fx", overlay_ops, priority=2)
        >>> engine = ScenarioEngine()
        >>> combined = engine.compose([rate_scenario, fx_scenario])

    Notes
    -----
    - Scenarios are applied in deterministic order
    - Operations are grouped by type (market data, statements, instruments)
    - Composition uses priority and last-wins conflict resolution
    - Execution context is mutated in-place
    - Application report summarizes operations applied

    See Also
    --------
    :class:`ScenarioSpec`: Scenario specification
    :class:`OperationSpec`: Individual operations
    :class:`ExecutionContext`: Execution context
    :class:`ApplicationReport`: Application results
    """

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

        Notes:
            - Mutates ``context`` in-place (market, model, instruments, ``as_of``).
            - Honors ``calendar`` on the context for tenor parsing and date math.
        """
        ...

    def __repr__(self) -> str: ...
