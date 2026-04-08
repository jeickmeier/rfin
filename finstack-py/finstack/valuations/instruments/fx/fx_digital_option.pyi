"""FX digital (binary) option instrument."""

from __future__ import annotations
from typing import Self

from datetime import date

from ....core.currency import Currency
from ....core.dates.daycount import DayCount
from ....core.market_data.context import MarketContext
from ....core.money import Money
from ...common import InstrumentType

class DigitalPayoutType:
    """Digital option payout type."""

    CASH_OR_NOTHING: DigitalPayoutType
    ASSET_OR_NOTHING: DigitalPayoutType
    @classmethod
    def from_name(cls, name: str) -> DigitalPayoutType: ...
    @property
    def name(self) -> str: ...

class FxDigitalOptionBuilder:
    """Fluent builder returned by :meth:`FxDigitalOption.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def base_currency(self, ccy: str | Currency) -> FxDigitalOptionBuilder: ...
    def quote_currency(self, ccy: str | Currency) -> FxDigitalOptionBuilder: ...
    def strike(self, strike: float) -> FxDigitalOptionBuilder: ...
    def option_type(self, option_type: str) -> FxDigitalOptionBuilder: ...
    def payout_type(self, payout_type: str | DigitalPayoutType) -> FxDigitalOptionBuilder: ...
    def payout_amount(self, amount: Money) -> FxDigitalOptionBuilder: ...
    def expiry(self, date: date) -> FxDigitalOptionBuilder: ...
    def notional(self, notional: Money) -> FxDigitalOptionBuilder: ...
    def domestic_discount_curve(self, curve_id: str) -> FxDigitalOptionBuilder: ...
    def foreign_discount_curve(self, curve_id: str) -> FxDigitalOptionBuilder: ...
    def vol_surface(self, surface_id: str) -> FxDigitalOptionBuilder: ...
    def day_count(self, dc: DayCount) -> FxDigitalOptionBuilder: ...
    def build(self) -> FxDigitalOption: ...
    def __repr__(self) -> str: ...

class FxDigitalOption:
    """FX digital (binary) option paying a fixed amount on expiry condition.

    An FX digital option pays a fixed cash or asset amount if the spot rate
    is above (call) or below (put) the strike at expiry.  Two payout types
    are supported:

    - **Cash-or-Nothing**: pays a fixed cash amount if in-the-money.
    - **Asset-or-Nothing**: pays the value of the base-currency notional if in-the-money.

    Pricing uses the closed-form digital formula derived from the
    Garman-Kohlhagen model (the derivative of the vanilla option price
    with respect to the strike).

    Examples
    --------
    Build a EUR/USD cash-or-nothing call:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments.fx import FxDigitalOption
        >>> opt = (
        ...     FxDigitalOption
        ...     .builder("FX-DIG-001")
        ...     .base_currency("EUR")
        ...     .quote_currency("USD")
        ...     .notional(Money(1_000_000, Currency("EUR")))
        ...     .option_type("call")
        ...     .payout_type("cash_or_nothing")
        ...     .payout_amount(Money(500_000, Currency("USD")))
        ...     .strike(1.10)
        ...     .expiry(date(2024, 12, 20))
        ...     .domestic_discount_curve("USD-OIS")
        ...     .foreign_discount_curve("EUR-OIS")
        ...     .vol_surface("EURUSD-VOL")
        ...     .build()
        ... )
        >>> opt.option_type
        'call'

    Attributes
    ----------
    instrument_id : str
        Unique trade identifier.
    base_currency : Currency
        Foreign (base) currency of the pair.
    quote_currency : Currency
        Domestic (quote) currency of the pair.
    notional : Money
        Reference notional.
    strike : float
        Option strike in quote-per-base terms.
    option_type : str
        ``"call"`` (pays if spot > strike) or ``"put"`` (pays if spot < strike).
    payout_type : DigitalPayoutType
        ``CASH_OR_NOTHING`` or ``ASSET_OR_NOTHING``.
    payout_amount : Money
        Fixed amount paid on exercise (for cash-or-nothing).
    expiry : date
        Expiration date.
    domestic_discount_curve : str
        Discount curve id for the domestic (quote-currency) leg.
    foreign_discount_curve : str
        Discount curve id for the foreign (base-currency) leg.
    vol_surface : str
        Volatility surface id.
    day_count : DayCount
        Day count convention for time-to-expiry calculation.

    MarketContext Requirements
    -------------------------
    - Domestic and foreign discount curves.
    - FX volatility surface.
    - FX spot rate.

    See Also
    --------
    :class:`FxOption` : FX vanilla option (Garman-Kohlhagen).
    :class:`FxTouchOption` : FX touch (American binary) option.
    :class:`FxBarrierOption` : FX barrier option.

    Sources
    -------
    - Garman & Kohlhagen (1983): see ``docs/REFERENCES.md#garmanKohlhagen1983``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> FxDigitalOptionBuilder: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    @property
    def base_currency(self) -> Currency: ...
    @property
    def quote_currency(self) -> Currency: ...
    @property
    def strike(self) -> float: ...
    @property
    def option_type(self) -> str: ...
    @property
    def payout_type(self) -> DigitalPayoutType: ...
    @property
    def payout_amount(self) -> Money: ...
    @property
    def expiry(self) -> date: ...
    @property
    def notional(self) -> Money: ...
    @property
    def domestic_discount_curve(self) -> str: ...
    @property
    def foreign_discount_curve(self) -> str: ...
    @property
    def vol_surface(self) -> str: ...
    @property
    def day_count(self) -> DayCount: ...
    def value(self, market: MarketContext, as_of: date) -> Money: ...
    def __repr__(self) -> str: ...
    def to_json(self) -> str:
        """Serialize to JSON in envelope format.

        Returns:
            str: JSON string with schema version and tagged instrument spec.
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> "Self":
        """Deserialize from JSON in envelope format.

        Args:
            json_str: JSON string in envelope format.

        Returns:
            The deserialized instrument.

        Raises:
            ValueError: If JSON is malformed or contains a different instrument type.
        """
        ...
