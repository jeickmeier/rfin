"""Equity instrument (builder-only API)."""

from typing import Optional, Union
from ...core.currency import Currency
from ..common import InstrumentType

class EquityBuilder:
    """Fluent builder returned by :meth:`Equity.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def ticker(self, ticker: str) -> EquityBuilder: ...
    def currency(self, currency: Union[str, Currency]) -> EquityBuilder: ...
    def shares(self, shares: float) -> EquityBuilder: ...
    def price(self, price: float) -> EquityBuilder: ...
    def price_id(self, price_id: str) -> EquityBuilder: ...
    def div_yield_id(self, div_yield_id: str) -> EquityBuilder: ...
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
        ...     Equity.builder("EQUITY-AAPL")
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
    def price_quote(self) -> Optional[float]: ...
    @property
    def price_id(self) -> Optional[str]: ...
    @property
    def div_yield_id(self) -> Optional[str]: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
