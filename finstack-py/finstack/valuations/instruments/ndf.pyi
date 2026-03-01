"""Non-Deliverable Forward (NDF) instrument."""

from __future__ import annotations

from datetime import date

from ...core.currency import Currency
from ...core.market_data.context import MarketContext
from ...core.money import Money
from ..common import InstrumentType


class NdfBuilder:
    """Fluent builder returned by :meth:`Ndf.builder`."""

    def base_currency(self, ccy: str | Currency) -> NdfBuilder: ...
    def settlement_currency(self, ccy: str | Currency) -> NdfBuilder: ...
    def fixing_date(self, date: date) -> NdfBuilder: ...
    def maturity_date(self, date: date) -> NdfBuilder: ...
    def notional(self, notional: Money) -> NdfBuilder: ...
    def contract_rate(self, rate: float) -> NdfBuilder: ...
    def settlement_curve(self, curve_id: str) -> NdfBuilder: ...
    def foreign_curve(self, curve_id: str) -> NdfBuilder: ...
    def fixing_rate(self, rate: float) -> NdfBuilder: ...
    def fixing_source_enum(self, source: str) -> NdfBuilder: ...
    def quote_convention(self, convention: str) -> NdfBuilder: ...
    def spot_rate_override(self, rate: float) -> NdfBuilder: ...
    def base_calendar(self, calendar_id: str) -> NdfBuilder: ...
    def quote_calendar(self, calendar_id: str) -> NdfBuilder: ...
    def build(self) -> Ndf: ...
    def __repr__(self) -> str: ...


class Ndf:
    """Non-Deliverable Forward (NDF) instrument.

    Represents a cash-settled forward contract on a restricted currency pair.
    The position is long base currency (restricted) and short settlement currency.

    Pricing Modes:

    - Pre-Fixing (fixing_rate not set): Forward rate is estimated via covered
      interest rate parity. PV = notional x (F_market - contract_rate) x DF(T)
    - Post-Fixing (fixing_rate set): Uses the observed fixing rate.
      PV = notional x (fixing_rate - contract_rate) x DF(T)

    Examples
    --------
    Create a 3-month USD/CNY NDF:

        >>> from finstack.valuations.instruments import Ndf
        >>> ndf = (
        ...     Ndf.builder("USDCNY-NDF-3M")
        ...     .base_currency("CNY")
        ...     .settlement_currency("USD")
        ...     .fixing_date(date(2025, 3, 13))
        ...     .maturity_date(date(2025, 3, 15))
        ...     .notional(Money(10_000_000, "CNY"))
        ...     .contract_rate(7.25)
        ...     .settlement_curve("USD-OIS")
        ...     .quote_convention("base_per_settlement")
        ...     .fixing_source_enum("CNHFIX")
        ...     .build()
        ... )

    See Also
    --------
    :class:`FxSpot`: FX spot transaction
    :class:`FxSwap`: FX swap
    """

    @classmethod
    def builder(cls, instrument_id: str) -> NdfBuilder:
        """Create a builder for a non-deliverable forward contract."""
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
        """Base currency (restricted/non-deliverable, numerator)."""
        ...
    @property
    def settlement_currency(self) -> Currency:
        """Settlement currency (convertible, denominator)."""
        ...
    @property
    def fixing_date(self) -> date:
        """Fixing date (rate observation date)."""
        ...
    @property
    def maturity_date(self) -> date:
        """Maturity/settlement date."""
        ...
    @property
    def notional(self) -> Money:
        """Notional amount in base currency."""
        ...
    @property
    def contract_rate(self) -> float:
        """Contract forward rate (base per settlement)."""
        ...
    @property
    def fixing_rate(self) -> float | None:
        """Observed fixing rate (if set)."""
        ...
    @property
    def fixing_source_enum(self) -> str | None:
        """Fixing source/benchmark (e.g., 'CNHFIX', 'RBI', 'PTAX')."""
        ...
    def is_fixed(self) -> bool:
        """Check if NDF is in post-fixing mode (fixing rate is set)."""
        ...
    def value(self, market: MarketContext, as_of: date) -> Money:
        """Calculate present value in settlement currency."""
        ...
    def __repr__(self) -> str: ...
