"""FX touch option (American binary option) instrument."""

from __future__ import annotations
from typing import Self

from datetime import date

from ....core.currency import Currency
from ....core.dates.daycount import DayCount
from ....core.market_data.context import MarketContext
from ....core.money import Money
from ...common import InstrumentType

class TouchType:
    """Touch option payoff style."""

    ONE_TOUCH: TouchType
    NO_TOUCH: TouchType
    @classmethod
    def from_name(cls, name: str) -> TouchType: ...
    @property
    def name(self) -> str: ...

class BarrierDirection:
    """Touch option barrier direction."""

    UP: BarrierDirection
    DOWN: BarrierDirection
    @classmethod
    def from_name(cls, name: str) -> BarrierDirection: ...
    @property
    def name(self) -> str: ...

class PayoutTiming:
    """Touch option payout timing."""

    AT_HIT: PayoutTiming
    AT_EXPIRY: PayoutTiming
    @classmethod
    def from_name(cls, name: str) -> PayoutTiming: ...
    @property
    def name(self) -> str: ...

class FxTouchOptionBuilder:
    """Fluent builder returned by :meth:`FxTouchOption.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def base_currency(self, ccy: str | Currency) -> FxTouchOptionBuilder: ...
    def quote_currency(self, ccy: str | Currency) -> FxTouchOptionBuilder: ...
    def barrier_level(self, level: float) -> FxTouchOptionBuilder: ...
    def touch_type(self, touch_type: str | TouchType) -> FxTouchOptionBuilder: ...
    def barrier_direction(self, direction: str | BarrierDirection) -> FxTouchOptionBuilder: ...
    def payout_amount(self, amount: Money) -> FxTouchOptionBuilder: ...
    def payout_timing(self, timing: str | PayoutTiming) -> FxTouchOptionBuilder: ...
    def expiry(self, date: date) -> FxTouchOptionBuilder: ...
    def domestic_discount_curve(self, curve_id: str) -> FxTouchOptionBuilder: ...
    def foreign_discount_curve(self, curve_id: str) -> FxTouchOptionBuilder: ...
    def vol_surface(self, surface_id: str) -> FxTouchOptionBuilder: ...
    def day_count(self, dc: DayCount) -> FxTouchOptionBuilder: ...
    def build(self) -> FxTouchOption: ...
    def __repr__(self) -> str: ...

class FxTouchOption:
    """FX touch option — American-style binary option triggered by barrier crossing.

    A touch option pays a fixed amount if the spot rate touches (one-touch)
    or never touches (no-touch) a barrier level at any time before expiry.
    Unlike European digital options, the monitoring is continuous
    (or near-continuous), making these path-dependent instruments.

    Payout can occur at the moment of touch or deferred to expiry:

    - **One-Touch / At Hit**: pays immediately when the barrier is crossed.
    - **One-Touch / At Expiry**: pays at expiry if the barrier was touched.
    - **No-Touch / At Expiry**: pays at expiry if the barrier was never touched.

    Pricing uses the analytic Garman-Kohlhagen barrier formula for
    continuously-monitored touch payoffs.

    Examples
    --------
    Build a EUR/USD one-touch option:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments.fx import FxTouchOption
        >>> opt = (
        ...     FxTouchOption
        ...     .builder("FX-TOUCH-001")
        ...     .base_currency("EUR")
        ...     .quote_currency("USD")
        ...     .barrier_level(1.15)
        ...     .barrier_direction("up")
        ...     .touch_type("one_touch")
        ...     .payout_amount(Money(100_000, Currency("USD")))
        ...     .payout_timing("at_expiry")
        ...     .expiry(date(2024, 12, 20))
        ...     .domestic_discount_curve("USD-OIS")
        ...     .foreign_discount_curve("EUR-OIS")
        ...     .vol_surface("EURUSD-VOL")
        ...     .build()
        ... )
        >>> opt.touch_type.name
        'OneTouch'

    Attributes
    ----------
    instrument_id : str
        Unique trade identifier.
    base_currency : Currency
        Foreign (base) currency of the pair.
    quote_currency : Currency
        Domestic (quote) currency of the pair.
    barrier_level : float
        Barrier level in quote-per-base terms.
    touch_type : TouchType
        ``ONE_TOUCH`` or ``NO_TOUCH``.
    barrier_direction : BarrierDirection
        ``UP`` (barrier above current spot) or ``DOWN`` (below current spot).
    payout_amount : Money
        Fixed cash payout on trigger.
    payout_timing : PayoutTiming
        ``AT_HIT`` (immediate) or ``AT_EXPIRY`` (deferred).
    expiry : date
        Option expiration / monitoring end date.
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
    :class:`FxDigitalOption` : FX European digital option.
    :class:`FxBarrierOption` : FX barrier option with strike payoff.
    :class:`FxOption` : FX vanilla option.

    Sources
    -------
    - Garman & Kohlhagen (1983): see ``docs/REFERENCES.md#garmanKohlhagen1983``.
    - Reiner & Rubinstein (1991) "Breaking Down the Barriers":
      see ``docs/REFERENCES.md#reinerRubinsteinBarrier1991``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> FxTouchOptionBuilder: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    @property
    def base_currency(self) -> Currency: ...
    @property
    def quote_currency(self) -> Currency: ...
    @property
    def barrier_level(self) -> float: ...
    @property
    def touch_type(self) -> TouchType: ...
    @property
    def barrier_direction(self) -> BarrierDirection: ...
    @property
    def payout_amount(self) -> Money: ...
    @property
    def payout_timing(self) -> PayoutTiming: ...
    @property
    def expiry(self) -> date: ...
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
