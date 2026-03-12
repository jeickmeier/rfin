"""P&L Attribution type stubs."""

from __future__ import annotations
from datetime import date
from typing import Dict, List, Mapping, Any, Tuple
from finstack.core.money import Money
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.scalars import MarketScalar, ScalarTimeSeries
from finstack.core.market_data.surfaces import VolSurface
from finstack.core.market_data.term_structures import (
    BaseCorrelationCurve,
    DiscountCurve,
    ForwardCurve,
    HazardCurve,
    InflationCurve,
)
from finstack.portfolio import Portfolio

class AttributionMethod:
    """Attribution methodology selector."""

    @staticmethod
    def parallel() -> AttributionMethod:
        """Independent factor isolation (may not sum due to cross-effects).

        Each factor is analyzed separately by restoring T₀ values for that
        factor while keeping all others at T₁. Residual captures cross-effects.

        Returns:
            AttributionMethod configured for parallel attribution
        """
        ...

    @staticmethod
    def waterfall(factors: List[str]) -> AttributionMethod:
        """Sequential waterfall (guarantees sum = total, order matters).

        Factors are applied one-by-one in the specified order. Each factor's
        P&L is computed with all previous factors at T₁ and remaining at T₀.

        Args:
            factors: Ordered list of factor names:
                - "carry"
                - "rates_curves"
                - "credit_curves"
                - "inflation_curves"
                - "correlations"
                - "fx"
                - "volatility"
                - "model_parameters"
                - "market_scalars"

        Returns:
            AttributionMethod configured for waterfall attribution

        Raises:
            ValueError: If unknown factor name provided
        """
        ...

    @staticmethod
    def metrics_based() -> AttributionMethod:
        """Use existing metrics (Theta, DV01, CS01) for approximation.

        Fast linear approximation using pre-computed sensitivities. Less
        accurate for large market moves due to convexity effects.

        Returns:
            AttributionMethod configured for metrics-based attribution
        """
        ...

    @staticmethod
    def taylor(config: TaylorAttributionConfig | None = None) -> AttributionMethod:
        """Sensitivity-based Taylor expansion attribution."""
        ...

class AttributionMeta:
    """Attribution metadata."""

    @property
    def method(self) -> AttributionMethod:
        """Attribution method used."""
        ...

    @property
    def t0(self) -> date:
        """Start date (T₀)."""
        ...

    @property
    def t1(self) -> date:
        """End date (T₁)."""
        ...

    @property
    def instrument_id(self) -> str:
        """Instrument identifier."""
        ...

    @property
    def num_repricings(self) -> int:
        """Number of repricings performed."""
        ...

    @property
    def residual_pct(self) -> float:
        """Residual as percentage of total P&L."""
        ...

    @property
    def tolerance_abs(self) -> float:
        """Absolute tolerance for residual validation."""
        ...

    @property
    def tolerance_pct(self) -> float:
        """Relative tolerance for residual validation."""
        ...

class RatesCurvesAttribution:
    """Detailed attribution for interest rate curves."""

    def by_curve_to_dict(self) -> Dict[str, Money]:
        """Get P&L by curve ID.

        Returns:
            Dictionary mapping curve ID to P&L amount
        """
        ...

    def by_tenor_to_dict(self) -> Dict[Tuple[str, str], Money]:
        """Get P&L by (curve ID, tenor)."""
        ...

    @property
    def discount_total(self) -> Money:
        """Total discount curves P&L."""
        ...

    @property
    def forward_total(self) -> Money:
        """Total forward curves P&L."""
        ...

class CreditCurvesAttribution:
    """Detailed attribution for credit hazard curves."""

    def by_curve_to_dict(self) -> Dict[str, Money]:
        """Get P&L by hazard curve ID.

        Returns:
            Dictionary mapping curve ID to P&L amount
        """
        ...

    def by_tenor_to_dict(self) -> Dict[Tuple[str, str], Money]:
        """Get P&L by (hazard curve ID, tenor)."""
        ...

class ModelParamsAttribution:
    """Detailed attribution for model-specific parameters."""

    @property
    def prepayment(self) -> Money | None:
        """Prepayment speed changes P&L (for MBS/ABS)."""
        ...

    @property
    def default_rate(self) -> Money | None:
        """Default rate changes P&L (for structured credit)."""
        ...

    @property
    def recovery_rate(self) -> Money | None:
        """Recovery rate changes P&L (for credit instruments)."""
        ...

    @property
    def conversion_ratio(self) -> Money | None:
        """Conversion ratio changes P&L (for convertible bonds)."""
        ...

class CarryDetail:
    """Detailed carry decomposition."""

    def __init__(
        self,
        total: Money,
        *,
        theta: Money | None = None,
        roll_down: Money | None = None,
    ) -> None: ...
    @property
    def total(self) -> Money: ...
    @property
    def theta(self) -> Money | None: ...
    @property
    def roll_down(self) -> Money | None: ...

class InflationCurvesAttribution:
    """Detailed attribution for inflation curves."""

    def __init__(
        self,
        by_curve: Dict[str, Money],
        *,
        by_tenor: Dict[Tuple[str, str], Money] | None = None,
    ) -> None: ...
    def by_curve_to_dict(self) -> Dict[str, Money]: ...
    def by_tenor_to_dict(self) -> Dict[Tuple[str, str], Money] | None: ...

class CorrelationsAttribution:
    """Detailed attribution for correlation curves."""

    def __init__(self, by_curve: Dict[str, Money]) -> None: ...
    def by_curve_to_dict(self) -> Dict[str, Money]: ...

class FxAttribution:
    """Detailed attribution for FX rate moves."""

    def __init__(self, by_pair: Dict[Tuple[str, str], Money]) -> None: ...
    def by_pair_to_dict(self) -> Dict[Tuple[str, str], Money]: ...

class VolAttribution:
    """Detailed attribution for volatility surface moves."""

    def __init__(self, by_surface: Dict[str, Money]) -> None: ...
    def by_surface_to_dict(self) -> Dict[str, Money]: ...

class ScalarsAttribution:
    """Detailed attribution for market scalar moves."""

    def __init__(
        self,
        *,
        dividends: Dict[str, Money] | None = None,
        inflation: Dict[str, Money] | None = None,
        equity_prices: Dict[str, Money] | None = None,
        commodity_prices: Dict[str, Money] | None = None,
    ) -> None: ...
    def dividends_to_dict(self) -> Dict[str, Money]: ...
    def inflation_to_dict(self) -> Dict[str, Money]: ...
    def equity_prices_to_dict(self) -> Dict[str, Money]: ...
    def commodity_prices_to_dict(self) -> Dict[str, Money]: ...

class TaylorAttributionConfig:
    """Configuration for Taylor-based P&L attribution."""

    def __init__(
        self,
        *,
        include_gamma: bool = False,
        rate_bump_bp: float = 1.0,
        credit_bump_bp: float = 1.0,
        vol_bump: float = 0.01,
    ) -> None: ...
    @property
    def include_gamma(self) -> bool: ...
    @property
    def rate_bump_bp(self) -> float: ...
    @property
    def credit_bump_bp(self) -> float: ...
    @property
    def vol_bump(self) -> float: ...

class TaylorFactorResult:
    """Per-factor Taylor attribution contribution."""

    def __init__(
        self,
        factor_name: str,
        sensitivity: float,
        market_move: float,
        explained_pnl: float,
        *,
        gamma_pnl: float | None = None,
    ) -> None: ...
    @property
    def factor_name(self) -> str: ...
    @property
    def sensitivity(self) -> float: ...
    @property
    def market_move(self) -> float: ...
    @property
    def explained_pnl(self) -> float: ...
    @property
    def gamma_pnl(self) -> float | None: ...

class TaylorAttributionResult:
    """Complete Taylor attribution result."""

    def __init__(
        self,
        actual_pnl: float,
        total_explained: float,
        unexplained: float,
        unexplained_pct: float,
        factors: List[TaylorFactorResult],
        num_repricings: int,
        pv_t0: Money,
        pv_t1: Money,
    ) -> None: ...
    @property
    def actual_pnl(self) -> float: ...
    @property
    def total_explained(self) -> float: ...
    @property
    def unexplained(self) -> float: ...
    @property
    def unexplained_pct(self) -> float: ...
    @property
    def factors(self) -> List[TaylorFactorResult]: ...
    @property
    def num_repricings(self) -> int: ...
    @property
    def pv_t0(self) -> Money: ...
    @property
    def pv_t1(self) -> Money: ...

class CurveRestoreFlags:
    """Bitflag-like selector for restoring market curve families."""

    DISCOUNT: CurveRestoreFlags
    FORWARD: CurveRestoreFlags
    HAZARD: CurveRestoreFlags
    INFLATION: CurveRestoreFlags
    CORRELATION: CurveRestoreFlags
    RATES: CurveRestoreFlags
    CREDIT: CurveRestoreFlags

    @classmethod
    def all(cls) -> CurveRestoreFlags: ...
    @classmethod
    def empty(cls) -> CurveRestoreFlags: ...
    def contains(self, other: CurveRestoreFlags) -> bool: ...
    def __or__(self, other: CurveRestoreFlags) -> CurveRestoreFlags: ...
    def __and__(self, other: CurveRestoreFlags) -> CurveRestoreFlags: ...
    def __invert__(self) -> CurveRestoreFlags: ...

class MarketSnapshot:
    """Snapshot of selected curve families from a market context."""

    @classmethod
    def extract(cls, market: MarketContext, flags: CurveRestoreFlags) -> MarketSnapshot: ...
    @staticmethod
    def restore_market(
        current_market: MarketContext,
        snapshot: MarketSnapshot,
        restore_flags: CurveRestoreFlags,
    ) -> MarketContext: ...
    def discount_curves(self) -> Dict[str, DiscountCurve]: ...
    def forward_curves(self) -> Dict[str, ForwardCurve]: ...
    def hazard_curves(self) -> Dict[str, HazardCurve]: ...
    def inflation_curves(self) -> Dict[str, InflationCurve]: ...
    def base_correlation_curves(self) -> Dict[str, BaseCorrelationCurve]: ...

class VolatilitySnapshot:
    """Snapshot of volatility surfaces."""

    @classmethod
    def extract(cls, market: MarketContext) -> VolatilitySnapshot: ...
    def surfaces(self) -> Dict[str, VolSurface]: ...

class ScalarsSnapshot:
    """Snapshot of market scalar data."""

    @classmethod
    def extract(cls, market: MarketContext) -> ScalarsSnapshot: ...
    def prices(self) -> Dict[str, MarketScalar]: ...
    def series(self) -> Dict[str, ScalarTimeSeries]: ...

class PnlAttribution:
    """P&L attribution result for a single instrument."""

    @property
    def total_pnl(self) -> Money:
        """Total P&L (val_t1 - val_t0)."""
        ...

    @property
    def carry(self) -> Money:
        """Carry P&L (theta + accruals)."""
        ...

    @property
    def rates_curves_pnl(self) -> Money:
        """Interest rate curves P&L."""
        ...

    @property
    def credit_curves_pnl(self) -> Money:
        """Credit hazard curves P&L."""
        ...

    @property
    def inflation_curves_pnl(self) -> Money:
        """Inflation curves P&L."""
        ...

    @property
    def correlations_pnl(self) -> Money:
        """Base correlation curves P&L."""
        ...

    @property
    def fx_pnl(self) -> Money:
        """FX rate changes P&L."""
        ...

    @property
    def vol_pnl(self) -> Money:
        """Implied volatility changes P&L."""
        ...

    @property
    def model_params_pnl(self) -> Money:
        """Model parameters P&L."""
        ...

    @property
    def market_scalars_pnl(self) -> Money:
        """Market scalars P&L."""
        ...

    @property
    def residual(self) -> Money:
        """Residual P&L."""
        ...

    @property
    def meta(self) -> AttributionMeta:
        """Attribution metadata."""
        ...

    @property
    def rates_detail(self) -> RatesCurvesAttribution | None:
        """Detailed rates curves attribution."""
        ...

    @property
    def credit_detail(self) -> CreditCurvesAttribution | None:
        """Detailed credit curves attribution."""
        ...

    @property
    def model_params_detail(self) -> ModelParamsAttribution | None:
        """Detailed model parameters attribution."""
        ...

    @property
    def carry_detail(self) -> CarryDetail | None:
        """Detailed carry attribution."""
        ...

    @property
    def inflation_detail(self) -> InflationCurvesAttribution | None:
        """Detailed inflation curves attribution."""
        ...

    @property
    def correlations_detail(self) -> CorrelationsAttribution | None:
        """Detailed correlations attribution."""
        ...

    @property
    def fx_detail(self) -> FxAttribution | None:
        """Detailed FX attribution."""
        ...

    @property
    def vol_detail(self) -> VolAttribution | None:
        """Detailed volatility attribution."""
        ...

    @property
    def scalars_detail(self) -> ScalarsAttribution | None:
        """Detailed market scalars attribution."""
        ...

    def to_csv(self) -> str:
        """Export attribution as CSV string.

        Returns:
            CSV string with headers and data row containing all factors
        """
        ...

    def to_json(self) -> str:
        """Export attribution as JSON string.

        Requires serde feature enabled in Rust build.

        Returns:
            JSON string with complete attribution data

        Raises:
            RuntimeError: If serde feature not enabled
        """
        ...

    def rates_detail_to_csv(self) -> str | None:
        """Export rates curves detail as CSV.

        Returns:
            CSV string with curve-by-curve breakdown, or None if no detail available
        """
        ...

    def explain(self) -> str:
        """Generate structured tree explanation of P&L attribution.

        Returns:
            Multi-line string with tree structure showing factor breakdown

        Notes:
            Call :func:`attribute_pnl` or :func:`attribute_portfolio_pnl` to obtain a
            ``PnlAttribution`` instance (``attr`` in the examples above) and then
            invoke ``attr.explain()`` to render the formatted tree.
        """
        ...

    def residual_within_tolerance(self, pct_tolerance: float, abs_tolerance: float) -> bool:
        """Check if residual is within acceptable tolerance.

        Tolerance is the larger of percentage-based (relative to total P&L) or
        absolute value.

        Args:
            pct_tolerance: Percentage tolerance (e.g., 0.1 for 0.1%)
            abs_tolerance: Absolute tolerance (e.g., 100.0 for $100)

        Returns:
            True if residual is within tolerance
        """
        ...

class PortfolioAttribution:
    """Portfolio-level P&L attribution result."""

    @property
    def total_pnl(self) -> Money:
        """Total portfolio P&L in base currency."""
        ...

    @property
    def carry(self) -> Money:
        """Carry P&L (theta + accruals)."""
        ...

    @property
    def rates_curves_pnl(self) -> Money:
        """Interest rate curves P&L."""
        ...

    @property
    def credit_curves_pnl(self) -> Money:
        """Credit hazard curves P&L."""
        ...

    @property
    def inflation_curves_pnl(self) -> Money:
        """Inflation curves P&L."""
        ...

    @property
    def correlations_pnl(self) -> Money:
        """Base correlation curves P&L."""
        ...

    @property
    def fx_pnl(self) -> Money:
        """FX rate changes P&L."""
        ...

    @property
    def vol_pnl(self) -> Money:
        """Implied volatility changes P&L."""
        ...

    @property
    def model_params_pnl(self) -> Money:
        """Model parameters P&L."""
        ...

    @property
    def market_scalars_pnl(self) -> Money:
        """Market scalars P&L."""
        ...

    @property
    def residual(self) -> Money:
        """Residual P&L."""
        ...

    def by_position_to_dict(self) -> Dict[str, PnlAttribution]:
        """Get attribution by position ID.

        Returns:
            Dictionary mapping position ID to PnlAttribution
        """
        ...

    def to_csv(self) -> str:
        """Export portfolio attribution summary as CSV."""
        ...

    def position_detail_to_csv(self) -> str:
        """Export position-by-position detail as CSV."""
        ...

    def explain(self) -> str:
        """Generate explanation tree for portfolio attribution."""
        ...

def attribute_pnl(
    instrument: Any,
    market_t0: MarketContext,
    market_t1: MarketContext,
    as_of_t0: date,
    as_of_t1: date,
    method: AttributionMethod | None = None,
    model_params_t0: Mapping[str, Any] | str | None = None,
) -> PnlAttribution:
    """Perform P&L attribution for an instrument.

    Decomposes total P&L between T₀ and T₁ into constituent factors:
    carry, curve shifts, spread changes, FX, volatility, model parameters,
    and market scalars.

    Args:
        instrument: Any finstack instrument (Bond, IRS, Equity, StructuredCredit, etc.)
        market_t0: Market context at T₀
        market_t1: Market context at T₁
        as_of_t0: Valuation date at T₀
        as_of_t1: Valuation date at T₁
        method: Attribution methodology (defaults to Parallel)
        model_params_t0: Optional dict/JSON describing T₀ model parameters

    Returns:
        Complete P&L attribution with factor breakdown

    Raises:
        ValueError: If instrument type not supported or dates invalid
        RuntimeError: If pricing fails

    Example:
        >>> from datetime import date
        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.term_structures import DiscountCurve
        >>> from finstack.valuations.attribution import attribute_pnl, AttributionMethod
        >>> from finstack.valuations.instruments import Bond
        >>> bond = (
        ...     Bond
        ...     .builder("CORP-001")
        ...     .money(Money(1_000_000, Currency("USD")))
        ...     .coupon_rate(0.05)
        ...     .issue(date(2025, 1, 1))
        ...     .maturity(date(2030, 1, 1))
        ...     .disc_id("USD-OIS")
        ...     .build()
        ... )
        >>> market_t0 = MarketContext()
        >>> market_t1 = MarketContext()
        >>> curve_t0 = DiscountCurve("USD-OIS", date(2025, 1, 1), [(0.0, 1.0), (1.0, 0.99)])
        >>> curve_t1 = DiscountCurve("USD-OIS", date(2025, 1, 1), [(0.0, 1.0), (1.0, 0.9895)])
        >>> market_t0.insert(curve_t0)
        >>> market_t1.insert(curve_t1)
        >>> attr = attribute_pnl(
        ...     bond,
        ...     market_t0,
        ...     market_t1,
        ...     date(2025, 1, 15),
        ...     date(2025, 1, 16),
        ...     method=AttributionMethod.parallel(),
        ... )
        >>> print(f"Total P&L: {attr.total_pnl}")
        >>> print(f"Carry: {attr.carry}")
        >>> print(f"Rates: {attr.rates_curves_pnl}")
        >>> print(f"Residual: {attr.residual} ({attr.meta.residual_pct:.2f}%)")
        >>> # Export
        >>> csv_data = attr.to_csv()
        >>> print(attr.explain())
    """
    ...

def attribute_portfolio_pnl(
    portfolio: Portfolio,
    market_t0: MarketContext,
    market_t1: MarketContext,
    as_of_t0: date,
    as_of_t1: date,
    method: AttributionMethod | None = None,
) -> PortfolioAttribution:
    """Perform P&L attribution for an entire portfolio.

    Attributes each position's P&L and aggregates to portfolio base currency
    with full factor decomposition.

    Args:
        portfolio: Portfolio to attribute
        market_t0: Market context at T₀
        market_t1: Market context at T₁
        method: Attribution methodology (defaults to Parallel)

    Returns:
        Portfolio-level attribution with position-by-position breakdown

    Raises:
        ValueError: If portfolio invalid
        RuntimeError: If attribution fails for any position

    Example:
        >>> from datetime import date
        >>> from finstack.portfolio import PortfolioBuilder
        >>> from finstack.valuations.attribution import attribute_portfolio_pnl
        >>> from finstack.core.market_data.context import MarketContext
        >>> portfolio = PortfolioBuilder("MY_FUND").base_ccy("USD").as_of(date(2025, 1, 16)).build()
        >>> market_t0 = MarketContext()
        >>> market_t1 = MarketContext()
        >>> attr = attribute_portfolio_pnl(
        ...     portfolio,
        ...     market_t0,
        ...     market_t1,
        ...     date(2025, 1, 15),
        ...     date(2025, 1, 16),
        ... )
        >>> print(f"Portfolio P&L: {attr.total_pnl}")
        >>> print(f"Total Carry: {attr.carry}")
        >>> # Position breakdown
        >>> for pos_id, pos_attr in attr.by_position_to_dict().items():
        ...     print(f"{pos_id}: {pos_attr.total_pnl}")
        >>> # Export
        >>> print(attr.to_csv())
        >>> print(attr.position_detail_to_csv())
    """
    ...

def attribute_pnl_taylor(
    instrument: Any,
    market_t0: MarketContext,
    market_t1: MarketContext,
    as_of_t0: date,
    as_of_t1: date,
    config: TaylorAttributionConfig | None = None,
) -> TaylorAttributionResult: ...
def reprice_instrument(
    instrument: Any,
    market: MarketContext,
    as_of: date,
) -> Money: ...
def convert_currency(
    money: Money,
    target_ccy: Any,
    market: MarketContext,
    as_of: date,
) -> Money: ...
def compute_pnl(
    val_t0: Money,
    val_t1: Money,
    target_ccy: Any,
    market_t1: MarketContext,
    as_of_t1: date,
) -> Money: ...
def compute_pnl_with_fx(
    val_t0: Money,
    val_t1: Money,
    target_ccy: Any,
    market_fx_t0: MarketContext,
    market_fx_t1: MarketContext,
    as_of_t0: date,
    as_of_t1: date,
) -> Money: ...
def default_waterfall_order() -> List[str]: ...
def attribute_pnl_from_json(spec_json: str) -> PnlAttribution:
    """Perform P&L attribution from a JSON specification.

    Accepts a JSON string containing a complete attribution request with
    instrument, market snapshots, dates, and methodology. This enables
    external systems to trigger attribution runs via stable JSON contracts.

    Args:
        spec_json: JSON string conforming to finstack.attribution/1 schema

    Returns:
        Complete P&L attribution with factor breakdown

    Raises:
        ValueError: If JSON is malformed or schema invalid
        RuntimeError: If attribution execution fails

    Example:
        Use this entry point when another service already has a JSON request ready.
        A minimal payload includes the schema, instrument specification, market
        snapshots, valuation dates, and methodology, for example::

            {
                "schema": "finstack.attribution/1",
                "attribution": {
                    "instrument": { ... JSON instrument spec ... },
                    "market_t0": { ... },
                    "market_t1": { ... },
                    "as_of_t0": "2025-01-15",
                    "as_of_t1": "2025-01-16",
                    "method": "Parallel"
                }
            }

        Once populated, call ``attribute_pnl_from_json(json.dumps(spec))`` to execute
        the request exactly as the REST API would.
    """
    ...

def attribution_result_to_json(attribution: PnlAttribution) -> str:
    """Serialize an attribution result to JSON.

    Wraps the attribution result in a versioned envelope for stable
    interchange with external systems.

    Args:
        attribution: P&L attribution result to serialize

    Returns:
        JSON string conforming to finstack.attribution/1 result schema

    Raises:
        RuntimeError: If serialization fails

    Example:
        >>> from finstack.valuations.attribution import attribution_result_to_json
        >>> # Assuming attr is a PnlAttribution from attribute_pnl()
        >>> # json_str = attribution_result_to_json(attr)
        >>> # Save to file or send to API
        >>> # with open("attribution_result.json", "w") as f:
        >>> #     f.write(json_str)
    """
    ...
