"""Equity instrument (builder-only API)."""

from __future__ import annotations

from datetime import date

from ....core.currency import Currency
from ....core.market_data.context import MarketContext
from ....core.money import Money
from ...common import InstrumentType

class EquityBuilder:
    """Fluent builder returned by :meth:`Equity.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def ticker(self, ticker: str) -> EquityBuilder: ...
    def currency(self, currency: str | Currency) -> EquityBuilder: ...
    def shares(self, shares: float) -> EquityBuilder: ...
    def price(self, price: float) -> EquityBuilder: ...
    def price_id(self, price_id: str) -> EquityBuilder: ...
    def div_yield_id(self, div_yield_id: str) -> EquityBuilder: ...
    def discount_curve_id(self, discount_curve_id: str) -> EquityBuilder: ...
    def build(self) -> "Equity": ...

class Equity:
    """Spot equity position for equity valuation and portfolio modeling.

    Equity represents a long or short position in a single equity or equity
    index. It can be used for portfolio valuation, risk calculations, and
    as an underlying for equity derivatives.

    Equity positions are valued using spot prices from MarketContext and can
    include dividend yield for total return calculations.

    Examples
    --------
    Create an equity position:

        >>> from finstack.valuations.instruments import Equity
        >>> from finstack import Currency
        >>> equity = (
        ...     Equity
        ...     .builder("EQUITY-AAPL")
        ...     .ticker("AAPL")
        ...     .currency(Currency("USD"))
        ...     .shares(100.0)
        ...     .price_id("AAPL")  # Spot price ID in MarketContext
        ...     .build()
        ... )

    Notes
    -----
    - Equity requires spot price in MarketContext (via price_id or MarketScalar)
    - Shares can be positive (long) or negative (short)
    - Dividend yield can be specified for total return calculations
    - Price can be provided directly or retrieved from MarketContext

    MarketContext Requirements
    -------------------------
    - Spot price: ``price_id`` (required if ``price`` is not provided).
    - Dividend yield: ``div_yield_id`` (optional; used when provided).

    See Also
    --------
    :class:`EquityOption`: Equity options
    :class:`EquityTotalReturnSwap`: Equity TRS
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> EquityBuilder: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def ticker(self) -> str: ...
    @property
    def currency(self) -> Currency: ...
    @property
    def shares(self) -> float: ...
    @property
    def price_quote(self) -> float | None: ...
    @property
    def price_id(self) -> str | None: ...
    @property
    def div_yield_id(self) -> str | None: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def value(self, market: MarketContext, as_of: date) -> Money: ...
    def price_per_share(self, market: MarketContext, as_of: date) -> Money: ...
    def dividend_yield(self, market: MarketContext) -> float: ...
    def forward_price_per_share(self, market: MarketContext, as_of: date, t: float) -> Money: ...
    def forward_value(self, market: MarketContext, as_of: date, t: float) -> Money: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
