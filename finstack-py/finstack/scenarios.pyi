"""
Python type stubs for finstack.scenarios module.

This module provides deterministic scenario capabilities for stress testing
and what-if analysis.

Note: Import as `import finstack` then access via `finstack.scenarios.*`
rather than `from finstack.scenarios import *`.
"""

from typing import Optional, List, Dict, Tuple, Any
from datetime import date
from finstack.core import Currency
from finstack.core.market_data import MarketContext
from finstack.statements.types import FinancialModelSpec
from finstack.valuations import InstrumentType

class CurveKind:
    """Identifies which family of curve an operation targets."""

    Discount: CurveKind
    Forecast: CurveKind
    Hazard: CurveKind
    Inflation: CurveKind

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class VolSurfaceKind:
    """Identifies which category of volatility surface an operation targets."""

    Equity: VolSurfaceKind
    Credit: VolSurfaceKind
    Swaption: VolSurfaceKind

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class TenorMatchMode:
    """Strategy for aligning requested tenor bumps with curve pillars."""

    Exact: TenorMatchMode
    Interpolate: TenorMatchMode

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __eq__(self, other: object) -> bool: ...
    def __hash__(self) -> int: ...

class OperationSpec:
    """Individual operation within a scenario."""

    @classmethod
    def market_fx_pct(cls, base: Currency, quote: Currency, pct: float) -> OperationSpec:
        """
        FX rate percent shift.

        Parameters
        ----------
        base : Currency
            Base currency.
        quote : Currency
            Quote currency.
        pct : float
            Percentage change (positive strengthens base).

        Returns
        -------
        OperationSpec
            Operation specification.
        """
        ...

    @classmethod
    def equity_price_pct(cls, ids: List[str], pct: float) -> OperationSpec:
        """
        Equity price percent shock.

        Parameters
        ----------
        ids : list[str]
            List of equity identifiers.
        pct : float
            Percentage change to apply.

        Returns
        -------
        OperationSpec
            Operation specification.
        """
        ...

    @classmethod
    def instrument_price_pct_by_attr(cls, attrs: Dict[str, str], pct: float) -> OperationSpec:
        """
        Instrument price shock by exact attribute match.

        Parameters
        ----------
        attrs : dict[str, str]
            Attribute filters.
        pct : float
            Percentage change to apply.

        Returns
        -------
        OperationSpec
            Operation specification.
        """
        ...

    @classmethod
    def curve_parallel_bp(cls, curve_kind: CurveKind, curve_id: str, bp: float) -> OperationSpec:
        """
        Parallel shift to a curve (additive in basis points).

        Parameters
        ----------
        curve_kind : CurveKind
            Type of curve to shock.
        curve_id : str
            Curve identifier.
        bp : float
            Basis points to add.

        Returns
        -------
        OperationSpec
            Operation specification.
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
        """
        Node-specific basis point shifts for curve shaping.

        Parameters
        ----------
        curve_kind : CurveKind
            Type of curve to shock.
        curve_id : str
            Curve identifier.
        nodes : list[tuple[str, float]]
            List of (tenor, bp) pairs.
        match_mode : TenorMatchMode, optional
            Tenor matching strategy (default: Interpolate).

        Returns
        -------
        OperationSpec
            Operation specification.
        """
        ...

    @classmethod
    def basecorr_parallel_pts(cls, surface_id: str, points: float) -> OperationSpec:
        """
        Parallel shift to base correlation surface (absolute points).

        Parameters
        ----------
        surface_id : str
            Surface identifier.
        points : float
            Correlation points to add.

        Returns
        -------
        OperationSpec
            Operation specification.
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
        """
        Bucket-specific base correlation shifts.

        Parameters
        ----------
        surface_id : str
            Surface identifier.
        points : float
            Correlation points to add.
        detachment_bps : list[int], optional
            Detachment points in basis points.
        maturities : list[str], optional
            Maturity filters.

        Returns
        -------
        OperationSpec
            Operation specification.
        """
        ...

    @classmethod
    def vol_surface_parallel_pct(cls, surface_kind: VolSurfaceKind, surface_id: str, pct: float) -> OperationSpec:
        """
        Parallel percent shift to volatility surface.

        Parameters
        ----------
        surface_kind : VolSurfaceKind
            Type of volatility surface.
        surface_id : str
            Surface identifier.
        pct : float
            Percentage change to apply.

        Returns
        -------
        OperationSpec
            Operation specification.
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
        """
        Bucketed volatility surface shock.

        Parameters
        ----------
        surface_kind : VolSurfaceKind
            Type of volatility surface.
        surface_id : str
            Surface identifier.
        pct : float
            Percentage change to apply.
        tenors : list[str], optional
            Tenor filters.
        strikes : list[float], optional
            Strike filters.

        Returns
        -------
        OperationSpec
            Operation specification.
        """
        ...

    @classmethod
    def stmt_forecast_percent(cls, node_id: str, pct: float) -> OperationSpec:
        """
        Statement forecast percent change.

        Parameters
        ----------
        node_id : str
            Node identifier.
        pct : float
            Percentage change to apply.

        Returns
        -------
        OperationSpec
            Operation specification.
        """
        ...

    @classmethod
    def stmt_forecast_assign(cls, node_id: str, value: float) -> OperationSpec:
        """
        Statement forecast value assignment.

        Parameters
        ----------
        node_id : str
            Node identifier.
        value : float
            Value to assign.

        Returns
        -------
        OperationSpec
            Operation specification.
        """
        ...

    @classmethod
    def instrument_spread_bp_by_attr(cls, attrs: Dict[str, str], bp: float) -> OperationSpec:
        """
        Instrument spread shock by exact attribute match.

        Parameters
        ----------
        attrs : dict[str, str]
            Attribute filters.
        bp : float
            Basis points to add.

        Returns
        -------
        OperationSpec
            Operation specification.
        """
        ...

    @classmethod
    def instrument_price_pct_by_type(cls, instrument_types: List[InstrumentType], pct: float) -> OperationSpec:
        """
        Instrument price shock by type.

        Parameters
        ----------
        instrument_types : list[InstrumentType]
            List of instrument types to shock.
        pct : float
            Percentage change to apply.

        Returns
        -------
        OperationSpec
            Operation specification.
        """
        ...

    @classmethod
    def instrument_spread_bp_by_type(cls, instrument_types: List[InstrumentType], bp: float) -> OperationSpec:
        """
        Instrument spread shock by type.

        Parameters
        ----------
        instrument_types : list[InstrumentType]
            List of instrument types to shock.
        bp : float
            Basis points to add.

        Returns
        -------
        OperationSpec
            Operation specification.
        """
        ...

    @classmethod
    def time_roll_forward(cls, period: str, apply_shocks: bool = True) -> OperationSpec:
        """
        Roll forward horizon by a period with carry/theta.

        Parameters
        ----------
        period : str
            Period to roll forward (e.g., "1D", "1W", "1M", "1Y").
        apply_shocks : bool, optional
            Whether to apply market shocks after rolling (default: True).

        Returns
        -------
        OperationSpec
            Operation specification.
        """
        ...

    def to_dict(self) -> Dict[str, Any]:
        """
        Convert to JSON-compatible dictionary.

        Returns
        -------
        dict
            JSON-serializable dictionary.
        """
        ...

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> OperationSpec:
        """
        Create from JSON-compatible dictionary.

        Parameters
        ----------
        data : dict
            JSON-serializable dictionary.

        Returns
        -------
        OperationSpec
            Operation specification.
        """
        ...

    def __repr__(self) -> str: ...

class ScenarioSpec:
    """A complete scenario specification with metadata and ordered operations."""

    def __init__(
        self,
        id: str,
        operations: List[OperationSpec],
        name: Optional[str] = None,
        description: Optional[str] = None,
        priority: int = 0,
    ) -> None:
        """
        Create a new scenario specification.

        Parameters
        ----------
        id : str
            Scenario identifier.
        operations : list[OperationSpec]
            List of operations to apply.
        name : str, optional
            Display name.
        description : str, optional
            Description text.
        priority : int, optional
            Priority for composition (default: 0).
        """
        ...

    @property
    def id(self) -> str:
        """Scenario identifier."""
        ...

    @property
    def name(self) -> Optional[str]:
        """Display name."""
        ...

    @property
    def description(self) -> Optional[str]:
        """Description text."""
        ...

    @property
    def operations(self) -> List[OperationSpec]:
        """List of operations."""
        ...

    @property
    def priority(self) -> int:
        """Priority for composition."""
        ...

    def to_dict(self) -> Dict[str, Any]:
        """
        Convert to JSON-compatible dictionary.

        Returns
        -------
        dict
            JSON-serializable dictionary.
        """
        ...

    def to_json(self) -> str:
        """
        Convert to JSON string.

        Returns
        -------
        str
            JSON string.
        """
        ...

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> ScenarioSpec:
        """
        Create from JSON-compatible dictionary.

        Parameters
        ----------
        data : dict
            JSON-serializable dictionary.

        Returns
        -------
        ScenarioSpec
            Scenario specification.
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> ScenarioSpec:
        """
        Create from JSON string.

        Parameters
        ----------
        json_str : str
            JSON string.

        Returns
        -------
        ScenarioSpec
            Scenario specification.
        """
        ...

    def __repr__(self) -> str: ...

class ApplicationReport:
    """Report describing what happened during scenario application."""

    @property
    def operations_applied(self) -> int:
        """Number of operations successfully applied."""
        ...

    @property
    def warnings(self) -> List[str]:
        """Warnings generated during application (non-fatal)."""
        ...

    @property
    def rounding_context(self) -> Optional[str]:
        """Rounding context stamp (for determinism tracking)."""
        ...

    def __repr__(self) -> str: ...

class RollForwardReport:
    """Report from time roll-forward operation."""

    @property
    def old_date(self) -> date:
        """Original as-of date."""
        ...

    @property
    def new_date(self) -> date:
        """New as-of date after roll."""
        ...

    @property
    def days(self) -> int:
        """Number of days rolled forward."""
        ...

    @property
    def instrument_carry(self) -> List[Tuple[str, float]]:
        """Per-instrument carry accrual."""
        ...

    @property
    def instrument_mv_change(self) -> List[Tuple[str, float]]:
        """Per-instrument market value change."""
        ...

    @property
    def total_carry(self) -> float:
        """Total P&L from carry."""
        ...

    @property
    def total_mv_change(self) -> float:
        """Total P&L from market value changes."""
        ...

    def __repr__(self) -> str: ...

class ExecutionContext:
    """Execution context for scenario application."""

    def __init__(
        self,
        market: MarketContext,
        model: FinancialModelSpec,
        as_of: date,
        instruments: Optional[List[Any]] = None,
        rate_bindings: Optional[Dict[str, str]] = None,
    ) -> None:
        """
        Create a new execution context.

        Parameters
        ----------
        market : MarketContext
            Market data context.
        model : FinancialModelSpec
            Financial model.
        as_of : date
            Valuation date.
        instruments : list, optional
            Optional instruments.
        rate_bindings : dict[str, str], optional
            Optional rate bindings.
        """
        ...

    @property
    def market(self) -> MarketContext:
        """Get the market context."""
        ...

    @property
    def model(self) -> FinancialModelSpec:
        """Get the financial model."""
        ...

    @property
    def as_of(self) -> date:
        """Get the valuation date."""
        ...

    @as_of.setter
    def as_of(self, value: date) -> None:
        """Set the valuation date."""
        ...

    @property
    def instruments(self) -> Optional[List[Any]]:
        """Get the instruments list."""
        ...

    @instruments.setter
    def instruments(self, value: Optional[List[Any]]) -> None:
        """Set the instruments list."""
        ...

    @property
    def rate_bindings(self) -> Optional[Dict[str, str]]:
        """Get the rate bindings."""
        ...

    @rate_bindings.setter
    def rate_bindings(self, value: Optional[Dict[str, str]]) -> None:
        """Set the rate bindings."""
        ...

    def __repr__(self) -> str: ...

class ScenarioEngine:
    """Orchestrates the deterministic application of a ScenarioSpec."""

    def __init__(self) -> None:
        """Create a new scenario engine with default settings."""
        ...

    def compose(self, scenarios: List[ScenarioSpec]) -> ScenarioSpec:
        """
        Compose multiple scenarios into a single deterministic spec.

        Operations are sorted by (priority, declaration_index); conflicts use last-wins.

        Parameters
        ----------
        scenarios : list[ScenarioSpec]
            Collection of scenario specifications to combine.

        Returns
        -------
        ScenarioSpec
            Combined scenario containing all operations with deterministic ordering.
        """
        ...

    def apply(self, scenario: ScenarioSpec, context: ExecutionContext) -> ApplicationReport:
        """
        Apply a scenario specification to the execution context.

        Operations are applied in this order:
        1. Market data (FX, equities, vol surfaces, curves, base correlation)
        2. Rate bindings update (if configured)
        3. Statement forecast adjustments
        4. Statement re-evaluation

        Parameters
        ----------
        scenario : ScenarioSpec
            Scenario specification to apply.
        context : ExecutionContext
            Mutable execution context that supplies market data, statements,
            instruments, and rate bindings.

        Returns
        -------
        ApplicationReport
            Summary of how many operations were applied and any warnings.

        Raises
        ------
        ValueError
            If operation cannot be completed.
        """
        ...

    def __repr__(self) -> str: ...

__all__ = [
    "CurveKind",
    "VolSurfaceKind",
    "TenorMatchMode",
    "OperationSpec",
    "ScenarioSpec",
    "ApplicationReport",
    "RollForwardReport",
    "ExecutionContext",
    "ScenarioEngine",
]
