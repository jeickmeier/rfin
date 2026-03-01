"""Equity index future contract instrument."""

from __future__ import annotations

from datetime import date

from ...core.currency import Currency
from ...core.money import Money
from ..common import InstrumentType


class FuturePosition:
    """Position side (Long or Short) for futures contracts."""

    LONG: FuturePosition
    SHORT: FuturePosition

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...


class EquityFutureSpecs:
    """Equity index future contract specifications.

    Contains exchange-specific contract parameters such as multiplier,
    tick size, and settlement method.
    """

    def __init__(
        self,
        multiplier: float,
        tick_size: float,
        tick_value: float,
        settlement_method: str,
    ) -> None: ...
    @property
    def multiplier(self) -> float:
        """Contract multiplier (currency per index point)."""
        ...
    @property
    def tick_size(self) -> float:
        """Tick size in index points."""
        ...
    @property
    def tick_value(self) -> float:
        """Tick value in currency units."""
        ...
    @property
    def settlement_method(self) -> str:
        """Settlement method description."""
        ...
    def __repr__(self) -> str: ...


class EquityIndexFutureBuilder:
    """Fluent builder returned by :meth:`EquityIndexFuture.builder`."""

    def index_ticker(self, ticker: str) -> EquityIndexFutureBuilder: ...
    def notional(self, money: Money) -> EquityIndexFutureBuilder: ...
    def expiry_date(self, date: date) -> EquityIndexFutureBuilder: ...
    def last_trading_date(self, date: date) -> EquityIndexFutureBuilder: ...
    def entry_price(self, price: float) -> EquityIndexFutureBuilder: ...
    def quoted_price(self, price: float) -> EquityIndexFutureBuilder: ...
    def position(self, pos: str | FuturePosition) -> EquityIndexFutureBuilder: ...
    def contract_specs(self, specs: EquityFutureSpecs) -> EquityIndexFutureBuilder: ...
    def discount_curve(self, curve_id: str) -> EquityIndexFutureBuilder: ...
    def spot_id(self, id: str) -> EquityIndexFutureBuilder: ...
    def div_yield_id(self, id: str) -> EquityIndexFutureBuilder: ...
    def dividend_yield_id(self, id: str) -> EquityIndexFutureBuilder:
        """Backward-compatible alias for div_yield_id."""
        ...
    def build(self) -> EquityIndexFuture: ...
    def __repr__(self) -> str: ...


class EquityIndexFuture:
    """Equity index future contract.

    Represents a futures contract on an equity index such as S&P 500, Nasdaq-100,
    Euro Stoxx 50, DAX, FTSE 100, or Nikkei 225.

    The contract supports two pricing modes:

    1. **Mark-to-Market** (when quoted_price is provided):
       NPV = (quoted_price - entry_price) x contracts x position_sign

    2. **Fair Value** (cost-of-carry model):
       F = S0 x exp((r - q) x T)
       NPV = (F - entry_price) x contracts x position_sign

    Examples
    --------
    Create an E-mini S&P 500 future:

        >>> from finstack.valuations.instruments import EquityIndexFuture
        >>> future = (
        ...     EquityIndexFuture.builder("ES-2025M03")
        ...     .index_ticker("SPX")
        ...     .notional(Money.from_code(2_250_000.0, "USD"))
        ...     .expiry_date(date(2025, 3, 21))
        ...     .last_trading_date(date(2025, 3, 20))
        ...     .entry_price(4500.0)
        ...     .quoted_price(4550.0)
        ...     .position("long")
        ...     .contract_specs(EquityIndexFuture.sp500_emini_specs())
        ...     .discount_curve("USD-OIS")
        ...     .spot_id("SPX-SPOT")
        ...     .build()
        ... )

    See Also
    --------
    :class:`InterestRateFuture`: Interest rate futures
    :class:`BondFuture`: Bond futures
    """

    @classmethod
    def builder(cls, instrument_id: str) -> EquityIndexFutureBuilder:
        """Create a builder for an equity index future contract."""
        ...
    @classmethod
    def sp500_emini_specs(cls) -> EquityFutureSpecs:
        """Create E-mini S&P 500 contract specifications."""
        ...
    @classmethod
    def nasdaq100_emini_specs(cls) -> EquityFutureSpecs:
        """Create E-mini Nasdaq-100 contract specifications."""
        ...
    @classmethod
    def sp500_micro_emini_specs(cls) -> EquityFutureSpecs:
        """Create Micro E-mini S&P 500 contract specifications."""
        ...
    @classmethod
    def euro_stoxx_50_specs(cls) -> EquityFutureSpecs:
        """Create Euro Stoxx 50 future contract specifications."""
        ...
    @classmethod
    def dax_specs(cls) -> EquityFutureSpecs:
        """Create DAX future contract specifications."""
        ...
    @classmethod
    def ftse_100_specs(cls) -> EquityFutureSpecs:
        """Create FTSE 100 future contract specifications."""
        ...
    @classmethod
    def nikkei_225_specs(cls) -> EquityFutureSpecs:
        """Create Nikkei 225 future contract specifications."""
        ...
    @property
    def instrument_id(self) -> str:
        """Instrument identifier."""
        ...
    @property
    def instrument_type(self) -> InstrumentType:
        """Instrument type."""
        ...
    @property
    def index_ticker(self) -> str:
        """Index ticker symbol (e.g., 'SPX', 'NDX')."""
        ...
    @property
    def currency(self) -> Currency:
        """Settlement currency."""
        ...
    @property
    def notional(self) -> Money:
        """Position notional."""
        ...
    @property
    def expiry_date(self) -> date:
        """Expiry/settlement date."""
        ...
    @property
    def last_trading_date(self) -> date:
        """Last trading date."""
        ...
    @property
    def entry_price(self) -> float | None:
        """Entry price (if set)."""
        ...
    @property
    def quoted_price(self) -> float | None:
        """Quoted market price (if set)."""
        ...
    @property
    def position(self) -> FuturePosition:
        """Position side (Long or Short)."""
        ...
    @property
    def contract_specs(self) -> EquityFutureSpecs:
        """Contract specifications."""
        ...
    def notional_value(self, price: float) -> float:
        """Calculate the notional value of the position at a given price."""
        ...
    def delta(self) -> float:
        """Calculate delta exposure (index point sensitivity)."""
        ...
    def __repr__(self) -> str: ...
