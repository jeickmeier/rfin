"""FX forward (outright forward) instrument."""

from __future__ import annotations

from datetime import date

from ....core.currency import Currency
from ....core.market_data.context import MarketContext
from ....core.money import Money
from ...common import InstrumentType

class FxForwardBuilder:
    """Fluent builder returned by :meth:`FxForward.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
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
    """FX outright forward contract for exchanging currencies at a future date.

    An FX forward locks in an exchange rate today for delivery at a specified
    future date.  The value at inception is zero if the ``contract_rate``
    equals the fair forward rate; subsequently the mark-to-market value
    reflects the present value of the difference between the contracted
    rate and the prevailing forward rate.

    Pricing follows the covered-interest-parity (CIP) model:

    .. math::

        F = S \\cdot \\frac{B_d(0, T)}{B_f(0, T)}

    where :math:`S` is the spot rate, :math:`B_d` the domestic discount
    factor, and :math:`B_f` the foreign discount factor.

    Examples
    --------
    Build a 6-month EUR/USD outright forward:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments.fx import FxForward
        >>> fwd = (
        ...     FxForward
        ...     .builder("FX-FWD-001")
        ...     .base_currency("EUR")
        ...     .quote_currency("USD")
        ...     .notional(Money(2_000_000, Currency("EUR")))
        ...     .contract_rate(1.0950)
        ...     .maturity(date(2024, 12, 5))
        ...     .domestic_discount_curve("USD-OIS")
        ...     .foreign_discount_curve("EUR-OIS")
        ...     .build()
        ... )
        >>> fwd.base_currency.code
        'EUR'

    Attributes
    ----------
    instrument_id : str
        Unique trade identifier.
    base_currency : Currency
        Foreign (base) currency of the pair.
    quote_currency : Currency
        Domestic (quote) currency of the pair.
    notional : Money
        Notional amount in the base currency.
    maturity : date
        Settlement / delivery date of the forward.
    contract_rate : float or None
        Agreed forward rate (quote-per-base); ``None`` when mark-to-market only.
    spot_rate_override : float or None
        Override for the spot rate used in forward rate calculation.
    domestic_discount_curve : str
        Curve id for the domestic (quote-currency) discount curve.
    foreign_discount_curve : str
        Curve id for the foreign (base-currency) discount curve.
    base_calendar : str or None
        Holiday calendar for the base currency leg.
    quote_calendar : str or None
        Holiday calendar for the quote currency leg.

    MarketContext Requirements
    -------------------------
    - Domestic discount curve (quote currency).
    - Foreign discount curve (base currency).
    - FX spot rate for the pair (when ``spot_rate_override`` is not set).

    See Also
    --------
    :class:`FxSpot` : FX spot transaction.
    :class:`FxSwap` : FX swap (near + far legs).
    :class:`Ndf` : Non-Deliverable Forward for restricted currencies.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> FxForwardBuilder: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    @property
    def base_currency(self) -> Currency: ...
    @property
    def quote_currency(self) -> Currency: ...
    @property
    def maturity(self) -> date: ...
    @property
    def notional(self) -> Money: ...
    @property
    def contract_rate(self) -> float | None: ...
    @property
    def spot_rate_override(self) -> float | None: ...
    @property
    def domestic_discount_curve(self) -> str: ...
    @property
    def foreign_discount_curve(self) -> str: ...
    @property
    def base_calendar(self) -> str | None: ...
    @property
    def quote_calendar(self) -> str | None: ...
    def value(self, market: MarketContext, as_of: date) -> Money: ...
    def market_forward_rate(self, market: MarketContext, as_of: date) -> float: ...
    def __repr__(self) -> str: ...
