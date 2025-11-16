"""Pricer registry bridging instruments and pricing models to valuation engines."""

from typing import Any, List, Tuple
from .common import ModelKey, PricerKey
from .results import ValuationResult
from ..core.market_data import MarketContext

class PricerRegistry:
    """Registry dispatching (instrument, model) pairs to pricing engines.

    Examples:
        >>> registry = create_standard_registry()
        >>> result = registry.price(bond, "discounting", market)
        >>> result.present_value
    """

    def __init__(self) -> None:
        """Create an empty registry instance.

        Returns:
            PricerRegistry: Registry without any registered engines.

        Examples:
            >>> empty = PricerRegistry()
            >>> list(empty.__dict__.keys())
            []
        """
        ...

    def price(self, instrument: Any, model: Any, market: MarketContext) -> ValuationResult:
        """Price an instrument given a model key and market data.

        Args:
            instrument: Instrument instance created from finstack.valuations.instruments.
            model: Pricing model key or its snake-case label.
            market: Market context supplying curves, spreads, and FX data.

        Returns:
            ValuationResult: Envelope containing PV, measures, and metadata.

        Raises:
            ValueError: If the instrument or model cannot be interpreted.
            RuntimeError: If pricing fails in the underlying engine.

        Examples:
            >>> registry = create_standard_registry()
            >>> result = registry.price(bond, "discounting", market)
            >>> result.present_value.amount
            123.45
        """
        ...

    def price_with_metrics(
        self, instrument: Any, model: Any, market: MarketContext, metrics: List[Any]
    ) -> ValuationResult:
        """Price an instrument and compute the requested metrics.

        Args:
            instrument: Instrument instance created from the bindings.
            model: Pricing model key or name.
            market: Market context with the necessary curve data.
            metrics: Iterable of metric identifiers or names to evaluate.

        Returns:
            ValuationResult: Pricing result enriched with computed metrics.

        Raises:
            ValueError: If any metric identifier is invalid.
            RuntimeError: If pricing or metric calculation fails.

        Examples:
            >>> registry = create_standard_registry()
            >>> result = registry.price_with_metrics(bond, "discounting", market, ["dv01"])
            >>> result.metrics["dv01"].value
            -415.2
        """
        ...

    def asw_forward(
        self,
        bond: Any,
        market: MarketContext,
        forward_curve: str,
        float_margin_bp: float,
        dirty_price_ccy: float = None,
    ) -> Tuple[float, float]:
        """Compute par and market asset swap spreads using a forward curve.

        Args:
            bond: Bond instrument previously constructed in the bindings.
            market: Market context providing discount curves.
            forward_curve: Identifier for the forward curve.
            float_margin_bp: Floating margin in basis points.
            dirty_price_ccy: Dirty market price expressed in currency.
                This argument is **required** to compute the market ASW leg.
                To interpret the market spread relative to par, explicitly pass
                the par notional amount (for example, ``bond.notional.amount``);
                omitting it will cause the underlying engine to raise a configuration
                error instead of silently assuming a par price.

        Returns:
            tuple[float, float]: Par and market asset swap spreads in basis points.

        Raises:
            TypeError: If bond is not a bond instrument.
            RuntimeError: If the underlying calculation fails.

        Examples:
            >>> registry = create_standard_registry()
            >>> registry.asw_forward(bond, market, "usd_libor_3m", 25.0)
            (23.4, 27.1)
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
            >>> registry = create_standard_registry()
            >>> registry.key(bond, "discounting").instrument
            InstrumentType.BOND
        """
        ...

    def clone(self) -> PricerRegistry:
        """Clone the registry for isolated pricing threads.

        Returns:
            PricerRegistry: Fresh registry without shared state.

        Examples:
            >>> registry = create_standard_registry()
            >>> isolated = registry.clone()
            >>> isinstance(isolated, PricerRegistry)
            True
        """
        ...

def create_standard_registry() -> PricerRegistry:
    """Create a registry populated with all standard finstack pricers.

    Returns:
        PricerRegistry: Registry with all built-in pricers loaded.

    Examples:
        >>> registry = create_standard_registry()
        >>> registry.price(bond, "discounting", market)
        <ValuationResult ...>
    """
    ...
