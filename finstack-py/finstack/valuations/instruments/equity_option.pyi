"""Equity option instrument (builder-only API)."""

from __future__ import annotations
from datetime import date
from ...core.money import Money
from ...core.currency import Currency
from ...core.dates.daycount import DayCount
from ..common import InstrumentType

class EquityOptionBuilder:
    """Fluent builder returned by :meth:`EquityOption.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def ticker(self, ticker: str) -> "EquityOptionBuilder": ...
    def strike(self, strike: float) -> "EquityOptionBuilder": ...
    def option_type(self, option_type: str) -> "EquityOptionBuilder": ...
    def exercise_style(self, exercise_style: str) -> "EquityOptionBuilder": ...
    def expiry(self, expiry: date) -> "EquityOptionBuilder": ...
    def day_count(self, day_count: DayCount | str) -> "EquityOptionBuilder": ...
    def settlement(self, settlement: str) -> "EquityOptionBuilder": ...
    def disc_id(self, curve_id: str) -> "EquityOptionBuilder": ...
    def spot_id(self, spot_id: str) -> "EquityOptionBuilder": ...
    def vol_surface(self, vol_surface: str) -> "EquityOptionBuilder": ...
    def div_yield_id(self, div_yield_id: str | None = ...) -> "EquityOptionBuilder": ...
    def build(self) -> "EquityOption": ...

class EquityOption:
    """Equity option instrument for pricing European and American options.

    EquityOption represents a call or put option on a single equity or equity
    index. Options are priced using Black-Scholes or similar models, requiring
    a discount curve, spot price, and volatility surface in the MarketContext.

    Options can be European (exercisable only at expiry) or American (exercisable
    at any time before expiry). The instrument supports standard market conventions
    and can be priced with various models (Black-Scholes, binomial, etc.).

    Examples
    --------
    Create a European call option:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import EquityOption
        >>> option = (
        ...     EquityOption
        ...     .builder("SPX-CALL-4500")
        ...     .ticker("SPX")
        ...     .strike(4500.0)
        ...     .expiry(date(2024, 12, 20))
        ...     .option_type("call")
        ...     .exercise_style("european")
        ...     .disc_id("USD-OIS")
        ...     .spot_id("SPX")
        ...     .vol_surface("EQUITY-VOL")
        ...     .build()
        ... )

    Price the option:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.scalars import MarketScalar
        >>> from finstack.core.market_data.surfaces import VolSurface
        >>> from finstack.core.market_data.term_structures import DiscountCurve
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import EquityOption
        >>> from finstack.valuations.pricer import create_standard_registry
        >>> option = (
        ...     EquityOption
        ...     .builder("SPX-CALL-4500")
        ...     .ticker("SPX")
        ...     .strike(4500.0)
        ...     .expiry(date(2024, 12, 20))
        ...     .option_type("call")
        ...     .exercise_style("european")
        ...     .disc_id("USD-OIS")
        ...     .spot_id("SPX")
        ...     .vol_surface("EQUITY-VOL")
        ...     .build()
        ... )
        >>> ctx = MarketContext()
        >>> ctx.insert_discount(DiscountCurve("USD-OIS", date(2024, 1, 1), [(0.0, 1.0), (1.0, 0.99)]))
        >>> expiries = [0.5, 1.0, 2.0]
        >>> strikes = [4000.0, 4500.0, 5000.0]
        >>> grid = [
        ...     [0.28, 0.27, 0.26],
        ...     [0.27, 0.26, 0.25],
        ...     [0.26, 0.25, 0.24],
        ... ]
        >>> ctx.insert_surface(VolSurface("EQUITY-VOL", expiries, strikes, grid))
        >>> spot_scalar = MarketScalar.price(Money(4400, Currency("USD")))
        >>> ctx.insert_price("SPX", spot_scalar)
        >>> ctx.insert_price("EQUITY-SPOT", spot_scalar)
        >>> registry = create_standard_registry()
        >>> pv = registry.price(option, "black76", ctx).value
        >>> pv.currency.code
        'USD'

    Notes
    -----
    - Options require discount curve, spot price, and volatility surface
    - Strike is in absolute terms (not moneyness)
    - Contract size multiplies the notional for position sizing
    - Dividend yield can be specified for dividend-paying stocks
    - Use :meth:`builder` for all configurations (builder-only API)

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required for pricing; provided explicitly via ``builder`` or by the pricer).
    - Spot price: ``spot_id`` (required for pricing; provided explicitly via ``builder`` or by ticker/MarketContext conventions).
    - Volatility surface: ``vol_surface`` (required for pricing; provided explicitly via ``builder`` or via pricing config).
    - Dividend yield: ``div_yield_id`` (optional; used when provided).

    See Also
    --------
    :class:`Swaption`: Interest rate swaptions
    :class:`InterestRateOption`: Interest rate caps/floors
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - Black & Scholes (1973): see ``docs/REFERENCES.md#blackScholes1973``.
    - Merton (1973): see ``docs/REFERENCES.md#merton1973``.
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> EquityOptionBuilder: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def ticker(self) -> str: ...
    @property
    def strike(self) -> float: ...
    @property
    def notional(self) -> Money: ...
    @property
    def option_type(self) -> str: ...
    @property
    def exercise_style(self) -> str: ...
    @property
    def expiry(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def vol_surface(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
