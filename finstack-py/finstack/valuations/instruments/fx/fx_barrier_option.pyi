"""FX barrier option instrument."""

from __future__ import annotations

from datetime import date

from ....core.currency import Currency
from ....core.dates.daycount import DayCount
from ....core.market_data.context import MarketContext
from ....core.money import Money
from ...common import InstrumentType

class FxBarrierOptionBuilder:
    """Fluent builder returned by :meth:`FxBarrierOption.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def base_currency(self, ccy: str | Currency) -> FxBarrierOptionBuilder: ...
    def quote_currency(self, ccy: str | Currency) -> FxBarrierOptionBuilder: ...
    def strike(self, strike: float) -> FxBarrierOptionBuilder: ...
    def barrier(self, barrier: float) -> FxBarrierOptionBuilder: ...
    def rebate(self, rebate: float) -> FxBarrierOptionBuilder: ...
    def option_type(self, option_type: str) -> FxBarrierOptionBuilder: ...
    def barrier_type(self, barrier_type: str) -> FxBarrierOptionBuilder: ...
    def expiry(self, date: date) -> FxBarrierOptionBuilder: ...
    def notional(self, notional: Money) -> FxBarrierOptionBuilder: ...
    def domestic_discount_curve(self, curve_id: str) -> FxBarrierOptionBuilder: ...
    def foreign_discount_curve(self, curve_id: str) -> FxBarrierOptionBuilder: ...
    def vol_surface(self, surface_id: str) -> FxBarrierOptionBuilder: ...
    def fx_spot_id(self, spot_id: str) -> FxBarrierOptionBuilder: ...
    def use_gobet_miri(self, flag: bool) -> FxBarrierOptionBuilder: ...
    def day_count(self, dc: DayCount) -> FxBarrierOptionBuilder: ...
    def build(self) -> FxBarrierOption: ...
    def __repr__(self) -> str: ...

class FxBarrierOption:
    """FX barrier option — vanilla option that activates or extinguishes at a barrier.

    An FX barrier option is a path-dependent option that either comes into
    existence (knock-in) or ceases to exist (knock-out) when the spot rate
    crosses the barrier level.  Common varieties include up-and-out calls,
    down-and-in puts, and double-barrier options.

    Pricing uses an analytic formula for single-barrier options (Reiner &
    Rubinstein), with optional Gobet-Miri continuous-monitoring correction
    for discretely-observed barriers.

    Examples
    --------
    Build a EUR/USD up-and-out call:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments.fx import FxBarrierOption
        >>> opt = (
        ...     FxBarrierOption
        ...     .builder("FX-BAR-001")
        ...     .base_currency("EUR")
        ...     .quote_currency("USD")
        ...     .notional(Money(1_000_000, Currency("EUR")))
        ...     .option_type("call")
        ...     .barrier_type("up_out")
        ...     .strike(1.10)
        ...     .barrier(1.15)
        ...     .expiry(date(2024, 12, 20))
        ...     .domestic_discount_curve("USD-OIS")
        ...     .foreign_discount_curve("EUR-OIS")
        ...     .vol_surface("EURUSD-VOL")
        ...     .build()
        ... )
        >>> opt.barrier_type
        'up_out'

    Attributes
    ----------
    instrument_id : str
        Unique trade identifier.
    base_currency : Currency
        Foreign (base) currency of the pair.
    quote_currency : Currency
        Domestic (quote) currency of the pair.
    notional : Money
        Notional in the base currency.
    strike : float
        Option strike expressed in quote-per-base terms.
    barrier : float
        Barrier level in quote-per-base terms.
    rebate : float or None
        Rebate paid on barrier breach (for knock-out options).
    option_type : str
        ``"call"`` or ``"put"``.
    barrier_type : str
        Barrier classification: ``"up_in"``, ``"up_out"``, ``"down_in"``,
        or ``"down_out"``.
    expiry : date
        Option expiration date.
    domestic_discount_curve : str
        Discount curve id for the domestic (quote-currency) leg.
    foreign_discount_curve : str
        Discount curve id for the foreign (base-currency) leg.
    vol_surface : str
        Volatility surface id.
    fx_spot_id : str or None
        Market data id for the FX spot rate.
    use_gobet_miri : bool
        If ``True``, applies the Gobet-Miri discrete-barrier correction.
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
    :class:`FxDigitalOption` : FX digital (binary) option.
    :class:`FxTouchOption` : FX touch (American binary) option.

    Sources
    -------
    - Reiner & Rubinstein (1991) "Breaking Down the Barriers":
      see ``docs/REFERENCES.md#reinerRubinsteinBarrier1991``.
    - Gobet (2001) "Euler Schemes and Half-Space Approximation":
      see ``docs/REFERENCES.md#gobetMiri2001``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> FxBarrierOptionBuilder: ...
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
    def barrier(self) -> float: ...
    @property
    def rebate(self) -> float | None: ...
    @property
    def option_type(self) -> str: ...
    @property
    def barrier_type(self) -> str: ...
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
    def fx_spot_id(self) -> str | None: ...
    @property
    def use_gobet_miri(self) -> bool: ...
    @property
    def day_count(self) -> DayCount: ...
    def value(self, market: MarketContext, as_of: date) -> Money: ...
    def __repr__(self) -> str: ...
