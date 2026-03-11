"""FX spot, option, and swap instrument wrappers."""

from __future__ import annotations

from datetime import date

from ....core.currency import Currency
from ....core.dates.daycount import DayCount
from ....core.market_data.context import MarketContext
from ....core.money import Money
from ...common import InstrumentType

class FxSpotBuilder:
    """Fluent builder returned by :meth:`FxSpot.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def base_currency(self, ccy: str | Currency) -> FxSpotBuilder: ...
    def quote_currency(self, ccy: str | Currency) -> FxSpotBuilder: ...
    def settlement(self, settlement: date) -> FxSpotBuilder: ...
    def settlement_lag_days(self, settlement_lag_days: int) -> FxSpotBuilder: ...
    def spot_rate(self, spot_rate: float) -> FxSpotBuilder: ...
    def notional(self, notional: Money) -> FxSpotBuilder: ...
    def bdc(self, bdc: str) -> FxSpotBuilder: ...
    def base_calendar(self, calendar_id: str) -> FxSpotBuilder: ...
    def quote_calendar(self, calendar_id: str) -> FxSpotBuilder: ...
    def build(self) -> FxSpot: ...
    def __repr__(self) -> str: ...

class FxSpot:
    """FX spot transaction for exchanging currencies at the prevailing spot rate.

    FxSpot models a single-dated foreign exchange transaction where one
    currency is bought and another is sold at the market spot rate.
    Settlement typically occurs T+2 (T+1 for certain pairs such as
    USD/CAD, USD/MXN, and USD/TRY).

    The instrument can either carry a fixed ``spot_rate`` or resolve the
    rate from a :class:`MarketContext` at pricing time.

    Examples
    --------
    Build and value an FX spot trade:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments.fx import FxSpot
        >>> spot = (
        ...     FxSpot
        ...     .builder("FX-SPOT-001")
        ...     .base_currency("EUR")
        ...     .quote_currency("USD")
        ...     .notional(Money(1_000_000, Currency("EUR")))
        ...     .spot_rate(1.0850)
        ...     .settlement(date(2024, 6, 5))
        ...     .build()
        ... )
        >>> spot.pair_name
        'EURUSD'

    Attributes
    ----------
    instrument_id : str
        Unique trade identifier.
    base_currency : Currency
        Currency being bought (left side of the pair).
    quote_currency : Currency
        Currency being sold (right side of the pair).
    notional : Money
        Notional amount in the base currency.
    spot_rate : float or None
        Contracted spot rate; ``None`` when resolved from market data.
    settlement : date or None
        Explicit settlement date, if set.
    settlement_lag_days : int or None
        Number of business days from trade date to settlement (default T+2).
    pair_name : str
        ISO pair string, e.g. ``"EURUSD"``.

    MarketContext Requirements
    -------------------------
    - FX spot rate for the currency pair (if ``spot_rate`` is not fixed).

    See Also
    --------
    :class:`FxSwap` : FX swap (near + far legs).
    :class:`FxOption` : FX vanilla option (Garman-Kohlhagen).
    """

    @classmethod
    def builder(cls, instrument_id: str) -> FxSpotBuilder: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def base_currency(self) -> Currency: ...
    @property
    def quote_currency(self) -> Currency: ...
    @property
    def notional(self) -> Money: ...
    @property
    def spot_rate(self) -> float | None: ...
    @property
    def settlement(self) -> date | None: ...
    @property
    def settlement_lag_days(self) -> int | None: ...
    @property
    def business_day_convention(self) -> str: ...
    @property
    def base_calendar(self) -> str | None: ...
    @property
    def quote_calendar(self) -> str | None: ...
    @property
    def pair_name(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def value(self, market: MarketContext, as_of: date) -> Money: ...
    def effective_settlement_date(self, as_of: date) -> date: ...
    def is_t1_pair(self) -> bool: ...
    def __repr__(self) -> str: ...

class FxOptionBuilder:
    """Fluent builder returned by :meth:`FxOption.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def base_currency(self, ccy: str | Currency) -> FxOptionBuilder: ...
    def quote_currency(self, ccy: str | Currency) -> FxOptionBuilder: ...
    def strike(self, strike: float) -> FxOptionBuilder: ...
    def expiry(self, expiry: date) -> FxOptionBuilder: ...
    def notional(self, notional: Money) -> FxOptionBuilder: ...
    def domestic_discount_curve(self, curve_id: str) -> FxOptionBuilder: ...
    def foreign_discount_curve(self, curve_id: str) -> FxOptionBuilder: ...
    def vol_surface(self, surface_id: str) -> FxOptionBuilder: ...
    def option_type(self, option_type: str) -> FxOptionBuilder: ...
    def exercise_style(self, exercise_style: str) -> FxOptionBuilder: ...
    def settlement(self, settlement: str) -> FxOptionBuilder: ...
    def day_count(self, dc: DayCount) -> FxOptionBuilder: ...
    def build(self) -> FxOption: ...
    def __repr__(self) -> str: ...

class FxOption:
    """Vanilla FX option priced with the Garman-Kohlhagen model.

    FxOption represents a European or American option on a foreign
    exchange rate.  Pricing follows the Garman-Kohlhagen extension of
    Black-Scholes, treating the foreign interest rate as a continuous
    dividend yield.

    Examples
    --------
    Build a EUR/USD call option:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments.fx import FxOption
        >>> option = (
        ...     FxOption
        ...     .builder("FX-OPT-001")
        ...     .base_currency("EUR")
        ...     .quote_currency("USD")
        ...     .notional(Money(1_000_000, Currency("EUR")))
        ...     .strike(1.10)
        ...     .expiry(date(2024, 12, 20))
        ...     .option_type("call")
        ...     .domestic_discount_curve("USD")
        ...     .foreign_discount_curve("EUR")
        ...     .vol_surface("EURUSD-VOL")
        ...     .build()
        ... )

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
    expiry : date
        Expiration date of the option.
    option_type : str
        ``"call"`` or ``"put"``.
    exercise_style : str
        ``"european"`` or ``"american"``.
    settlement : str
        Settlement convention (e.g., ``"physical"``, ``"cash"``).
    domestic_discount_curve : str
        Curve id for the domestic (quote-currency) discount curve.
    foreign_discount_curve : str
        Curve id for the foreign (base-currency) discount curve.
    vol_surface : str
        Volatility surface id in MarketContext.

    MarketContext Requirements
    -------------------------
    - Domestic discount curve (quote currency).
    - Foreign discount curve (base currency).
    - FX volatility surface for the pair.
    - FX spot rate (for delta-strike conversion and forward calculation).

    See Also
    --------
    :class:`FxSpot` : FX spot transaction.
    :class:`FxSwap` : FX swap (near + far legs).

    Sources
    -------
    - Garman & Kohlhagen (1983): see ``docs/REFERENCES.md#garmanKohlhagen1983``.
    - Clark (2011) *Foreign Exchange Option Pricing*: see ``docs/REFERENCES.md#clarkFxOptions2011``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> FxOptionBuilder: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def base_currency(self) -> Currency: ...
    @property
    def quote_currency(self) -> Currency: ...
    @property
    def notional(self) -> Money: ...
    @property
    def strike(self) -> float: ...
    @property
    def expiry(self) -> date: ...
    @property
    def option_type(self) -> str: ...
    @property
    def exercise_style(self) -> str: ...
    @property
    def settlement(self) -> str: ...
    @property
    def domestic_discount_curve(self) -> str: ...
    @property
    def foreign_discount_curve(self) -> str: ...
    @property
    def vol_surface(self) -> str: ...
    @property
    def day_count(self) -> DayCount: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def value(self, market: MarketContext, as_of: date) -> Money: ...
    def implied_vol(
        self,
        market: MarketContext,
        as_of: date,
        target_price: float,
        initial_guess: float | None = None,
    ) -> float: ...
    @staticmethod
    def atm_forward_strike(spot: float, df_domestic: float, df_foreign: float) -> float: ...
    @staticmethod
    def atm_dns_strike(forward: float, vol: float, time_to_expiry: float, use_forward_delta: bool) -> float: ...
    def __repr__(self) -> str: ...

class FxSwapBuilder:
    """Fluent builder returned by :meth:`FxSwap.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def base_currency(self, ccy: str | Currency) -> FxSwapBuilder: ...
    def quote_currency(self, ccy: str | Currency) -> FxSwapBuilder: ...
    def notional(self, notional: Money) -> FxSwapBuilder: ...
    def near_date(self, near_date: date) -> FxSwapBuilder: ...
    def far_date(self, far_date: date) -> FxSwapBuilder: ...
    def domestic_discount_curve(self, curve_id: str) -> FxSwapBuilder: ...
    def foreign_discount_curve(self, curve_id: str) -> FxSwapBuilder: ...
    def near_rate(self, near_rate: float | None = None) -> FxSwapBuilder: ...
    def far_rate(self, far_rate: float | None = None) -> FxSwapBuilder: ...
    def base_calendar(self, calendar_id: str) -> FxSwapBuilder: ...
    def quote_calendar(self, calendar_id: str) -> FxSwapBuilder: ...
    def build(self) -> FxSwap: ...
    def __repr__(self) -> str: ...

class FxSwap:
    """FX swap consisting of a near-leg and a far-leg currency exchange.

    An FX swap is a simultaneous purchase and sale of identical amounts
    of one currency for another with two different value dates.  The near
    leg is typically spot while the far leg is a forward date.  The
    difference between the two exchange rates (the swap points) reflects
    the interest-rate differential between the two currencies.

    FX swaps are the most traded instrument in the global FX market and
    are widely used for funding, hedging, and rolling forward positions.

    Examples
    --------
    Build a 3-month EUR/USD FX swap:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments.fx import FxSwap
        >>> swap = (
        ...     FxSwap
        ...     .builder("FX-SWAP-001")
        ...     .base_currency("EUR")
        ...     .quote_currency("USD")
        ...     .notional(Money(5_000_000, Currency("EUR")))
        ...     .near_date(date(2024, 6, 5))
        ...     .far_date(date(2024, 9, 5))
        ...     .domestic_discount_curve("USD")
        ...     .foreign_discount_curve("EUR")
        ...     .build()
        ... )

    Attributes
    ----------
    instrument_id : str
        Unique trade identifier.
    base_currency : Currency
        Foreign (base) currency of the pair.
    quote_currency : Currency
        Domestic (quote) currency of the pair.
    base_notional : Money
        Notional amount in the base currency.
    near_date : date
        Settlement date of the near (spot) leg.
    far_date : date
        Settlement date of the far (forward) leg.
    near_rate : float or None
        Exchange rate for the near leg; resolved from market data when ``None``.
    far_rate : float or None
        Exchange rate for the far leg; resolved from market data when ``None``.
    domestic_discount_curve : str
        Curve id for the domestic (quote-currency) discount curve.
    foreign_discount_curve : str
        Curve id for the foreign (base-currency) discount curve.

    MarketContext Requirements
    -------------------------
    - Domestic discount curve (quote currency).
    - Foreign discount curve (base currency).
    - FX spot rate for the pair (when near/far rates are not fixed).

    See Also
    --------
    :class:`FxSpot` : FX spot transaction.
    :class:`FxOption` : FX vanilla option (Garman-Kohlhagen).
    """

    @classmethod
    def builder(cls, instrument_id: str) -> FxSwapBuilder: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def base_currency(self) -> Currency: ...
    @property
    def quote_currency(self) -> Currency: ...
    @property
    def base_notional(self) -> Money: ...
    @property
    def near_date(self) -> date: ...
    @property
    def far_date(self) -> date: ...
    @property
    def near_rate(self) -> float | None: ...
    @property
    def far_rate(self) -> float | None: ...
    @property
    def domestic_discount_curve(self) -> str: ...
    @property
    def foreign_discount_curve(self) -> str: ...
    @property
    def base_calendar(self) -> str | None: ...
    @property
    def quote_calendar(self) -> str | None: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def value(self, market: MarketContext, as_of: date) -> Money: ...
    def __repr__(self) -> str: ...
