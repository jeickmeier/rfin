"""Non-Deliverable Forward (NDF) instrument."""

from __future__ import annotations

from datetime import date

from ....core.currency import Currency
from ....core.market_data.context import MarketContext
from ....core.money import Money
from ...common import InstrumentType

class NdfBuilder:
    """Fluent builder returned by :meth:`Ndf.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def base_currency(self, ccy: str | Currency) -> NdfBuilder: ...
    def settlement_currency(self, ccy: str | Currency) -> NdfBuilder: ...
    def fixing_date(self, date: date) -> NdfBuilder: ...
    def maturity_date(self, date: date) -> NdfBuilder: ...
    def notional(self, notional: Money) -> NdfBuilder: ...
    def contract_rate(self, rate: float) -> NdfBuilder: ...
    def domestic_discount_curve(self, curve_id: str) -> NdfBuilder: ...
    def foreign_discount_curve(self, curve_id: str) -> NdfBuilder: ...
    def fixing_rate(self, rate: float) -> NdfBuilder: ...
    def fixing_source_enum(self, source: str) -> NdfBuilder: ...
    def quote_convention(self, convention: str) -> NdfBuilder: ...
    def spot_rate_override(self, rate: float) -> NdfBuilder: ...
    def base_calendar(self, calendar_id: str) -> NdfBuilder: ...
    def quote_calendar(self, calendar_id: str) -> NdfBuilder: ...
    def build(self) -> Ndf: ...
    def __repr__(self) -> str: ...

class Ndf:
    """Non-Deliverable Forward (NDF) for FX-restricted currency pairs.

    An NDF is a cash-settled FX forward used for currencies that are not
    freely convertible (e.g. CNY, KRW, BRL, INR).  Rather than physical
    delivery, settlement is based on the difference between the contracted
    rate and a reference fixing rate, paid in the settlement (typically
    USD or EUR) currency.

    The fixing rate is sourced from a specified fixing provider (e.g.
    PBOC, Reuters) on the fixing date, which typically precedes the
    maturity / value date by one or two business days.

    Examples
    --------
    Build a 3-month USD/CNY NDF:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments.fx import Ndf
        >>> ndf = (
        ...     Ndf
        ...     .builder("NDF-001")
        ...     .base_currency("CNY")
        ...     .settlement_currency("USD")
        ...     .notional(Money(10_000_000, Currency("CNY")))
        ...     .contract_rate(7.25)
        ...     .fixing_date(date(2024, 9, 3))
        ...     .maturity_date(date(2024, 9, 5))
        ...     .domestic_discount_curve("USD-OIS")
        ...     .build()
        ... )
        >>> ndf.settlement_currency.code
        'USD'

    Attributes
    ----------
    instrument_id : str
        Unique trade identifier.
    base_currency : Currency
        Non-deliverable (restricted) currency.
    settlement_currency : Currency
        Freely convertible settlement currency (typically USD).
    notional : Money
        Notional in the base currency.
    contract_rate : float
        Agreed NDF rate (base-currency units per settlement currency).
    fixing_date : date
        Date on which the reference fixing rate is observed.
    maturity_date : date
        Cash settlement / value date (typically fixing + 1–2 business days).
    fixing_rate : float or None
        Reference fixing rate when already known; ``None`` before fixing.
    fixing_source_enum : str or None
        Fixing source identifier (e.g. ``"PBOC"``, ``"Reuters"``).
    domestic_discount_curve : str
        Discount curve id for the settlement currency.
    foreign_discount_curve : str or None
        Discount curve id for the base currency (optional).
    quote_convention : str
        Quote convention: ``"direct"`` (base per settlement) or ``"indirect"``.
    spot_rate_override : float or None
        Override for the spot rate used in forward calculation.

    MarketContext Requirements
    -------------------------
    - Settlement-currency discount curve.
    - FX forward rate for the pair (when ``fixing_rate`` is not yet known).

    See Also
    --------
    :class:`FxForward` : Physically-settled FX forward.
    :class:`FxSwap` : FX swap (near + far legs).
    """

    @classmethod
    def builder(cls, instrument_id: str) -> NdfBuilder: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    @property
    def base_currency(self) -> Currency: ...
    @property
    def settlement_currency(self) -> Currency: ...
    @property
    def fixing_date(self) -> date: ...
    @property
    def maturity_date(self) -> date: ...
    @property
    def notional(self) -> Money: ...
    @property
    def contract_rate(self) -> float: ...
    @property
    def fixing_rate(self) -> float | None: ...
    @property
    def fixing_source_enum(self) -> str | None: ...
    @property
    def domestic_discount_curve(self) -> str: ...
    @property
    def foreign_discount_curve(self) -> str | None: ...
    @property
    def quote_convention(self) -> str: ...
    @property
    def spot_rate_override(self) -> float | None: ...
    @property
    def base_calendar(self) -> str | None: ...
    @property
    def quote_calendar(self) -> str | None: ...
    def is_fixed(self) -> bool: ...
    def value(self, market: MarketContext, as_of: date) -> Money: ...
    def validate_fixing_source(self) -> None: ...
    def __repr__(self) -> str: ...
