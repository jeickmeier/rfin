"""FX forward (outright forward) instrument."""

from __future__ import annotations

from datetime import date

from ....core.currency import Currency
from ....core.money import Money
from ...common import InstrumentType

class FxForwardBuilder:
    """Fluent builder returned by :meth:`FxForward.builder`."""

    def base_currency(self, ccy: str | Currency) -> FxForwardBuilder: ...
    def quote_currency(self, ccy: str | Currency) -> FxForwardBuilder: ...
    def maturity(self, date: date) -> FxForwardBuilder: ...
    def notional(self, notional: Money) -> FxForwardBuilder: ...
    def contract_rate(self, rate: float) -> FxForwardBuilder: ...
    def domestic_discount_curve(self, curve_id: str) -> FxForwardBuilder: ...
    def foreign_discount_curve(self, curve_id: str) -> FxForwardBuilder: ...
    def spot_rate_override(self, rate: float) -> FxForwardBuilder: ...
    def base_calendar(self, calendar_id: str) -> FxForwardBuilder: ...
    def quote_calendar(self, calendar_id: str) -> FxForwardBuilder: ...
    def build(self) -> FxForward: ...
    def __repr__(self) -> str: ...

class FxForward:
    """FX forward (outright forward) instrument.

    Represents a commitment to exchange one currency for another at a specified
    future date at a predetermined rate. The position is long base currency
    (foreign) and short quote currency (domestic).

    Pricing:

    - Forward rate via covered interest rate parity:
      F_market = S * DF_foreign(T) / DF_domestic(T)
    - PV = notional * (F_market - F_contract) * DF_domestic(T)

    Examples
    --------
    Create a 6-month EUR/USD forward:

        >>> from finstack.valuations.instruments import FxForward
        >>> fwd = (
        ...     FxForward
        ...     .builder("EURUSD-FWD-6M")
        ...     .base_currency("EUR")
        ...     .quote_currency("USD")
        ...     .maturity(date(2025, 6, 15))
        ...     .notional(Money(1_000_000, "EUR"))
        ...     .domestic_discount_curve("USD-OIS")
        ...     .foreign_discount_curve("EUR-OIS")
        ...     .contract_rate(1.12)
        ...     .build()
        ... )

    See Also
    --------
    :class:`Ndf`: Non-deliverable forward
    :class:`FxSwap`: FX swap
    """

    @classmethod
    def builder(cls, instrument_id: str) -> FxForwardBuilder:
        """Create a builder for an FX forward contract."""
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
    def base_currency(self) -> Currency:
        """Base currency (foreign currency, numerator of the pair)."""
        ...
    @property
    def quote_currency(self) -> Currency:
        """Quote currency (domestic currency, denominator of the pair)."""
        ...
    @property
    def maturity(self) -> date:
        """Maturity/settlement date."""
        ...
    @property
    def notional(self) -> Money:
        """Notional amount in base currency."""
        ...
    @property
    def contract_rate(self) -> float | None:
        """Contract forward rate (quote per base). None if at-market."""
        ...
    @property
    def spot_rate_override(self) -> float | None:
        """Spot rate override (if set)."""
        ...
    def __repr__(self) -> str: ...
