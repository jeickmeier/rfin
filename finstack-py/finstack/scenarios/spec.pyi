"""Scenario specification bindings for the finstack.scenarios module.

These types are thin, documented wrappers over the Rust structs. They are
fully serializable (``to_dict``/``from_dict``/``to_json``) and contain no
Python-side business logic — everything executes in Rust.

Use :class:`OperationSpec` to declare shocks, :class:`ScenarioSpec` to group
them, and :class:`RateBindingSpec` to link market curves into statement nodes.
"""

from __future__ import annotations
from typing import List, Dict, Any, Tuple
from datetime import date
from finstack.core.currency import Currency
from finstack.valuations.common import InstrumentType
from .enums import CurveKind, VolSurfaceKind, TenorMatchMode

class Compounding:
    """Compounding convention for rate conversions.

    Use the class attributes to select a convention:
    ``Compounding.Simple``, ``Continuous`` (default), ``Annual``,
    ``SemiAnnual``, ``Quarterly``, ``Monthly``.
    """

    Simple: Compounding
    Continuous: Compounding
    Annual: Compounding
    SemiAnnual: Compounding
    Quarterly: Compounding
    Monthly: Compounding

class TimeRollMode:
    """Controls how time roll-forward periods are interpreted.

    - ``BusinessDays`` (default): Calendar-aware roll using provided holiday calendars and ModifiedFollowing.
    - ``CalendarDays``: Pure calendar addition, no business-day adjustment.
    - ``Approximate``: Legacy fixed-day approximations (30/365 style).
    """

    BusinessDays: TimeRollMode
    CalendarDays: TimeRollMode
    Approximate: TimeRollMode

class RateBindingSpec:
    """Configuration for rate binding between curves and statement nodes.

    A rate binding tells the scenario engine how to pull a rate off a curve
    and feed it into a statement forecast node. All computation happens in
    Rust; this class only carries data.
    """

    def __init__(
        self,
        node_id: str,
        curve_id: str,
        tenor: str,
        compounding: Compounding | None = None,
        day_count: str | None = None,
    ) -> None: ...
    @property
    def node_id(self) -> str:
        """Statement node ID to receive the bound rate."""

    @property
    def curve_id(self) -> str:
        """Curve identifier to extract the rate from."""

    @property
    def tenor(self) -> str:
        """Tenor string (e.g., ``\"3M\"``) used for rate extraction."""

    @property
    def compounding(self) -> Compounding:
        """Compounding convention used when converting the extracted rate."""

    @property
    def day_count(self) -> str | None:
        """Optional day-count override; defaults to the curve's convention."""

    def to_dict(self) -> Dict[str, Any]:
        """Convert to a JSON-serializable mapping."""

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> RateBindingSpec:
        """Create a binding spec from a JSON-style mapping."""

    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...
    def __repr__(self) -> str: ...

class OperationSpec:
    """Individual operation within a scenario.

    Use class methods to construct specific operation types. Operations are
    data-only; they are passed straight to the Rust engine for execution.

    Categories
    ----------
    - Market data: ``market_fx_pct``, ``curve_parallel_bp``, ``curve_node_bp``,
      ``vol_surface_parallel_pct``, ``vol_surface_bucket_pct``,
      ``basecorr_parallel_pts``, ``basecorr_bucket_pts``
    - Instruments: ``instrument_price_pct_by_type``, ``instrument_spread_bp_by_type``,
      ``instrument_price_pct_by_attr``, ``instrument_spread_bp_by_attr``
    - Statements: ``stmt_forecast_percent``, ``stmt_forecast_assign``
    - Structured credit: ``asset_correlation_pts``,
      ``prepay_default_correlation_pts``, ``recovery_correlation_pts``,
      ``prepay_factor_loading_pts``
    - Time: ``time_roll_forward``

    Examples:
        >>> from finstack.scenarios import OperationSpec, CurveKind
        >>> op = OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD_SOFR", 50.0)
    """

    @classmethod
    def market_fx_pct(cls, base: Currency, quote: Currency, pct: float) -> OperationSpec:
        """FX rate percent shift.

        Args:
            base: Base currency
            quote: Quote currency
            pct: Percentage change (positive strengthens base)

        Returns:
            OperationSpec: Operation specification
        """
        ...

    @classmethod
    def equity_price_pct(cls, ids: List[str], pct: float) -> OperationSpec:
        """Equity price percent shock.

        Args:
            ids: List of equity identifiers
            pct: Percentage change to apply

        Returns:
            OperationSpec: Operation specification
        """
        ...

    @classmethod
    def instrument_price_pct_by_attr(cls, attrs: Dict[str, str], pct: float) -> OperationSpec:
        """Instrument price shock by attribute match.

        Attributes are matched with AND semantics on metadata (case-insensitive keys/values).

        Args:
            attrs: Attribute filters (e.g., {"sector": "Energy", "rating": "BBB"})
            pct: Percentage change to apply

        Returns:
            OperationSpec: Operation specification
        """
        ...

    @classmethod
    def curve_parallel_bp(
        cls,
        curve_kind: CurveKind,
        curve_id: str,
        bp: float,
        discount_curve_id: str | None = None,
    ) -> OperationSpec:
        """Parallel shift to a curve (additive in basis points).

        Args:
            curve_kind: Type of curve to shock
            curve_id: Curve identifier
            bp: Basis points to add
            discount_curve_id: Optional explicit discount curve identifier for
                recalibration-based bumps

        Returns:
            OperationSpec: Operation specification
        """
        ...

    @classmethod
    def curve_node_bp(
        cls,
        curve_kind: CurveKind,
        curve_id: str,
        nodes: List[Tuple[str, float]],
        match_mode: TenorMatchMode | None = None,
        discount_curve_id: str | None = None,
    ) -> OperationSpec:
        """Node-specific basis point shifts for curve shaping.

        Args:
            curve_kind: Type of curve to shock
            curve_id: Curve identifier
            nodes: List of (tenor, bp) pairs (e.g., [("2Y", 25.0), ("10Y", -10.0)])
            match_mode: Tenor matching strategy (default: Interpolate)
            discount_curve_id: Optional explicit discount curve identifier for
                recalibration-based bumps

        Returns:
            OperationSpec: Operation specification
        """
        ...

    @classmethod
    def basecorr_parallel_pts(cls, surface_id: str, points: float) -> OperationSpec:
        """Parallel shift to base correlation surface (absolute points).

        Args:
            surface_id: Surface identifier
            points: Correlation points to add (e.g., 0.05 for +5 percentage points)

        Returns:
            OperationSpec: Operation specification
        """
        ...

    @classmethod
    def basecorr_bucket_pts(
        cls,
        surface_id: str,
        points: float,
        detachment_bps: List[int] | None = None,
        maturities: List[str] | None = None,
    ) -> OperationSpec:
        """Bucket-specific base correlation shifts.

        Args:
            surface_id: Surface identifier
            points: Correlation points to add
            detachment_bps: Detachment points in basis points (e.g., [300, 700] for 3% and 7%)
            maturities: Maturity filters

        Returns:
            OperationSpec: Operation specification
        """
        ...

    @classmethod
    def vol_surface_parallel_pct(cls, surface_kind: VolSurfaceKind, surface_id: str, pct: float) -> OperationSpec:
        """Parallel percent shift to volatility surface.

        Args:
            surface_kind: Type of volatility surface
            surface_id: Surface identifier
            pct: Percentage change to apply

        Returns:
            OperationSpec: Operation specification
        """
        ...

    @classmethod
    def vol_surface_bucket_pct(
        cls,
        surface_kind: VolSurfaceKind,
        surface_id: str,
        pct: float,
        tenors: List[str] | None = None,
        strikes: List[float] | None = None,
    ) -> OperationSpec:
        """Bucketed volatility surface shock.

        Args:
            surface_kind: Type of volatility surface
            surface_id: Surface identifier
            pct: Percentage change to apply
            tenors: Tenor filters (e.g., ["1M", "3M"])
            strikes: Strike filters

        Returns:
            OperationSpec: Operation specification
        """
        ...

    @classmethod
    def stmt_forecast_percent(cls, node_id: str, pct: float) -> OperationSpec:
        """Statement forecast percent change.

        Args:
            node_id: Node identifier
            pct: Percentage change to apply

        Returns:
            OperationSpec: Operation specification
        """
        ...

    @classmethod
    def stmt_forecast_assign(cls, node_id: str, value: float) -> OperationSpec:
        """Statement forecast value assignment.

        Args:
            node_id: Node identifier
            value: Value to assign

        Returns:
            OperationSpec: Operation specification
        """
        ...

    @classmethod
    def instrument_spread_bp_by_attr(cls, attrs: Dict[str, str], bp: float) -> OperationSpec:
        """Instrument spread shock by attribute match.

        Args:
            attrs: Attribute filters (case-insensitive keys/values, AND semantics)
            bp: Basis points to add

        Returns:
            OperationSpec: Operation specification
        """
        ...

    @classmethod
    def instrument_price_pct_by_type(cls, instrument_types: List[InstrumentType], pct: float) -> OperationSpec:
        """Instrument price shock by type.

        Args:
            instrument_types: List of instrument types to shock
            pct: Percentage change to apply

        Returns:
            OperationSpec: Operation specification
        """
        ...

    @classmethod
    def instrument_spread_bp_by_type(cls, instrument_types: List[InstrumentType], bp: float) -> OperationSpec:
        """Instrument spread shock by type.

        Args:
            instrument_types: List of instrument types to shock
            bp: Basis points to add

        Returns:
            OperationSpec: Operation specification
        """
        ...

    @classmethod
    def asset_correlation_pts(cls, delta_pts: float) -> OperationSpec:
        """Shock asset correlation for structured credit instruments.

        Args:
            delta_pts: Additive shock in correlation points (e.g., 0.05 for +5%)

        Returns:
            OperationSpec: Operation specification
        """
        ...

    @classmethod
    def prepay_default_correlation_pts(cls, delta_pts: float) -> OperationSpec:
        """Shock prepay-default correlation for structured credit instruments.

        Args:
            delta_pts: Additive shock in correlation points

        Returns:
            OperationSpec: Operation specification
        """
        ...

    @classmethod
    def recovery_correlation_pts(cls, delta_pts: float) -> OperationSpec:
        """Shock recovery-default correlation for structured credit instruments.

        Args:
            delta_pts: Additive shock in correlation points

        Returns:
            OperationSpec: Operation specification
        """
        ...

    @classmethod
    def prepay_factor_loading_pts(cls, delta_pts: float) -> OperationSpec:
        """Shock prepayment factor loading (systematic factor sensitivity).

        Args:
            delta_pts: Additive shock to factor loading

        Returns:
            OperationSpec: Operation specification
        """
        ...

    @classmethod
    def time_roll_forward(
        cls, period: str, apply_shocks: bool | None = True, roll_mode: TimeRollMode | None = None
    ) -> OperationSpec:
        """Roll forward horizon by a period with carry/theta.

        Args:
            period: Period to roll forward (e.g., "1D", "1W", "1M", "1Y")
            apply_shocks: Whether to apply market shocks after rolling (default: True)
            roll_mode: Roll interpretation (business days by default)

        Returns:
            OperationSpec: Operation specification
        """
        ...

    def to_dict(self) -> Dict[str, Any]:
        """Convert to JSON-compatible dictionary.

        Returns:
            dict: JSON-serializable dictionary
        """
        ...

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> OperationSpec:
        """Create from JSON-compatible dictionary.

        Args:
            data: JSON-serializable dictionary

        Returns:
            OperationSpec: Operation specification
        """
        ...

    def validate(self) -> None:
        """Validate this operation for consistency.

        Raises:
            ValueError: If the operation is invalid (NaN values, empty IDs, etc.)
        """
        ...

    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...
    def __repr__(self) -> str: ...

class ScenarioSpec:
    """Complete scenario specification with metadata and ordered operations.

    ScenarioSpec represents a deterministic scenario for stress testing and
    what-if analysis. It contains an ordered list of operations that modify
    market data, statement models, or instrument prices. Scenarios can be
    composed from multiple specs with conflict resolution.

    Scenarios are used for:
    - Stress testing (rate shocks, FX moves, equity crashes)
    - What-if analysis (forecast adjustments, model parameter changes)
    - Sensitivity analysis (parallel shifts, key-rate shocks)
    - Time roll-forward (carry/theta calculations)

    Examples
    --------
    Create a rate shock scenario:

        >>> from finstack.scenarios import ScenarioSpec, OperationSpec, CurveKind
        >>> from finstack.core.currency import Currency
        >>> # +50bp parallel shift to USD discount curve
        >>> ops = [
        ...     OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD-SOFR", 50.0),
        ...     OperationSpec.market_fx_pct(Currency("EUR"), Currency("USD"), -0.05),
        ... ]
        >>> scenario = ScenarioSpec(
        ...     "rate_shock_50bp",
        ...     ops,
        ...     name="+50bp Rate Shock",
        ...     description="Parallel shift to USD discount curve",
        ... )

    Compose scenarios:

        >>> from finstack.scenarios import ScenarioSpec, ScenarioEngine, OperationSpec, CurveKind
        >>> from finstack.core.currency import Currency
        >>> base_ops = [OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD-SOFR", 50.0)]
        >>> overlay_ops = [OperationSpec.market_fx_pct(Currency("EUR"), Currency("USD"), -0.05)]
        >>> base_scenario = ScenarioSpec("base", base_ops, priority=1)
        >>> overlay_scenario = ScenarioSpec("overlay", overlay_ops, priority=2)
        >>> engine = ScenarioEngine()
        >>> combined = engine.compose([base_scenario, overlay_scenario])

    Serialize to JSON:

        >>> from finstack.scenarios import ScenarioSpec, OperationSpec, CurveKind
        >>> ops = [OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD-SOFR", 25.0)]
        >>> scenario = ScenarioSpec("serialize_me", ops, name="Serialize Me")
        >>> json_str = scenario.to_json()
        >>> restored = ScenarioSpec.from_json(json_str)
        >>> restored.name
        'Serialize Me'

    Notes
    -----
    - Scenarios are deterministic and reproducible
    - Operations are applied in order
    - Priority determines merge order in composition (lower = higher priority)
    - Scenarios can be serialized to JSON for persistence

    See Also
    --------
    :class:`OperationSpec`: Individual operations
    :class:`ScenarioEngine`: Scenario execution engine
    :class:`ExecutionContext`: Execution context
    """

    def __init__(
        self,
        id: str,
        operations: List[OperationSpec],
        name: str | None = None,
        description: str | None = None,
        priority: int = 0,
    ) -> None:
        """Create a new scenario specification.

        Args:
            id: Scenario identifier
            operations: List of operations to apply
            name: Display name
            description: Description text
            priority: Priority for composition (default: 0)
        """
        ...

    @property
    def id(self) -> str:
        """Scenario identifier.

        Returns:
            str: Scenario ID
        """
        ...

    @property
    def name(self) -> str | None:
        """Display name.

        Returns:
            str | None: Name if set
        """
        ...

    @property
    def description(self) -> str | None:
        """Description text.

        Returns:
            str | None: Description if set
        """
        ...

    @property
    def operations(self) -> List[OperationSpec]:
        """List of operations.

        Returns:
            list[OperationSpec]: Operations to apply
        """
        ...

    @property
    def priority(self) -> int:
        """Priority for composition.

        Returns:
            int: Priority value (lower = higher priority)
        """
        ...

    def to_dict(self) -> Dict[str, Any]:
        """Convert to JSON-compatible dictionary.

        Returns:
            dict: JSON-serializable dictionary
        """
        ...

    def to_json(self) -> str:
        """Convert to JSON string.

        Returns:
            str: JSON string
        """
        ...

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> ScenarioSpec:
        """Create from JSON-compatible dictionary.

        Args:
            data: JSON-serializable dictionary

        Returns:
            ScenarioSpec: Scenario specification
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> ScenarioSpec:
        """Create from JSON string.

        Args:
            json_str: JSON string

        Returns:
            ScenarioSpec: Scenario specification
        """
        ...

    def validate(self) -> None:
        """Validate this scenario specification for consistency.

        Checks for non-empty ID, valid operations, and at most one
        TimeRollForward operation.

        Raises:
            ValueError: If the specification is invalid
        """
        ...

    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...
    def __repr__(self) -> str: ...
