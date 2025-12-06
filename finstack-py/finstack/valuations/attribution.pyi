"""P&L Attribution type stubs."""

from datetime import date
from typing import Optional, Dict, List, Mapping, Any
from finstack.core.money import Money
from finstack.core.market_data.context import MarketContext
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
    def tolerance(self) -> float:
        """Tolerance for residual validation."""
        ...

class RatesCurvesAttribution:
    """Detailed attribution for interest rate curves."""

    def by_curve_to_dict(self) -> Dict[str, Money]:
        """Get P&L by curve ID.

        Returns:
            Dictionary mapping curve ID to P&L amount
        """
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

class ModelParamsAttribution:
    """Detailed attribution for model-specific parameters."""

    @property
    def prepayment(self) -> Optional[Money]:
        """Prepayment speed changes P&L (for MBS/ABS)."""
        ...

    @property
    def default_rate(self) -> Optional[Money]:
        """Default rate changes P&L (for structured credit)."""
        ...

    @property
    def recovery_rate(self) -> Optional[Money]:
        """Recovery rate changes P&L (for credit instruments)."""
        ...

    @property
    def conversion_ratio(self) -> Optional[Money]:
        """Conversion ratio changes P&L (for convertible bonds)."""
        ...

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
    def rates_detail(self) -> Optional[RatesCurvesAttribution]:
        """Detailed rates curves attribution."""
        ...

    @property
    def credit_detail(self) -> Optional[CreditCurvesAttribution]:
        """Detailed credit curves attribution."""
        ...

    @property
    def model_params_detail(self) -> Optional[ModelParamsAttribution]:
        """Detailed model parameters attribution."""
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

    def rates_detail_to_csv(self) -> Optional[str]:
        """Export rates curves detail as CSV.

        Returns:
            CSV string with curve-by-curve breakdown, or None if no detail available
        """
        ...

    def credit_detail_to_csv(self) -> Optional[str]:
        """Export credit curves detail as CSV.

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
    method: Optional[AttributionMethod] = None,
    model_params_t0: Optional[Mapping[str, Any] | str] = None,
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
        >>> bond = Bond.fixed_semiannual(
        ...     "CORP-001", Money(1_000_000, Currency("USD")), 0.05, date(2025, 1, 1), date(2030, 1, 1), "USD-OIS"
        ... )
        >>> market_t0 = MarketContext()
        >>> market_t1 = MarketContext()
        >>> curve_t0 = DiscountCurve("USD-OIS", date(2025, 1, 1), [(0.0, 1.0), (1.0, 0.99)])
        >>> curve_t1 = DiscountCurve("USD-OIS", date(2025, 1, 1), [(0.0, 1.0), (1.0, 0.9895)])
        >>> market_t0.insert_discount(curve_t0)
        >>> market_t1.insert_discount(curve_t1)
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
    method: Optional[AttributionMethod] = None,
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
