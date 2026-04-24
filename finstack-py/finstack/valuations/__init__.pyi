"""Instrument pricing, risk metrics, and P&L attribution."""

from __future__ import annotations

import pandas as pd

from finstack.core.market_data import MarketContext
from finstack.valuations import correlation as correlation
from finstack.valuations import instruments as instruments

__all__ = [
    "correlation",
    "instruments",
    "ValuationResult",
    "validate_instrument_json",
    "price_instrument",
    "price_instrument_with_metrics",
    "list_standard_metrics",
    "list_standard_metrics_grouped",
    "PnlAttribution",
    "attribute_pnl",
    "attribute_pnl_from_spec",
    "validate_attribution_json",
    "default_waterfall_order",
    "default_attribution_metrics",
    "SensitivityMatrix",
    "FactorPnlProfile",
    "compute_factor_sensitivities",
    "compute_pnl_profiles",
    "RiskDecomposition",
    "decompose_factor_risk",
    "CalibrationResult",
    "validate_calibration_json",
    "calibrate",
    "bs_cos_price",
    "vg_cos_price",
    "merton_jump_cos_price",
    "tarn_coupon_profile",
    "snowball_coupon_profile",
    "cms_spread_option_intrinsic",
    "callable_range_accrual_accrued",
    "bs_price",
    "bs_greeks",
    "bs_implied_vol",
    "black76_implied_vol",
    "SabrParameters",
    "SabrModel",
    "SabrSmile",
    "SabrCalibrator",
    "instrument_cashflows",
    "instrument_cashflows_json",
]

class ValuationResult:
    """Valuation envelope: PV, currency, risk metrics, covenant flags, and JSON round-trip.

    Instantiate via :meth:`from_json` or the ``price_*`` helpers that emit JSON.

    Args:
        None (use ``from_json``).

    Returns:
        A ``ValuationResult`` instance (type description only).

    Example:
        >>> from finstack.valuations import ValuationResult
        >>> ValuationResult.from_json(result_json)  # doctest: +SKIP
    """

    @staticmethod
    def from_json(json: str) -> ValuationResult:
        """Deserialize a ``ValuationResult`` from JSON.

        Args:
            json: JSON string produced by the pricing pipeline or ``to_json``.

        Returns:
            Parsed ``ValuationResult`` instance.

        Example:
            >>> from finstack.valuations import ValuationResult
            >>> ValuationResult.from_json('{"instrument_id":"x","value":{}}')  # doctest: +SKIP
        """
        ...

    def to_json(self) -> str:
        """Serialize this result to pretty-printed JSON.

        Args:
            (none)

        Returns:
            Pretty-printed JSON string.

        Example:
            >>> ValuationResult.from_json(
            ...     '{"instrument_id":"i","value":{"amount":1.0,"currency":"USD"},"measures":{}}'
            ... ).to_json()  # doctest: +SKIP
            ''
        """
        ...

    @property
    def instrument_id(self) -> str:
        """Instrument identifier assigned by the pricer.

        Args:
            None (read-only property).

        Returns:
            Instrument ID string.

        Example:
            >>> vr = ValuationResult.from_json("{}")  # doctest: +SKIP
            >>> vr.instrument_id  # doctest: +SKIP
            ''
        """
        ...

    @property
    def price(self) -> float:
        """Present value amount (NPV).

        Args:
            None (read-only property).

        Returns:
            PV amount as a float.

        Example:
            >>> vr = ValuationResult.from_json("{}")  # doctest: +SKIP
            >>> vr.price  # doctest: +SKIP
            0.0
        """
        ...

    @property
    def currency(self) -> str:
        """Currency code for the present value.

        Args:
            None (read-only property).

        Returns:
            Currency code string.

        Example:
            >>> vr = ValuationResult.from_json("{}")  # doctest: +SKIP
            >>> vr.currency  # doctest: +SKIP
            'USD'
        """
        ...

    def get_metric(self, key: str) -> float | None:
        """Return a scalar risk measure by string key.

        Args:
            key: Metric identifier (e.g. ``"ytm"``, ``"dv01"``).

        Returns:
            Metric value, or ``None`` if missing.

        Example:
            >>> vr = ValuationResult.from_json(
            ...     '{"instrument_id":"i","value":{"amount":1,"currency":"USD"},"measures":{}}'
            ... )  # doctest: +SKIP
            >>> vr.get_metric("ytm")  # doctest: +SKIP
        """
        ...

    def metric_keys(self) -> list[str]:
        """List metric keys present on this result.

        Args:
            (none)

        Returns:
            All measure keys as strings.

        Example:
            >>> ValuationResult.from_json(
            ...     '{"instrument_id":"i","value":{"amount":1,"currency":"USD"},"measures":{}}'
            ... ).metric_keys()  # doctest: +SKIP
            []
        """
        ...

    def metric_count(self) -> int:
        """Count of measures stored on this result.

        Args:
            (none)

        Returns:
            Number of entries in the measures map.

        Example:
            >>> ValuationResult.from_json(
            ...     '{"instrument_id":"i","value":{"amount":1,"currency":"USD"},"measures":{}}'
            ... ).metric_count()  # doctest: +SKIP
            0
        """
        ...

    def all_covenants_passed(self) -> bool:
        """Whether every covenant passed (or none were evaluated).

        Args:
            (none)

        Returns:
            ``True`` if no covenant failures are recorded.

        Example:
            >>> ValuationResult.from_json(
            ...     '{"instrument_id":"i","value":{"amount":1,"currency":"USD"},"measures":{}}'
            ... ).all_covenants_passed()  # doctest: +SKIP
            True
        """
        ...

    def failed_covenants(self) -> list[str]:
        """Covenant IDs that failed, if any.

        Args:
            (none)

        Returns:
            List of failed covenant identifiers.

        Example:
            >>> ValuationResult.from_json(
            ...     '{"instrument_id":"i","value":{"amount":1,"currency":"USD"},"measures":{}}'
            ... ).failed_covenants()  # doctest: +SKIP
            []
        """
        ...

    def metrics_to_dataframe(self) -> pd.DataFrame:
        """Export as a single-row pandas DataFrame.

        Columns include ``instrument_id``, ``price``, ``currency``, plus one
        column per metric key.  Useful for stacking multiple results with
        ``pd.concat``.

        Returns:
            Single-row DataFrame.
        """
        ...

    def __repr__(self) -> str:
        """Return a concise debug string for this result.

        Args:
            None (uses ``self``).

        Returns:
            ``ValuationResult(id=..., price=..., currency=..., metrics=...)`` text.

        Example:
            >>> repr(ValuationResult.from_json("{}"))  # doctest: +SKIP
            ''
        """
        ...

def validate_instrument_json(json: str) -> str:
    """Parse tagged instrument JSON and return canonical pretty JSON.

    Args:
        json: Tagged instrument JSON (e.g. ``{"type": "bond", ...}``).

    Returns:
        Canonical pretty-printed JSON accepted by the instrument loader.

    Example:
        >>> from finstack.valuations import validate_instrument_json
        >>> validate_instrument_json(inst_json)  # doctest: +SKIP
        ''
    """
    ...

def price_instrument(
    instrument_json: str,
    market_json: str,
    as_of: str,
    model: str = "discounting",
) -> str:
    """Price an instrument using the standard registry and a model key.

    Args:
        instrument_json: Tagged instrument JSON.
        market_json: JSON-serialized ``MarketContext``.
        as_of: Valuation date in ISO 8601 format.
        model: Model key: ``discounting`` (default), ``black76``, ``hazard_rate``,
            ``hull_white_1f``, ``tree``, ``normal``, ``monte_carlo_gbm``, etc.

    Returns:
        Pretty-printed JSON ``ValuationResult``.

    Example:
        >>> from finstack.valuations import price_instrument
        >>> price_instrument(inst_json, mkt_json, "2025-01-15")  # doctest: +SKIP
        ''
    """
    ...

def price_instrument_with_metrics(
    instrument_json: str,
    market_json: str,
    as_of: str,
    model: str = "discounting",
    metrics: list[str] = [],
    pricing_options: str | None = None,
) -> str:
    """Price an instrument and request explicit risk metrics.

    Args:
        instrument_json: Tagged instrument JSON.
        market_json: JSON-serialized ``MarketContext``.
        as_of: Valuation date in ISO 8601 format.
        model: Model key string (same vocabulary as ``price_instrument``).
        metrics: Metric names to compute (default empty list).
        pricing_options: Optional JSON string of ``MetricPricingOverrides``
            merged into the instrument's ``pricing_overrides``. Supports
            ``"theta_period"`` (e.g. ``"6M"``) and ``"breakeven_config"``
            (e.g. ``{"target": "z_spread", "mode": "linear"}``).

    Returns:
        Pretty-printed JSON ``ValuationResult`` including requested metrics.

    Example:
        >>> from finstack.valuations import price_instrument_with_metrics
        >>> price_instrument_with_metrics(inst_json, mkt_json, "2025-01-15", metrics=["dv01"])  # doctest: +SKIP
        ''
    """
    ...

def instrument_cashflows_json(
    instrument_json: str,
    market: MarketContext | str,
    as_of: str,
    model: str = "discounting",
) -> str:
    """Per-flow cashflow envelope (DF / survival / PV) for a discountable instrument.

    Supports ``model in {"discounting", "hazard_rate"}``. The envelope's
    ``total_pv`` reconciles with the instrument's ``base_value`` for the
    supported model-instrument pairs.

    Args:
        instrument_json: Tagged instrument JSON.
        market: ``MarketContext`` instance or JSON string.
        as_of: ISO 8601 valuation date.
        model: ``"discounting"`` (DF only) or ``"hazard_rate"`` (adds
            survival probability, conditional default probability, and
            recovery-adjusted principal PV).

    Returns:
        JSON-serialized ``InstrumentCashflowEnvelope``.

    Raises:
        ValueError: If ``model`` is unsupported or the instrument type isn't
            priced under that model.
    """
    ...

def instrument_cashflows(
    instrument_json: str,
    market: MarketContext | str,
    as_of: str,
    *,
    model: str = "discounting",
) -> tuple[dict, pd.DataFrame]:
    """DataFrame-friendly wrapper around :func:`instrument_cashflows_json`.

    Parses the JSON envelope returned by the low-level binding and constructs
    a per-flow ``pandas.DataFrame`` with ``date`` / ``reset_date`` parsed as
    ``datetime64``. See :func:`instrument_cashflows_json` for argument and
    error semantics.

    Returns:
        ``(envelope, df)`` where ``envelope`` is the parsed dict and ``df``
        carries one row per flow with columns ``date``, ``amount``,
        ``currency``, ``kind``, ``accrual_factor``, ``year_fraction``,
        ``rate``, ``reset_date``, ``discount_factor``, ``survival_probability``,
        ``conditional_default_prob``, ``inflation_index_ratio``,
        ``prepayment_smm``, ``beginning_balance``, ``ending_balance``, and
        ``pv``.
    """
    ...

def list_standard_metrics() -> list[str]:
    """Return every metric ID exposed by the standard metric registry.

    Args:
        (none)

    Returns:
        Sorted metric identifier strings.

    Example:
        >>> from finstack.valuations import list_standard_metrics
        >>> isinstance(list_standard_metrics(), list)
        True
    """
    ...

def list_standard_metrics_grouped() -> dict[str, list[str]]:
    """Return standard metrics organized by group.

    Each key is a human-readable group name (e.g. ``"Pricing"``,
    ``"Greeks"``, ``"Sensitivity"``).  Values are sorted lists of
    metric identifier strings belonging to that group.

    Returns:
        Mapping from group name to metric identifiers.

    Example:
        >>> from finstack.valuations import list_standard_metrics_grouped
        >>> grouped = list_standard_metrics_grouped()
        >>> "Greeks" in grouped
        True
        >>> "delta" in grouped["Greeks"]
        True
    """
    ...

# ---------------------------------------------------------------------------
# P&L Attribution
# ---------------------------------------------------------------------------

class PnlAttribution:
    """P&L attribution result decomposing total P&L into risk factor contributions.

    Factors include carry, rates curves, credit curves, inflation, correlations,
    FX, volatility, cross-factor interactions, model parameters, market scalars,
    and residual.

    Construct via :meth:`from_json` or the :func:`attribute_pnl` helper.

    Example:
        >>> from finstack.valuations import PnlAttribution
        >>> attr = PnlAttribution.from_json(result_json)  # doctest: +SKIP
    """

    @staticmethod
    def from_json(json: str) -> PnlAttribution:
        """Deserialize a ``PnlAttribution`` from JSON.

        Args:
            json: JSON string (the ``attribution`` field from an
                ``AttributionResultEnvelope``).

        Returns:
            Parsed ``PnlAttribution`` instance.
        """
        ...

    def to_json(self) -> str:
        """Serialize to pretty-printed JSON.

        Returns:
            Pretty-printed JSON string.
        """
        ...

    @property
    def total_pnl(self) -> float:
        """Total P&L amount (val_t1 − val_t0)."""
        ...

    @property
    def carry(self) -> float:
        """Carry (theta + accruals) P&L amount."""
        ...

    @property
    def rates_curves_pnl(self) -> float:
        """Interest rate curves P&L amount."""
        ...

    @property
    def credit_curves_pnl(self) -> float:
        """Credit hazard curves P&L amount."""
        ...

    @property
    def inflation_curves_pnl(self) -> float:
        """Inflation curves P&L amount."""
        ...

    @property
    def correlations_pnl(self) -> float:
        """Base correlation curves P&L amount."""
        ...

    @property
    def fx_pnl(self) -> float:
        """FX rate changes P&L amount."""
        ...

    @property
    def vol_pnl(self) -> float:
        """Implied volatility changes P&L amount."""
        ...

    @property
    def cross_factor_pnl(self) -> float:
        """Cross-factor interaction P&L amount."""
        ...

    @property
    def model_params_pnl(self) -> float:
        """Model parameters P&L amount."""
        ...

    @property
    def market_scalars_pnl(self) -> float:
        """Market scalars P&L amount."""
        ...

    @property
    def residual(self) -> float:
        """Residual (unexplained) P&L amount."""
        ...

    @property
    def currency(self) -> str:
        """Currency code for all P&L amounts."""
        ...

    @property
    def instrument_id(self) -> str:
        """Instrument identifier."""
        ...

    @property
    def method(self) -> str:
        """Attribution method name (Parallel, Waterfall, MetricsBased, Taylor)."""
        ...

    @property
    def t0(self) -> str:
        """Start date (T₀) as ISO string."""
        ...

    @property
    def t1(self) -> str:
        """End date (T₁) as ISO string."""
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
    def notes(self) -> list[str]:
        """Diagnostic notes and warnings."""
        ...

    def residual_within_tolerance(self, pct_tolerance: float = 0.1, abs_tolerance: float = 1.0) -> bool:
        """Check if residual is within tolerance.

        Args:
            pct_tolerance: Percentage tolerance (e.g. 0.1 for 0.1%).
            abs_tolerance: Absolute tolerance (e.g. 100.0 for $100).

        Returns:
            ``True`` if residual is within tolerance.
        """
        ...

    def explain(self) -> str:
        """Human-readable tree explanation (non-zero factors only).

        Returns:
            Multi-line string with tree structure showing P&L breakdown.
        """
        ...

    def explain_verbose(self) -> str:
        """Verbose tree explanation including zero-valued factors.

        Returns:
            Multi-line string with tree structure showing all factors.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """Export attribution as a single-row pandas DataFrame.

        Columns include ``instrument_id``, ``method``, ``t0``, ``t1``,
        ``currency``, ``total_pnl``, all factor P&L amounts, ``residual``,
        ``residual_pct``, and ``num_repricings``.

        Returns:
            Single-row DataFrame.
        """
        ...

    def __repr__(self) -> str: ...

def attribute_pnl(
    instrument_json: str,
    market_t0_json: str,
    market_t1_json: str,
    as_of_t0: str,
    as_of_t1: str,
    method: str | dict,
    config: dict | None = None,
) -> str:
    """Run P&L attribution for a single instrument.

    This is the main entry point. Accepts the instrument, two market
    snapshots, valuation dates, and a method descriptor and returns the
    canonical JSON form of the attribution. Use
    ``PnlAttribution.from_json(...)`` when you want the richer wrapper.

    Args:
        instrument_json: Tagged instrument JSON (``{"type": "bond", ...}``).
        market_t0_json: JSON-serialized ``MarketContext`` at T₀.
        market_t1_json: JSON-serialized ``MarketContext`` at T₁.
        as_of_t0: Valuation date T₀ in ISO 8601 format.
        as_of_t1: Valuation date T₁ in ISO 8601 format.
        method: Attribution method — one of ``"Parallel"``,
            ``{"Waterfall": ["Carry", "RatesCurves", ...]}``,
            ``"MetricsBased"``, or ``{"Taylor": {"include_gamma": True, ...}}``.
        config: Optional config overrides (tolerance, metrics, bump sizes).

    Returns:
        Pretty-printed JSON ``PnlAttribution`` payload.

    Example:
        >>> attr_json = attribute_pnl(inst, mkt_t0, mkt_t1, "2025-01-15", "2025-01-16", "Parallel")
        >>> attr = PnlAttribution.from_json(attr_json)  # doctest: +SKIP
        >>> print(attr.explain())  # doctest: +SKIP
    """
    ...

def attribute_pnl_from_spec(spec_json: str) -> str:
    """Run attribution from a full JSON ``AttributionEnvelope``.

    Power-user variant for full envelope round-trip workflows.
    Most users should prefer :func:`attribute_pnl`.

    Args:
        spec_json: JSON-serialized ``AttributionEnvelope``.

    Returns:
        JSON-serialized ``AttributionResultEnvelope``.
    """
    ...

def validate_attribution_json(json: str) -> str:
    """Validate an attribution specification JSON.

    Deserializes against the ``AttributionEnvelope`` schema and returns
    the canonical (re-serialized) JSON.

    Args:
        json: JSON-serialized ``AttributionEnvelope``.

    Returns:
        Canonical pretty-printed JSON.
    """
    ...

def default_waterfall_order() -> list[str]:
    """Return the default waterfall factor ordering.

    Returns:
        Factor names in the default waterfall order.

    Example:
        >>> from finstack.valuations import default_waterfall_order
        >>> default_waterfall_order()  # doctest: +SKIP
        ['Carry', 'RatesCurves', 'CreditCurves', ...]
    """
    ...

def default_attribution_metrics() -> list[str]:
    """Return the default metric IDs used by metrics-based attribution.

    Returns:
        Metric identifier strings.

    Example:
        >>> from finstack.valuations import default_attribution_metrics
        >>> default_attribution_metrics()  # doctest: +SKIP
        ['theta', 'dv01', 'cs01', ...]
    """
    ...

# ---------------------------------------------------------------------------
# Factor Sensitivity
# ---------------------------------------------------------------------------

class SensitivityMatrix:
    """Positions-by-factors sensitivity matrix.

    Each element ``(i, j)`` is the first-order sensitivity of position *i* to
    factor *j*, denominated in the factor's bump units (e.g. PV change per 1 bp
    for a rates factor).

    Construct via :func:`compute_factor_sensitivities`.

    Example:
        >>> from finstack.valuations import compute_factor_sensitivities
        >>> matrix = compute_factor_sensitivities(pos_json, fac_json, mkt_json, "2025-01-15")  # doctest: +SKIP
    """

    @property
    def position_ids(self) -> list[str]:
        """Ordered position identifiers (row axis)."""
        ...

    @property
    def factor_ids(self) -> list[str]:
        """Ordered factor identifiers (column axis)."""
        ...

    @property
    def n_positions(self) -> int:
        """Number of positions (rows)."""
        ...

    @property
    def n_factors(self) -> int:
        """Number of factors (columns)."""
        ...

    def delta(self, position_idx: int, factor_idx: int) -> float:
        """Read a single sensitivity element.

        Args:
            position_idx: Row index.
            factor_idx: Column index.

        Returns:
            Sensitivity value.
        """
        ...

    def position_deltas(self, position_idx: int) -> list[float]:
        """Sensitivity row for a single position across all factors.

        Args:
            position_idx: Row index.

        Returns:
            List of delta values, one per factor.
        """
        ...

    def factor_deltas(self, factor_idx: int) -> list[float]:
        """Sensitivity column for a single factor across all positions.

        Args:
            factor_idx: Column index.

        Returns:
            List of delta values, one per position.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """Export as a pandas DataFrame with positions as rows and factors as columns.

        Returns:
            DataFrame indexed by position IDs with factor IDs as column names.
        """
        ...

    def __repr__(self) -> str: ...

class FactorPnlProfile:
    """P&L profile for one factor across a scenario grid.

    Each profile captures the hypothetical P&L for every position at each
    scenario shift, enabling non-linear (gamma, convexity) analysis.

    Construct via :func:`compute_pnl_profiles`.

    Example:
        >>> from finstack.valuations import compute_pnl_profiles
        >>> profiles = compute_pnl_profiles(pos_json, fac_json, mkt_json, "2025-01-15")  # doctest: +SKIP
    """

    @property
    def factor_id(self) -> str:
        """Factor identifier."""
        ...

    @property
    def shifts(self) -> list[float]:
        """Scenario shift coordinates (bump-size multiples)."""
        ...

    @property
    def position_pnls(self) -> list[list[float]]:
        """Per-shift P&L vectors indexed as ``[shift_idx][position_idx]``."""
        ...

    def to_dataframe(self, position_ids: list[str]) -> pd.DataFrame:
        """Export as a pandas DataFrame with shifts as rows and positions as columns.

        Args:
            position_ids: Position identifiers to use as column names.  Must
                match the number of positions in the profile.

        Returns:
            DataFrame indexed by shift values with position IDs as column names.

        Raises:
            ValueError: If ``len(position_ids)`` does not match the profile width.
        """
        ...

    def __repr__(self) -> str: ...

def compute_factor_sensitivities(
    positions_json: str,
    factors_json: str,
    market_json: str,
    as_of: str,
    bump_config_json: str | None = None,
) -> SensitivityMatrix:
    """Compute first-order factor sensitivities using central finite differences.

    Args:
        positions_json: JSON array of position objects, each with ``id`` (str),
            ``instrument`` (tagged instrument JSON), and ``weight`` (float).
        factors_json: JSON array of ``FactorDefinition`` objects.
        market_json: JSON-serialized ``MarketContext``.
        as_of: Valuation date in ISO 8601 format.
        bump_config_json: Optional JSON-serialized ``BumpSizeConfig``.
            Defaults to 1 bp / 1 % per factor type.

    Returns:
        Positions-by-factors delta matrix.

    Example:
        >>> from finstack.valuations import compute_factor_sensitivities
        >>> matrix = compute_factor_sensitivities(pos_json, fac_json, mkt_json, "2025-01-15")  # doctest: +SKIP
        >>> matrix.to_dataframe()  # doctest: +SKIP
    """
    ...

def compute_pnl_profiles(
    positions_json: str,
    factors_json: str,
    market_json: str,
    as_of: str,
    bump_config_json: str | None = None,
    n_scenario_points: int = 5,
) -> list[FactorPnlProfile]:
    """Compute scenario P&L profiles via full repricing across a factor grid.

    Args:
        positions_json: JSON array of position objects (same schema as
            :func:`compute_factor_sensitivities`).
        factors_json: JSON array of ``FactorDefinition`` objects.
        market_json: JSON-serialized ``MarketContext``.
        as_of: Valuation date in ISO 8601 format.
        bump_config_json: Optional JSON-serialized ``BumpSizeConfig``.
        n_scenario_points: Number of scenario grid points
            (default 5 produces shifts ``[-2, -1, 0, 1, 2]``).

    Returns:
        One profile per factor, each containing scenario P&L for every position.

    Example:
        >>> from finstack.valuations import compute_pnl_profiles
        >>> profiles = compute_pnl_profiles(pos_json, fac_json, mkt_json, "2025-01-15")  # doctest: +SKIP
        >>> profiles[0].to_dataframe(["bond_1", "equity_1"])  # doctest: +SKIP
    """
    ...

# ---------------------------------------------------------------------------
# Risk Decomposition
# ---------------------------------------------------------------------------

class RiskDecomposition:
    """Portfolio-level decomposition of total risk across factors and positions.

    Obtain via :func:`decompose_factor_risk`.  The decomposition expresses
    forecasted portfolio risk (variance, volatility, VaR, or ES) as a sum of
    Euler-allocated factor-level contributions, each drillable to per-position
    detail.

    Example:
        >>> from finstack.valuations import decompose_factor_risk  # doctest: +SKIP
        >>> result = decompose_factor_risk(sens, cov_json)  # doctest: +SKIP
        >>> result.total_risk  # doctest: +SKIP
        0.042
    """

    @property
    def total_risk(self) -> float:
        """Total portfolio risk under the selected measure."""
        ...

    @property
    def measure(self) -> str:
        """Risk measure used (e.g. ``"Variance"``, ``"Volatility"``)."""
        ...

    @property
    def residual_risk(self) -> float:
        """Residual (idiosyncratic) risk not attributed to any factor."""
        ...

    def factor_contributions(self) -> list[dict[str, object]]:
        """Factor-level contributions as a list of dicts.

        Each dict contains ``factor_id``, ``absolute_risk``, ``relative_risk``,
        and ``marginal_risk``.

        Returns:
            List of per-factor contribution dicts.
        """
        ...

    def position_factor_contributions(self) -> list[dict[str, object]]:
        """Position x factor contributions as a list of dicts.

        Each dict contains ``position_id``, ``factor_id``, and
        ``risk_contribution``.

        Returns:
            List of per-position, per-factor contribution dicts.
        """
        ...

    def to_factor_dataframe(self) -> pd.DataFrame:
        """Export factor contributions as a pandas DataFrame.

        Columns: ``factor_id``, ``absolute_risk``, ``relative_risk``,
        ``marginal_risk``.

        Returns:
            DataFrame with one row per factor.
        """
        ...

    def to_position_factor_dataframe(self) -> pd.DataFrame:
        """Export position x factor contributions as a pandas DataFrame.

        Columns: ``position_id``, ``factor_id``, ``risk_contribution``.

        Returns:
            DataFrame with one row per position-factor pair.
        """
        ...

    def __repr__(self) -> str: ...

def decompose_factor_risk(
    sensitivities: SensitivityMatrix,
    covariance_json: str,
    risk_measure: str | dict | None = None,
) -> RiskDecomposition:
    """Decompose portfolio risk into factor and position contributions.

    Uses the parametric (covariance-based) Euler decomposition to attribute
    forecasted portfolio risk across factors and individual positions.

    Args:
        sensitivities: Weighted position x factor sensitivity matrix, as
            returned by :func:`compute_factor_sensitivities`.
        covariance_json: JSON-serialized ``FactorCovarianceMatrix``.  Must use
            the same factor IDs and ordering as the sensitivity matrix.
        risk_measure: Risk measure.  Defaults to ``"variance"``.
            Accepts Python strings (``"variance"``, ``"volatility"``) or dicts
            (``{"var": {"confidence": 0.99}}``,
            ``{"expected_shortfall": {"confidence": 0.975}}``).

    Returns:
        Portfolio-level risk decomposition with factor and position detail.

    Raises:
        ValueError: If factor axes do not match or the covariance matrix is
            invalid.

    Example:
        >>> from finstack.valuations import compute_factor_sensitivities, decompose_factor_risk
        >>> sens = compute_factor_sensitivities(pos, fac, mkt, "2025-01-15")  # doctest: +SKIP
        >>> result = decompose_factor_risk(sens, cov_json, "volatility")  # doctest: +SKIP
        >>> result.to_factor_dataframe()  # doctest: +SKIP
    """
    ...

# ---------------------------------------------------------------------------
# Calibration
# ---------------------------------------------------------------------------

class CalibrationResult:
    """Result of a calibration plan execution.

    Provides access to the calibrated market context, per-step reports,
    and overall success status.  Construct via :func:`calibrate` or
    :meth:`from_json`.

    Example:
        >>> import json
        >>> from finstack.valuations import calibrate
        >>> result = calibrate(json.dumps(plan))  # doctest: +SKIP
        >>> result.success  # doctest: +SKIP
        True
    """

    @staticmethod
    def from_json(json: str) -> CalibrationResult:
        """Deserialize a ``CalibrationResult`` from JSON.

        Args:
            json: JSON string (a ``CalibrationResultEnvelope``).

        Returns:
            Parsed ``CalibrationResult`` instance.
        """
        ...

    def to_json(self) -> str:
        """Serialize to pretty-printed JSON.

        Returns:
            Pretty-printed JSON string.
        """
        ...

    @property
    def success(self) -> bool:
        """Whether the overall calibration succeeded (all steps passed)."""
        ...

    @property
    def market(self) -> MarketContext:
        """The calibrated ``MarketContext`` containing all produced curves."""
        ...

    @property
    def market_json(self) -> str:
        """The calibrated market serialized as a JSON string."""
        ...

    @property
    def report_json(self) -> str:
        """The aggregated calibration report as a JSON string."""
        ...

    @property
    def step_ids(self) -> list[str]:
        """List of step identifiers that were executed."""
        ...

    @property
    def iterations(self) -> int:
        """Total solver iterations across all steps."""
        ...

    @property
    def max_residual(self) -> float:
        """Maximum absolute residual across all steps."""
        ...

    @property
    def rmse(self) -> float:
        """Root mean square error across all steps."""
        ...

    def step_report_json(self, step_id: str) -> str:
        """Per-step calibration report as a JSON string.

        Args:
            step_id: Identifier of the calibration step.

        Returns:
            JSON-serialized calibration report for the step.

        Raises:
            ValueError: If no step with the given *step_id* exists.
        """
        ...

    def report_to_dataframe(self) -> pd.DataFrame:
        """Per-step summary as a pandas DataFrame.

        Columns: ``step_id``, ``success``, ``iterations``, ``max_residual``,
        ``rmse``, ``convergence_reason``.

        Returns:
            DataFrame with one row per calibration step.
        """
        ...

    def __repr__(self) -> str: ...

def validate_calibration_json(json: str) -> str:
    """Validate a calibration plan JSON and return canonical pretty-printed form.

    Args:
        json: JSON-serialized ``CalibrationEnvelope``.

    Returns:
        Canonical pretty-printed JSON.

    Raises:
        ValueError: If the JSON is not a valid calibration envelope.

    Example:
        >>> from finstack.valuations import validate_calibration_json
        >>> validate_calibration_json(plan_json)  # doctest: +SKIP
        ''
    """
    ...

def calibrate(json: str) -> CalibrationResult:
    """Execute a calibration plan and return the full result.

    Accepts a JSON-serialized ``CalibrationEnvelope`` containing the plan,
    quote sets, and optional initial market state.

    Args:
        json: JSON-serialized ``CalibrationEnvelope``.

    Returns:
        The calibration result with calibrated market, reports, and diagnostics.

    Raises:
        ValueError: If the JSON is invalid or calibration fails.

    Example:
        >>> import json as _json
        >>> from finstack.valuations import calibrate
        >>> result = calibrate(_json.dumps(plan))  # doctest: +SKIP
        >>> result.success  # doctest: +SKIP
        True
        >>> curve = result.market.get_discount("USD-OIS")  # doctest: +SKIP
    """
    ...

# ---------------------------------------------------------------------------
# Closed-form analytic primitives (Black-Scholes / Black-76)
# ---------------------------------------------------------------------------

def bs_price(
    spot: float,
    strike: float,
    r: float,
    q: float,
    sigma: float,
    t: float,
    is_call: bool,
) -> float:
    """Per-unit Black-Scholes / Garman-Kohlhagen price of a European option.

    All rates are continuously compounded decimals; ``sigma`` is annualized
    vol; ``t`` is years to expiry. Pass ``is_call=False`` for puts.
    """
    ...

def bs_greeks(
    spot: float,
    strike: float,
    r: float,
    q: float,
    sigma: float,
    t: float,
    is_call: bool,
    theta_days: float = 365.0,
) -> dict[str, float]:
    """Black-Scholes / Garman-Kohlhagen Greeks as a dict.

    Returns ``{"delta", "gamma", "vega", "theta", "rho", "rho_q"}``. ``vega``
    and both rho values are per 1% move; ``theta`` is per-day using the
    ``theta_days`` day-count denominator (ACT/365 by default).
    """
    ...

def bs_implied_vol(
    spot: float,
    strike: float,
    r: float,
    q: float,
    t: float,
    price: float,
    is_call: bool,
) -> float:
    """Solve for Black-Scholes implied volatility given a target price."""
    ...

def black76_implied_vol(
    forward: float,
    strike: float,
    df: float,
    t: float,
    price: float,
    is_call: bool,
) -> float:
    """Solve for Black-76 (forward-based) implied volatility given a target price."""
    ...

# ---------------------------------------------------------------------------
# SABR volatility smile
# ---------------------------------------------------------------------------

class SabrParameters:
    """SABR parameters ``(alpha, beta, nu, rho)`` with optional ``shift``.

    Enforces ``alpha > 0``, ``beta in [0, 1]``, ``nu >= 0``, ``rho in
    [-1, 1]``, and ``shift > 0`` when supplied.
    """

    def __init__(
        self,
        alpha: float,
        beta: float,
        nu: float,
        rho: float,
        shift: float | None = None,
    ) -> None: ...
    @staticmethod
    def equity_default() -> SabrParameters:
        """Equity-standard defaults ``(alpha=0.20, beta=1.0, nu=0.30, rho=-0.20)``."""
        ...

    @staticmethod
    def rates_default() -> SabrParameters:
        """Rates-standard defaults ``(alpha=0.02, beta=0.5, nu=0.30, rho=0.0)``."""
        ...

    @property
    def alpha(self) -> float: ...
    @property
    def beta(self) -> float: ...
    @property
    def nu(self) -> float: ...
    @property
    def rho(self) -> float: ...
    @property
    def shift(self) -> float | None: ...
    def is_shifted(self) -> bool:
        """``True`` when parameters include a non-zero shift (negative-rate support)."""
        ...

class SabrModel:
    """Hagan-2002 SABR volatility model."""

    def __init__(self, params: SabrParameters) -> None: ...
    def implied_vol(self, forward: float, strike: float, t: float) -> float:
        """Black-style implied volatility under the Hagan-2002 expansion."""
        ...

    @property
    def params(self) -> SabrParameters: ...
    def supports_negative_rates(self) -> bool: ...

class SabrSmile:
    """Volatility smile generator for a fixed ``(forward, t)`` pair."""

    def __init__(
        self,
        params: SabrParameters,
        forward: float,
        t: float,
    ) -> None: ...
    def atm_vol(self) -> float: ...
    def implied_vol(self, strike: float) -> float: ...
    def generate_smile(self, strikes: list[float]) -> list[float]: ...
    def arbitrage_diagnostics(
        self,
        strikes: list[float],
        r: float = 0.0,
        q: float = 0.0,
    ) -> dict:
        """Butterfly + monotonicity arbitrage diagnostics on ``strikes``.

        Returns a dict with ``arbitrage_free``, ``butterfly_violations``,
        and ``monotonicity_violations``.
        """
        ...

class SabrCalibrator:
    """SABR calibrator (Levenberg-Marquardt with beta fixed)."""

    def __init__(self) -> None: ...
    @staticmethod
    def high_precision() -> SabrCalibrator:
        """Tighter tolerance and higher iteration cap for production fits."""
        ...

    def with_tolerance(self, tolerance: float) -> SabrCalibrator: ...
    def calibrate(
        self,
        forward: float,
        strikes: list[float],
        market_vols: list[float],
        t: float,
        beta: float = 1.0,
    ) -> SabrParameters:
        """Fit ``(alpha, nu, rho)`` to market vols with ``beta`` fixed."""
        ...

# ---------------------------------------------------------------------------
# Fourier option pricing helpers
# ---------------------------------------------------------------------------

def bs_cos_price(
    spot: float,
    strike: float,
    rate: float,
    dividend: float,
    vol: float,
    maturity: float,
    is_call: bool,
    n_terms: int = 128,
) -> float:
    """Price a European option under Black-Scholes with the COS method."""
    ...

def vg_cos_price(
    spot: float,
    strike: float,
    rate: float,
    dividend: float,
    sigma: float,
    theta: float,
    nu: float,
    maturity: float,
    is_call: bool,
    n_terms: int = 128,
) -> float:
    """Price a European option under Variance Gamma with the COS method."""
    ...

def merton_jump_cos_price(
    spot: float,
    strike: float,
    rate: float,
    dividend: float,
    sigma: float,
    mu_jump: float,
    sigma_jump: float,
    lambda_: float,
    maturity: float,
    is_call: bool,
    n_terms: int = 128,
) -> float:
    """Price a European option under Merton jump-diffusion with the COS method."""
    ...

# ---------------------------------------------------------------------------
# Exotic rate products — deterministic coupon / payoff helpers
# ---------------------------------------------------------------------------

def tarn_coupon_profile(
    fixed_rate: float,
    coupon_floor: float,
    floating_fixings: list[float],
    target_coupon: float,
    day_count_fraction: float,
) -> dict:
    """Simulate a TARN coupon profile along a deterministic rate path.

    Each period coupon is ``max(fixed_rate - L_i, coupon_floor) * dcf``;
    payments accumulate until the cumulative reaches ``target_coupon``, at
    which point the final coupon is capped so the cumulative hits the
    target exactly and the note redeems early.

    Args:
        fixed_rate: Fixed strike rate.
        coupon_floor: Per-period floor on ``fixed_rate - L_i``.
        floating_fixings: Floating rate fixings (one per period).
        target_coupon: Cumulative target that triggers knockout (> 0).
        day_count_fraction: Year fraction applied to each period coupon.

    Returns:
        Dict with keys ``coupons_paid`` (list[float]), ``cumulative``
        (list[float]), ``redemption_index`` (int | None) and
        ``redeemed_early`` (bool).
    """
    ...

def snowball_coupon_profile(
    initial_coupon: float,
    fixed_rate: float,
    floating_fixings: list[float],
    floor: float,
    cap: float,
    is_inverse_floater: bool,
    leverage: float = 1.0,
) -> list[float]:
    """Compute a snowball or inverse-floater coupon schedule.

    Snowball: ``c_i = clip(c_{i-1} + fixed_rate - L_i, floor, cap)``
    with ``c_0 = initial_coupon``.

    Inverse floater: ``c_i = clip(fixed_rate - leverage * L_i, floor, cap)``
    (``initial_coupon`` ignored).

    Pass ``float('inf')`` as ``cap`` for an uncapped coupon.
    """
    ...

def cms_spread_option_intrinsic(
    long_cms: float,
    short_cms: float,
    strike: float,
    is_call: bool,
    notional: float,
) -> float:
    """Undiscounted intrinsic payoff of a CMS spread option.

    Call: ``notional * max(long_cms - short_cms - strike, 0)``.
    Put: ``notional * max(strike - (long_cms - short_cms), 0)``.

    Ignores CMS convexity, vol smile, and correlation adjustments — the
    full product pricer applies those on top of a copula model with
    SABR marginals.
    """
    ...

def callable_range_accrual_accrued(
    lower: float,
    upper: float,
    observations: list[float],
    coupon_rate: float,
    day_count_fraction: float,
) -> float:
    """Accrued coupon over a range-accrual period.

    Counts the fraction of ``observations`` within the inclusive interval
    ``[lower, upper]`` and returns
    ``coupon_rate * day_count_fraction * fraction``.

    The call provision is not applied here — this is the coupon that
    would accrue assuming the note is not called before period end.
    """
    ...

# ---------------------------------------------------------------------------
# Credit events / restructuring
# ---------------------------------------------------------------------------

def execute_recovery_waterfall(
    total_value: float,
    currency: str,
    claims: list[dict],
    allocation_mode: str = "pro_rata",
) -> dict:
    """Run a recovery waterfall over an ordered claim stack.

    Distributes ``total_value`` across ``claims`` in priority order
    following the Absolute Priority Rule (APR). Secured claims first
    recover from their collateral; any shortfall becomes a deficiency
    claim in the unsecured pool.

    Args:
        total_value: Total enterprise value or liquidation proceeds.
        currency: ISO currency code (e.g. ``"USD"``).
        claims: Ordered list of claim dicts. Each supports ``seniority``
            (str: ``first_lien``, ``second_lien``, ``senior_unsecured``,
            ``subordinated``, ``equity``, etc.), ``principal`` (float),
            optional ``accrued``, ``penalties``, ``collateral_value``,
            ``haircut``, ``id``, ``label``.
        allocation_mode: Intra-class allocation; ``"pro_rata"`` (default)
            or ``"strict_priority"``.

    Returns:
        Dict with ``total_distributed``, ``residual``, ``apr_satisfied``,
        ``apr_violations``, and ``per_claim_recovery``.
    """
    ...

def analyze_exchange_offer(
    old_pv: float,
    new_pv: float,
    consent_fee: float = 0.0,
    equity_sweetener_value: float = 0.0,
    exchange_type: str = "par_for_par",
) -> dict:
    """Compare hold-vs-tender economics for a distressed exchange offer.

    Args:
        old_pv: Present value of the existing claim (hold scenario).
        new_pv: Present value of the new instrument (tender scenario).
        consent_fee: Consent / early-tender fee paid to participants.
        equity_sweetener_value: Estimated value of any equity kicker.
        exchange_type: One of ``par_for_par``, ``discount``, ``uptier``,
            ``downtier`` (audit only).

    Returns:
        Dict with ``old_npv``, ``new_npv``, ``tender_total``,
        ``delta_npv``, ``breakeven_recovery``, ``tender_recommended``.
    """
    ...

def analyze_lme(
    lme_type: str,
    notional: float,
    repurchase_price_pct: float,
    opt_acceptance_pct: float = 1.0,
    ebitda: float | None = None,
) -> dict:
    """Analyze a liability management exercise.

    Args:
        lme_type: One of ``open_market`` / ``open_market_repurchase``,
            ``tender_offer``, ``amend_and_extend`` / ``ae``, ``dropdown``.
        notional: Outstanding notional of the target instrument.
        repurchase_price_pct: Price fraction for repurchases/tenders;
            extension fee fraction for A&E; transferred-asset fraction
            for dropdown.
        opt_acceptance_pct: Fraction of holders participating (0.0-1.0).
        ebitda: If provided, a ``leverage_impact`` block is returned.

    Returns:
        Dict with ``cost``, ``notional_reduction``, ``discount_capture``,
        ``discount_capture_pct``, ``remaining_holder_impact_pct``, and
        optional ``leverage_impact``.
    """
    ...
