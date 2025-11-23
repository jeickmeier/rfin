"""Scenario specification bindings."""

from typing import List, Optional, Dict, Any, Tuple
from datetime import date
from ...core.currency import Currency
from ...valuations.common import InstrumentType
from .enums import CurveKind, VolSurfaceKind, TenorMatchMode

class OperationSpec:
    """Individual operation within a scenario.

    Use class methods to construct specific operation types.

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
        """Instrument price shock by exact attribute match.

        Args:
            attrs: Attribute filters (e.g., {"sector": "Energy", "rating": "BBB"})
            pct: Percentage change to apply

        Returns:
            OperationSpec: Operation specification
        """
        ...

    @classmethod
    def curve_parallel_bp(cls, curve_kind: CurveKind, curve_id: str, bp: float) -> OperationSpec:
        """Parallel shift to a curve (additive in basis points).

        Args:
            curve_kind: Type of curve to shock
            curve_id: Curve identifier
            bp: Basis points to add

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
        match_mode: Optional[TenorMatchMode] = None,
    ) -> OperationSpec:
        """Node-specific basis point shifts for curve shaping.

        Args:
            curve_kind: Type of curve to shock
            curve_id: Curve identifier
            nodes: List of (tenor, bp) pairs (e.g., [("2Y", 25.0), ("10Y", -10.0)])
            match_mode: Tenor matching strategy (default: Interpolate)

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
        detachment_bps: Optional[List[int]] = None,
        maturities: Optional[List[str]] = None,
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
        tenors: Optional[List[str]] = None,
        strikes: Optional[List[float]] = None,
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
        """Instrument spread shock by exact attribute match.

        Args:
            attrs: Attribute filters
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
    def time_roll_forward(cls, period: str, apply_shocks: Optional[bool] = True) -> OperationSpec:
        """Roll forward horizon by a period with carry/theta.

        Args:
            period: Period to roll forward (e.g., "1D", "1W", "1M", "1Y")
            apply_shocks: Whether to apply market shocks after rolling (default: True)

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
        name: Optional[str] = None,
        description: Optional[str] = None,
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
    def name(self) -> Optional[str]:
        """Display name.

        Returns:
            str | None: Name if set
        """
        ...

    @property
    def description(self) -> Optional[str]:
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

    def __repr__(self) -> str: ...
