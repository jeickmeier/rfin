"""Builder API for financial models."""

from __future__ import annotations
from typing import Any, List, Dict
from ..types.model import FinancialModelSpec
from ..types.forecast import ForecastSpec
from ..types.value import AmountOrScalar
from ..types.waterfall import WaterfallSpec
from ..templates import (
    LeaseSpec,
    LeaseSpecV2,
    ManagementFeeSpec,
    PropertyTemplateNodes,
    RentRollOutputNodes,
)
from ...core.dates.periods import Period, PeriodId
from .mixed_builder import MixedNodeBuilder

class ModelBuilder:
    """Builder for financial statement models with fluent API.

    ModelBuilder provides a type-safe, fluent interface for constructing
    financial statement models. Models consist of nodes (value, forecast,
    formula) organized into periods, with optional capital structure and
    metric definitions.

    The builder enforces precedence rules: Value > Forecast > Formula.
    Nodes can reference other nodes in formulas, creating a directed graph
    that is evaluated period-by-period.

    Examples
    --------
    Build a simple income statement model:

        >>> from finstack.core.dates.periods import PeriodId
        >>> from finstack.statements.builder import ModelBuilder
        >>> from finstack.statements.types import AmountOrScalar
        >>> builder = ModelBuilder.new("DocCo")
        >>> builder.periods("2025Q1..Q2", None)
        >>> builder.value(
        ...     "revenue",
        ...     [
        ...         (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100_000.0)),
        ...         (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(110_000.0)),
        ...     ],
        ... )
        >>> builder.compute("gross_profit", "revenue * 0.4")
        >>> model = builder.build()
        >>> print(sorted(model.nodes.keys()))
        ['gross_profit', 'revenue']

    Add forecasting:

        >>> from finstack.core.dates.periods import PeriodId
        >>> from finstack.statements.builder import ModelBuilder
        >>> from finstack.statements.types import AmountOrScalar, ForecastSpec
        >>> builder = ModelBuilder.new("ForecastCo")
        >>> builder.periods("2025Q1..Q2", "2025Q1")
        >>> builder.value("revenue", [(PeriodId.quarter(2025, 1), AmountOrScalar.scalar(100.0))])
        >>> builder.forecast("revenue", ForecastSpec.growth(0.05))
        >>> has_forecast = builder.build().get_node("revenue").forecast is not None
        >>> print(has_forecast)
        True

    Add capital structure:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.dates.periods import PeriodId
        >>> from finstack.core.money import Money
        >>> from finstack.statements.builder import ModelBuilder
        >>> from finstack.statements.types import AmountOrScalar
        >>> builder = ModelBuilder.new("CapitalCo")
        >>> builder.periods("2025Q1..Q2", None)
        >>> builder.value("operating_income", [(PeriodId.quarter(2025, 1), AmountOrScalar.scalar(50.0))])
        >>> builder.add_bond(
        ...     "BOND_001",
        ...     Money(1_000_000, Currency("USD")),
        ...     0.05,
        ...     date(2024, 1, 1),
        ...     date(2029, 1, 1),
        ...     "USD-SOFR",
        ... )
        >>> print(builder.build().capital_structure is not None)
        True

    Notes
    -----
    - Periods must be defined before adding nodes
    - Node precedence: Value > Forecast > Formula
    - Formulas can reference other nodes by ID
    - Capital structure enables cs.* references in formulas
    - Metrics can be added from built-in registry or custom definitions

    See Also
    --------
    :class:`FinancialModelSpec`: Final model specification
    :class:`Evaluator`: Model evaluation engine
    :class:`StatementResult`: Evaluation results
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

    def periods(self, range: str, actuals_until: str | None = None) -> None:
        """Define periods using a range expression.

        Args:
            range: Period range (e.g., "2025Q1..Q4", "2025Q1..2026Q2")
            actuals_until: Optional cutoff for actuals (e.g., "2025Q2")

        Returns:
            None: Mutates the builder in place.
        """
        ...

    def periods_explicit(self, periods: List[Period]) -> None:
        """Define periods explicitly.

        Args:
            periods: Explicit list of periods

        Returns:
            None: Mutates the builder in place.
        """
        ...

    def value(self, node_id: str, values: Any) -> None:
        """Add a value node with explicit period values.

        Value nodes contain only explicit data (actuals or assumptions).

        Args:
            node_id: Node identifier
            values: Period values (list of tuples or dict)

        Returns:
            None: Mutates the builder in place.
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
            None: Mutates the builder in place.
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
            None: Mutates the builder in place.
        """
        ...

    def compute(self, node_id: str, formula: str) -> None:
        """Add a calculated node with a formula.

        Calculated nodes derive their values from formulas only.

        Args:
            node_id: Node identifier
            formula: Formula text in statement DSL

        Returns:
            None: Mutates the builder in place.
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
            None: Mutates the builder in place.
        """
        ...

    def with_meta(self, key: str, value: Any) -> None:
        """Add metadata to the model.

        Args:
            key: Metadata key
            value: Metadata value (must be JSON-serializable)

        Returns:
            None: Mutates the builder in place.
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

    def add_bond_with_convention(
        self,
        id: str,
        notional: Any,  # Money
        coupon_rate: float,
        issue_date: Any,  # date
        maturity_date: Any,  # date
        convention: str,
        discount_curve_id: str,
    ) -> None:
        """Add a bond with a named market convention.

        Uses pre-configured regional conventions for day count, frequency,
        settlement days, and business-day rules.

        Args:
            id: Unique instrument identifier
            notional: Principal amount (Money)
            coupon_rate: Annual coupon rate (e.g., 0.05 for 5%)
            issue_date: Bond issue date
            maturity_date: Bond maturity date
            convention: Convention name (e.g., "us_treasury", "german_bund",
                "uk_gilt", "corporate", "jgb", "french_oat", "us_agency")
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

        Uses default USD conventions (semi-annual fixed 30/360,
        quarterly float ACT/360, Modified Following).

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

    def add_swap_with_conventions(
        self,
        id: str,
        notional: Any,  # Money
        fixed_rate: float,
        start_date: Any,  # date
        maturity_date: Any,  # date
        discount_curve_id: str,
        forward_curve_id: str,
        fixed_freq: Any,  # Tenor
        fixed_dc: Any,  # DayCount
        float_freq: Any,  # Tenor
        float_dc: Any,  # DayCount
        bdc: Any,  # BusinessDayConvention
    ) -> None:
        """Add an interest rate swap with explicit leg conventions.

        Args:
            id: Unique instrument identifier
            notional: Notional amount (Money)
            fixed_rate: Fixed rate (e.g., 0.04 for 4%)
            start_date: Swap start date
            maturity_date: Swap maturity date
            discount_curve_id: Discount curve ID
            forward_curve_id: Forward curve ID for floating leg
            fixed_freq: Fixed leg payment frequency (Tenor)
            fixed_dc: Fixed leg day count (DayCount)
            float_freq: Float leg payment frequency (Tenor)
            float_dc: Float leg day count (DayCount)
            bdc: Business day convention (BusinessDayConvention)
        """
        ...

    def add_custom_debt(self, id: str, spec: Dict[str, Any]) -> None:
        """Add a generic debt instrument via JSON specification.

        Args:
            id: Unique instrument identifier
            spec: JSON specification for the debt instrument
        """
        ...

    def waterfall(self, waterfall_spec: WaterfallSpec) -> None:
        """Configure waterfall specification for dynamic cash flow allocation.

        Args:
            waterfall_spec: Waterfall configuration with ECF sweep and PIK toggle settings
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

    def add_registry_metrics(
        self,
        qualified_ids: List[str],
        registry: Any,
    ) -> None:
        """Add multiple metrics from a registry at once.

        Args:
            qualified_ids: List of fully qualified metric identifiers
            registry: Registry loaded by the caller
        """
        ...

    # ── Template helpers ─────────────────────────────────────────

    def add_roll_forward(
        self,
        name: str,
        increases: List[str],
        decreases: List[str],
    ) -> None:
        """Add a roll-forward structure (beginning + changes = ending).

        Creates ``{name}_beg`` and ``{name}_end`` calculated nodes where
        the beginning balance is the lagged ending balance and the ending
        balance is ``beg + sum(increases) - sum(decreases)``.

        Args:
            name: Base name for the roll-forward (e.g., ``"arr"``).
            increases: Node IDs that increase the balance.
            decreases: Node IDs that decrease the balance.
        """
        ...

    def add_vintage_buildup(
        self,
        name: str,
        new_volume_node: str,
        decay_curve: List[float],
    ) -> None:
        """Add a vintage buildup (cohort/convolution) structure.

        Models a stack of layers where each cohort originates from
        *new_volume_node* and evolves according to *decay_curve*.

        ``Total[t] = sum(NewVolume[t-k] * curve[k])`` for ``k`` in
        ``0..len(decay_curve)``.

        Args:
            name: Resulting total node name (e.g., ``"revenue"``).
            new_volume_node: Node ID for the new volume per period.
            decay_curve: Multipliers indexed by vintage age.
        """
        ...

    def add_noi_buildup(
        self,
        total_revenue_node: str,
        revenue_nodes: List[str],
        total_expenses_node: str,
        expense_nodes: List[str],
        noi_node: str,
    ) -> None:
        """Add a NOI (Net Operating Income) buildup template.

        Creates ``total_revenue = sum(revenue_nodes)``,
        ``total_expenses = sum(expense_nodes)``, and
        ``noi = total_revenue - total_expenses``.

        Args:
            total_revenue_node: Aggregated revenue node ID.
            revenue_nodes: Revenue line-item node IDs.
            total_expenses_node: Aggregated expense node ID.
            expense_nodes: Expense line-item node IDs.
            noi_node: Net operating income node ID.
        """
        ...

    def add_ncf_buildup(
        self,
        noi_node: str,
        capex_nodes: List[str],
        ncf_node: str,
    ) -> None:
        """Add a NCF (Net Cash Flow) buildup template.

        ``ncf = noi - sum(capex_nodes)``.  When *capex_nodes* is empty the
        NCF node is an alias of the NOI node.

        Args:
            noi_node: NOI source node ID.
            capex_nodes: Capital expenditure node IDs (positive outflows).
            ncf_node: Target NCF node ID.
        """
        ...

    def add_rent_roll_rental_revenue(
        self,
        leases: List[LeaseSpec],
        total_rent_node: str,
    ) -> None:
        """Add a rent-roll rental revenue projection (v1, simple leases).

        Creates one value node per lease and a computed total.

        Args:
            leases: Simple lease specifications.
            total_rent_node: Aggregated rental revenue node ID.
        """
        ...

    def add_rent_roll_rental_revenue_v2(
        self,
        leases: List[LeaseSpecV2],
        nodes: RentRollOutputNodes,
    ) -> None:
        """Add a rent-roll rental revenue projection (v2, enhanced leases).

        Creates per-lease detail nodes (``{id}.pgi``, ``{id}.free_rent``,
        ``{id}.vacancy_loss``, ``{id}.effective_rent``) and aggregated
        totals.

        Args:
            leases: Enhanced lease specifications.
            nodes: Output node names for the rent decomposition.
        """
        ...

    def add_property_operating_statement(
        self,
        leases: List[LeaseSpecV2],
        other_income_nodes: List[str],
        opex_nodes: List[str],
        capex_nodes: List[str],
        nodes: PropertyTemplateNodes,
        management_fee: ManagementFeeSpec | None = None,
    ) -> None:
        """Add a full property operating statement template.

        Combines rent roll, other income, expenses, management fees,
        and capex into a complete NOI-to-NCF waterfall:
        rent roll -> EGI -> NOI -> NCF.

        Args:
            leases: Enhanced lease specifications.
            other_income_nodes: Other income node IDs.
            opex_nodes: Operating expense node IDs.
            capex_nodes: Capital expenditure node IDs.
            nodes: Aggregate output node names.
            management_fee: Optional management fee specification.
        """
        ...

    def build(self) -> FinancialModelSpec:
        """Build the final model specification.

        Returns:
            FinancialModelSpec: Complete model specification
        """
        ...

    def __repr__(self) -> str: ...
