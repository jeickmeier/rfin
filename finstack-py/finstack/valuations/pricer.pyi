"""Pricer registry bridging instruments and pricing models to valuation engines."""

from __future__ import annotations
import datetime as dt
from typing import Any
from .common import ModelKey, PricerKey
from .results import ValuationResult
from ..core.market_data.context import MarketContext

class PricerRegistry:
    """Central registry for instrument pricing and risk calculations.

    PricerRegistry is the primary entry point for pricing financial instruments
    in finstack. It dispatches (instrument, model) pairs to appropriate pricing
    engines and manages the lifecycle of pricing calculations.

    The registry maintains a mapping of instrument types and pricing models to
    their corresponding pricers. Use :func:`standard_registry` to get
    a registry pre-populated with all standard finstack pricers, or create an
    empty registry and register custom pricers.

    Examples
    --------
        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.term_structures import DiscountCurve
        >>> from finstack.valuations.instruments import Bond
        >>> from finstack.valuations.pricer import standard_registry
        >>> registry = standard_registry()
        >>> bond = (
        ...     Bond
        ...     .builder("BOND-001")
        ...     .money(Money(1_000_000, Currency("USD")))
        ...     .coupon_rate(0.045)
        ...     .issue(date(2023, 1, 1))
        ...     .maturity(date(2028, 1, 1))
        ...     .disc_id("USD")
        ...     .build()
        ... )
        >>> ctx = MarketContext()
        >>> curve = DiscountCurve("USD", date(2024, 1, 1), [(0.0, 1.0), (5.0, 0.97)])
        >>> ctx.insert(curve)
        >>> result = registry.price(bond, "discounting", ctx, date(2024, 1, 1))
        >>> result.value.currency.code
        'USD'

    Notes
    -----
    - Use :func:`standard_registry` for most use cases
    - The registry is thread-safe and can be cloned for parallel pricing
    - Pricing models are specified by string (e.g., "discounting", "credit")
    - MarketContext must contain all required curves/surfaces for the instrument
    - Results include present value, metrics, and explainability trees

    See Also
    --------
    :func:`standard_registry`: Factory for standard registry
    :class:`ValuationResult`: Pricing result envelope
    :class:`MarketContext`: Market data container
    """

    def __init__(self) -> None: ...
    def price(
        self,
        instrument: Any,
        model: Any,
        market: MarketContext,
        as_of: dt.date,
    ) -> ValuationResult:
        """Price an instrument using the specified model and market data.

        This is the primary method for instrument valuation. It dispatches the
        (instrument, model) pair to the appropriate pricer and returns a
        ValuationResult containing present value, currency, and metadata.

        Parameters
        ----------
        instrument
            Instrument instance created from finstack.valuations.instruments
            (e.g., Bond, InterestRateSwap, EquityOption). The instrument type
            determines which pricers are available.
        model : str
            Pricing model identifier. Common models:
            - "discounting": Standard discount curve pricing
            - "credit": Credit-adjusted pricing (requires hazard curve)
            - Model-specific names for options, structured products, etc.
        market : MarketContext
            Market data container with all required curves, surfaces, and FX
            rates. Must contain the curves referenced by the instrument
            (e.g., discount_curve, forward_curve).

        Returns
        -------
        ValuationResult
            Result envelope containing:
            - present_value: Present value as Money
            - currency: Result currency
            - metrics: Dictionary of computed metrics (if any)
            - explanation: Explainability tree (if enabled)

        Raises
        ------
        ValueError
            If the instrument type is not recognized, if the model is not
            available for the instrument, or if required market data is missing
            from the MarketContext.
        RuntimeError
            If pricing fails in the underlying engine (e.g., numerical issues,
            invalid parameters).

        Notes
        -----
        - The instrument must be fully constructed before pricing
        - MarketContext must contain all curves referenced by the instrument
        - Model names are case-insensitive (e.g., "Discounting" = "discounting")
        - Use :meth:`price_with_metrics` to compute risk metrics in one call
        - Results are deterministic and reproducible with the same inputs

        See Also
        --------
        :meth:`price_with_metrics`: Price with risk metrics
        :meth:`price_batch`: Batch pricing for multiple instruments
        :meth:`price_batch_with_metrics`: Batch pricing with risk metrics
        :class:`ValuationResult`: Result structure
        """
        ...

    def get_price(
        self,
        instrument: Any,
        model: Any,
        market: MarketContext,
        as_of: dt.date,
    ) -> ValuationResult:
        """Backward-compatible alias for :meth:`price`."""
        ...

    def price_batch(
        self,
        instruments: list[Any],
        model: Any,
        market: MarketContext,
        as_of: dt.date,
    ) -> list[ValuationResult]:
        """Price a batch of instruments in parallel.

        Parameters
        ----------
        instruments : list[Any]
            List of instruments to price. All instruments must be
            compatible with the specified model.
        model : str
            Pricing model key or name (e.g., "discounting", "credit").
        market : MarketContext
            Market data container with all required curves for every
            instrument in the batch.
        as_of : dt.date
            Valuation date for the pricing run.

        Returns
        -------
        list[ValuationResult]
            List of results in the same order as *instruments*.
        """
        ...

    def price_with_metrics(
        self,
        instrument: Any,
        model: Any,
        market: MarketContext,
        as_of: dt.date,
        metrics: list[Any] | None = None,
    ) -> ValuationResult:
        """Price an instrument and compute the requested risk and return metrics.

        This method prices the instrument and computes all requested metrics in
        a single call, which is more efficient than separate pricing and metric
        calculations. The result includes both present value and all requested
        metrics in the measures dictionary.

        Parameters
        ----------
        instrument
            Instrument instance created from finstack.valuations.instruments
            (e.g., Bond, InterestRateSwap, EquityOption).
        model : str
            Pricing model identifier (e.g., "discounting", "credit", "black_scholes").
        market : MarketContext
            Market data container with all required curves, surfaces, and FX rates.
        as_of : dt.date
            Valuation date for the pricing run.
        metrics : list[Any] | None, optional
            List of metric identifiers to compute. Can be:
            - MetricId instances: MetricId.from_name("dv01")
            - Strings: "dv01", "cs01", "ytm", "z_spread"
            - Mixed list of both
            The documented call shape is ``price_with_metrics(..., as_of, metrics=[...])``.
            The legacy positional order ``price_with_metrics(..., metrics, as_of)``
            remains supported for backward compatibility.

        Returns
        -------
        ValuationResult
            Result envelope containing:
            - value: Present value as Money
            - measures: Dictionary of computed metrics (keyed by metric name)
            - meta: Calculation metadata
            - covenants: Optional covenant reports

        Raises
        ------
        ValueError
            If any metric identifier is invalid or not applicable to the
            instrument type. Use MetricRegistry.is_applicable() to check first.
        RuntimeError
            If pricing fails or if metric calculation encounters an error
            (e.g., missing market data, numerical issues).

        Notes
        -----
        - Metrics are computed in a single pass for efficiency
        - Missing metrics are simply absent from measures (no error)
        - Metric availability depends on instrument type
        - Use MetricRegistry to discover available metrics
        - Metric units vary by type (dollars for DV01/CS01, decimals for yield/spread)

        Common Metrics:
        - **Risk**: "dv01" (dollar value of 1bp), "cs01" (credit spread sensitivity),
          "theta" (time decay), "bucketed_dv01" (key-rate risk)
        - **Yield**: "ytm" (yield to maturity), "ytw" (yield to worst)
        - **Spread**: "z_spread" (Z-spread), "oas" (option-adjusted spread),
          "i_spread" (I-spread), "asw_spread" (asset swap spread)
        - **Pricing**: "clean_price", "dirty_price", "accrued_interest"
        - **Duration**: "duration_modified", "duration_macaulay", "convexity"
        - **Greeks**: "delta", "gamma", "vega", "theta", "rho"

        See Also
        --------
        :meth:`price`: Price without metrics (faster if metrics not needed)
        :class:`MetricId`: Metric identifiers
        :class:`MetricRegistry`: Check metric availability
        :class:`ValuationResult`: Result structure with measures
        """
        ...

    def price_batch_with_metrics(
        self,
        instruments: list[Any],
        model: Any,
        market: MarketContext,
        as_of: dt.date,
        metrics: list[Any] | None = None,
    ) -> list[ValuationResult]:
        """Price a batch of instruments in parallel and compute requested metrics.

        Parameters
        ----------
        instruments : list[Any]
            List of instruments to price. Results preserve this input order.
        model : str
            Pricing model key or name (e.g., "discounting", "credit").
        market : MarketContext
            Market data container with all required curves for every
            instrument in the batch.
        as_of : dt.date
            Valuation date for the pricing run.
        metrics : list[Any] | None, optional
            Metric identifiers to compute for each instrument. The documented call
            shape is ``price_batch_with_metrics(..., as_of, metrics=[...])``.
            The legacy positional order ``price_batch_with_metrics(..., metrics, as_of)``
            remains supported for backward compatibility.

        Returns
        -------
        list[ValuationResult]
            List of results in the same order as *instruments*.
        """
        ...

    def asw_forward(
        self,
        bond: Any,
        market: MarketContext,
        forward_curve: str,
        float_margin_bp: float,
        dirty_price_ccy: float | None = None,
    ) -> tuple[float, float]:
        """Compute par and market asset swap spreads using a forward curve.

        Calculates both par ASW spread (at par price) and market ASW spread
        (at market price) for a bond. The ASW spread is the spread on the
        floating leg of an asset swap that makes the swap have zero value.

        Asset swap spreads are used to compare fixed-rate bonds to floating-rate
        funding alternatives and to measure relative value.

        Parameters
        ----------
        bond
            Bond instrument (must support floating-rate specification via
            forward_curve and float_margin_bp).
        market : MarketContext
            Market data container providing discount curves. Must contain the
            discount curve referenced by the bond.
        forward_curve : str
            Forward curve identifier in MarketContext for the floating leg.
            Typically a 3-month or 6-month rate (e.g., "USD-LIBOR-3M").
        float_margin_bp : float
            Floating margin in basis points added to the forward rate for
            each reset period (e.g., 25.0 for 25bp).
        dirty_price_ccy : float, optional
            Dirty market price in currency units (e.g., 1,015,000 for $1M
            bond at 101.5%). This is required to compute the market ASW leg.
            To compute market ASW at par, pass bond.notional.amount.

        Returns
        -------
        tuple[float, float]
            Tuple of (par_asw_spread_bp, market_asw_spread_bp) in basis points.

        Raises
        ------
        TypeError
            If bond is not a bond instrument or doesn't support floating-rate spec.
        ValueError
            If dirty_price_ccy is None, if forward_curve is not found in MarketContext,
            or if bond doesn't have required floating-rate configuration.
        RuntimeError
            If the underlying ASW calculation fails (e.g., numerical issues).

        Notes
        -----
        - Par ASW: Spread when bond is priced at par (100%)
        - Market ASW: Spread at actual market price (wider if bond trades above par)
        - ASW spread = bond yield - swap rate + funding cost
        - Requires forward curve for floating leg projection
        - Float margin is added to forward rate for each reset period
        - Market ASW > Par ASW when bond trades above par (premium)

        See Also
        --------
        :meth:`price_with_metrics`: Request "asw_spread" metric during pricing
        :class:`Bond`: Bond instruments with floating-rate specs
        :class:`ForwardCurve`: Forward rate curves
        """
        ...

    def key(self, instrument: Any, model: Any) -> PricerKey:
        """Convenience accessor returning the internal dispatch key.

        Args:
            instrument: Instrument instance or instrument label.
            model: Model key or snake-case label.

        Returns:
            PricerKey: Key used internally to resolve engines.

        Raises:
            ValueError: If the arguments cannot be converted.

        Examples:
            >>> from datetime import date
            >>> from finstack.core.currency import Currency
            >>> from finstack.core.money import Money
            >>> from finstack.valuations.instruments import Bond
            >>> from finstack.valuations.pricer import standard_registry
            >>> registry = standard_registry()
            >>> bond = (
            ...     Bond
            ...     .builder("EXAMPLE")
            ...     .money(Money(1_000_000, Currency("USD")))
            ...     .coupon_rate(0.04)
            ...     .issue(date(2024, 1, 1))
            ...     .maturity(date(2029, 1, 1))
            ...     .disc_id("USD")
            ...     .build()
            ... )
            >>> key = registry.key(bond, "discounting")
            >>> (key.instrument.name, key.model.name)
            ('bond', 'discounting')
        """
        ...

    def clone(self) -> PricerRegistry:
        """Clone the registry for use across threads.

        Returns:
            PricerRegistry: A shallow clone sharing the same (immutable) registry.

        Examples:
            >>> from finstack.valuations.pricer import standard_registry
            >>> registry = standard_registry()
            >>> cloned = registry.clone()
            >>> isinstance(cloned, type(registry))
            True
        """
        ...

def standard_registry() -> PricerRegistry:
    """Return the shared registry pre-populated with all standard finstack pricers.

    This factory function returns a shared PricerRegistry with all built-in pricing
    engines registered. It supports pricing for bonds, swaps, options, credit
    instruments, and other standard financial instruments.

    The standard registry includes pricers for:
    - Fixed-income: Bonds, floating-rate notes, zero-coupon bonds
    - Derivatives: Interest rate swaps, swaptions, caps/floors
    - Options: Equity options, FX options, interest rate options
    - Credit: CDS, CDS indices, credit tranches
    - Structured products: ABS, RMBS, CMBS, CLO
    - And more...

    Returns
    -------
    PricerRegistry
        Shared registry instance with all standard pricers loaded and ready to use.

    Examples
    --------
    Create and use the standard registry:

        >>> from finstack.valuations.pricer import standard_registry
        >>> from finstack.valuations.instruments import Bond
        >>> registry = standard_registry()
        >>> # Price any standard instrument
        >>> result = registry.price(bond, "discounting", market_ctx, as_of)
        >>> result = registry.price(swap, "discounting", market_ctx, as_of)
        >>> result = registry.price(option, "black_scholes", market_ctx, as_of)

    Clone for parallel pricing:

        >>> base_registry = standard_registry()
        >>> # Clone for each thread
        >>> thread_registry = base_registry.clone()

    Notes
    -----
    - The standard registry is sufficient for most use cases
    - All standard instrument types are supported
    - Custom pricers can be added to an empty registry if needed
    - The registry is thread-safe and can be cloned for parallel execution
    - Reuse the returned registry across requests in service applications
    - Use this function rather than creating an empty PricerRegistry() for
      standard instruments

    See Also
    --------
    :class:`PricerRegistry`: Registry class for custom configurations
    """
    ...
