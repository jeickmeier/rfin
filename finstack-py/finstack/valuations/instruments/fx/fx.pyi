"""FX instruments."""

from __future__ import annotations
from datetime import date
from ....core.money import Money
from ....core.currency import Currency
from ....core.dates.calendar import BusinessDayConvention
from ...common import InstrumentType

class FxSpotBuilder:
    """Fluent builder returned by :meth:`FxSpot.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def base_currency(self, base_currency: Currency) -> "FxSpotBuilder": ...
    def quote_currency(self, quote_currency: Currency) -> "FxSpotBuilder": ...
    def settlement(self, settlement: date) -> "FxSpotBuilder": ...
    def settlement_lag_days(self, settlement_lag_days: int) -> "FxSpotBuilder": ...
    def spot_rate(self, spot_rate: float) -> "FxSpotBuilder": ...
    def notional(self, notional: Money) -> "FxSpotBuilder": ...
    def bdc(self, bdc: BusinessDayConvention) -> "FxSpotBuilder": ...
    def calendar(self, calendar: str | None = ...) -> "FxSpotBuilder": ...
    def build(self) -> "FxSpot": ...

class FxSpot:
    """FX spot transaction for immediate currency exchange.

    FxSpot represents a foreign exchange transaction where one currency is
    exchanged for another at the spot rate, typically with T+2 settlement
    (2 business days after trade date).

    FX spot transactions are used for currency conversion, hedging, and
    speculation. They are priced using FX rates from MarketContext.

    Examples
    --------
        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import FxSpot
        >>> fx_spot = (
        ...     FxSpot
        ...     .builder("FX-EURUSD-SPOT")
        ...     .base_currency(Currency("EUR"))
        ...     .quote_currency(Currency("USD"))
        ...     .notional(Money(1_000_000, Currency("EUR")))
        ...     .spot_rate(1.10)
        ...     .build()
        ... )

    Notes
    -----
    - FX spot requires FX rates in MarketContext (via FxMatrix)
    - Settlement is typically T+2 (2 business days)
    - Spot rate can be provided or derived from MarketContext
    - Notional is in base currency
    - Business day convention applies to settlement date

    Conventions
    -----------
    - FX rates are quoted as ``quote_currency per base_currency`` (e.g., EUR/USD = 1.10 means 1 EUR = 1.10 USD).
    - Settlement lag is expressed in business days; calendar/BDC parameters control settlement date adjustment.
    - Market FX is sourced from ``MarketContext`` (via ``FxMatrix``) when an explicit ``spot_rate`` is not provided.

    MarketContext Requirements
    -------------------------
    - FX rates: ``FxMatrix`` in ``MarketContext`` (required when ``spot_rate`` is not provided).

    See Also
    --------
    :class:`FxSwap`: FX swaps for forward exchange
    :class:`FxOption`: FX options for optional exchange
    :class:`FxMatrix`: FX rate matrix

    Sources
    -------
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> FxSpotBuilder:
        """Start a fluent builder (builder-only API).

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the FX spot (e.g., "FX-EURUSD-SPOT").
        base_currency : Currency
            Base currency of the FX pair (currency being sold).
        quote_currency : Currency
            Quote currency of the FX pair (currency being bought).
        settlement : date, optional
            Settlement date. If None, calculated from trade date + settlement_lag.
        settlement_lag_days : int, optional
            Number of business days between trade and settlement (default: 2 for T+2).
        spot_rate : float, optional
            Spot exchange rate (how many quote_currency per base_currency).
            If None, rate is retrieved from MarketContext.
        notional : Money, optional
            Notional amount in base currency. If None, uses a unit notional.
        bdc : BusinessDayConvention, optional
            Business day convention for settlement date adjustment.
        calendar : str, optional
            Holiday calendar identifier for business day calculations.

        Returns
        -------
        FxSpot
            Configured FX spot transaction ready for pricing.

        Raises
        ------
        ValueError
            If currencies are the same, if spot_rate is <= 0, or if dates
            are invalid.

        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def base_currency(self) -> Currency: ...
    @property
    def quote_currency(self) -> Currency: ...
    @property
    def notional(self) -> Money | None: ...
    @property
    def spot_rate(self) -> float | None: ...
    @property
    def settlement(self) -> date | None: ...
    @property
    def settlement_lag_days(self) -> int | None: ...
    @property
    def business_day_convention(self) -> str: ...
    @property
    def calendar_id(self) -> str | None: ...
    @property
    def pair_name(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class FxOption:
    """FX option for optional currency exchange at a fixed rate.

    FxOption represents an option to exchange one currency for another at
    a fixed strike rate on or before expiry. FX options are priced using
    the Garman-Kohlhagen model, requiring domestic and foreign discount
    curves and a volatility surface.

    FX options are used for hedging FX exposure, speculation, and creating
    structured products. They provide optionality on currency movements.

    Examples
    --------
    Create a European FX call option:

        >>> from finstack.valuations.instruments import FxOption
        >>> from finstack import Currency, Money
        >>> from datetime import date
        >>> fx_option = (
        ...     FxOption
        ...     .builder("FX-OPT-EURUSD-CALL")
        ...     .base_currency(Currency("EUR"))
        ...     .quote_currency(Currency("USD"))
        ...     .strike(1.10)  # EUR/USD strike
        ...     .expiry(date(2024, 12, 20))
        ...     .notional(Money(1_000_000, Currency("EUR")))
        ...     .domestic_curve("USD-OIS")
        ...     .foreign_curve("EUR-OIS")
        ...     .vol_surface("EURUSD-VOL")
        ...     .option_type("call")
        ...     .build()
        ... )

    Price the option:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.fx import FxMatrix
        >>> from finstack.core.market_data.surfaces import VolSurface
        >>> from finstack.core.market_data.term_structures import DiscountCurve
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import FxOption
        >>> from finstack.valuations.pricer import create_standard_registry
        >>> fx_option = (
        ...     FxOption
        ...     .builder("FX-OPT-EURUSD")
        ...     .base_currency(Currency("EUR"))
        ...     .quote_currency(Currency("USD"))
        ...     .strike(1.10)
        ...     .expiry(date(2024, 12, 20))
        ...     .notional(Money(1_000_000, Currency("EUR")))
        ...     .domestic_curve("USD-OIS")
        ...     .foreign_curve("EUR-OIS")
        ...     .vol_surface("EURUSD-VOL")
        ...     .option_type("call")
        ...     .build()
        ... )
        >>> ctx = MarketContext()
        >>> ctx.insert_discount(DiscountCurve("USD-OIS", date(2024, 1, 1), [(0.0, 1.0), (1.0, 0.99)]))
        >>> ctx.insert_discount(DiscountCurve("EUR-OIS", date(2024, 1, 1), [(0.0, 1.0), (1.0, 0.991)]))
        >>> expiries = [0.5, 1.0]
        >>> strikes = [0.95, 1.0, 1.05]
        >>> grid = [
        ...     [0.21, 0.20, 0.22],
        ...     [0.20, 0.19, 0.21],
        ... ]
        >>> ctx.insert_surface(VolSurface("EURUSD-VOL", expiries, strikes, grid))
        >>> fx_matrix = FxMatrix()
        >>> fx_matrix.set_quote(Currency("EUR"), Currency("USD"), 1.10)
        >>> ctx.insert_fx(fx_matrix)
        >>> registry = create_standard_registry()
        >>> result = registry.price(fx_option, "black76", ctx)
        >>> isinstance(result.value.amount, float)
        True

    Notes
    -----
    - FX options require domestic curve, foreign curve, and volatility surface
    - Strike is the exchange rate (quote_currency per base_currency)
    - Notional is in base currency
    - Settlement can be "cash" (default) or "physical"
    - Garman-Kohlhagen model accounts for both interest rate differentials

    Conventions
    -----------
    - Strike is quoted as ``quote_currency per base_currency``.
    - Volatilities in surfaces are expected as decimals.
    - Domestic/foreign curves correspond to quote/base currency respectively (see builder notes).
    - Settlement is specified by ``settlement`` ("cash" or "physical").

    MarketContext Requirements
    -------------------------
    - Discount curves: ``domestic_curve`` and ``foreign_curve`` (required by ``builder`` / pricing models).
    - Volatility surface: ``vol_surface`` (required).
    - FX spot: ``FxMatrix`` in ``MarketContext`` (required unless provided via alternate pricing configuration).

    See Also
    --------
    :class:`FxSpot`: FX spot transactions
    :class:`FxSwap`: FX swaps
    :class:`EquityOption`: Equity options

    Sources
    -------
    - Garman & Kohlhagen (1983): see ``docs/REFERENCES.md#garmanKohlhagen1983``.
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> "FxOptionBuilder":
        """Start a fluent builder (builder-only API)."""
        ...

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
    def domestic_curve(self) -> str: ...
    @property
    def foreign_curve(self) -> str: ...
    @property
    def vol_surface(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class FxOptionBuilder:
    """Fluent builder returned by :meth:`FxOption.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def base_currency(self, base_currency: Currency) -> "FxOptionBuilder": ...
    def quote_currency(self, quote_currency: Currency) -> "FxOptionBuilder": ...
    def strike(self, strike: float) -> "FxOptionBuilder": ...
    def expiry(self, expiry: date) -> "FxOptionBuilder": ...
    def notional(self, notional: Money) -> "FxOptionBuilder": ...
    def domestic_curve(self, domestic_curve: str) -> "FxOptionBuilder": ...
    def foreign_curve(self, foreign_curve: str) -> "FxOptionBuilder": ...
    def vol_surface(self, vol_surface: str) -> "FxOptionBuilder": ...
    def option_type(self, option_type: str) -> "FxOptionBuilder": ...
    def settlement(self, settlement: str) -> "FxOptionBuilder": ...
    def build(self) -> "FxOption": ...

class FxSwap:
    """FX swap for simultaneous spot and forward currency exchange.

    FxSwap represents a combination of a spot FX transaction and an offsetting
    forward FX transaction. The near leg exchanges currencies at the near_date
    (spot), and the far leg reverses the exchange at the far_date (forward).

    FX swaps are used for hedging FX exposure, managing currency positions,
    and creating synthetic currency deposits/loans. They lock in the forward
    exchange rate.

    Examples
    --------
    Create an FX swap:

        >>> from finstack.valuations.instruments import FxSwap
        >>> from finstack import Currency, Money
        >>> from datetime import date
        >>> fx_swap = (
        ...     FxSwap
        ...     .builder("FX-SWAP-EURUSD")
        ...     .base_currency(Currency("EUR"))
        ...     .quote_currency(Currency("USD"))
        ...     .notional(Money(1_000_000, Currency("EUR")))
        ...     .near_date(date(2024, 1, 3))  # Spot date (T+2)
        ...     .far_date(date(2024, 7, 3))  # 6-month forward
        ...     .domestic_curve("USD")
        ...     .foreign_curve("EUR")
        ...     .near_rate(1.10)  # Optional: spot rate
        ...     .far_rate(1.12)  # Optional: forward rate
        ...     .build()
        ... )

    Price the FX swap:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.fx import FxMatrix
        >>> from finstack.core.market_data.term_structures import DiscountCurve
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import FxSwap
        >>> from finstack.valuations.pricer import create_standard_registry
        >>> fx_swap = (
        ...     FxSwap
        ...     .builder("FX-SWAP-EURUSD")
        ...     .base_currency(Currency("EUR"))
        ...     .quote_currency(Currency("USD"))
        ...     .notional(Money(1_000_000, Currency("EUR")))
        ...     .near_date(date(2024, 1, 3))
        ...     .far_date(date(2024, 7, 3))
        ...     .domestic_curve("USD")
        ...     .foreign_curve("EUR")
        ...     .near_rate(1.10)
        ...     .far_rate(1.12)
        ...     .build()
        ... )
        >>> ctx = MarketContext()
        >>> ctx.insert_discount(DiscountCurve("USD", date(2024, 1, 1), [(0.0, 1.0), (0.5, 0.995)]))
        >>> ctx.insert_discount(DiscountCurve("EUR", date(2024, 1, 1), [(0.0, 1.0), (0.5, 0.996)]))
        >>> fx_matrix = FxMatrix()
        >>> fx_matrix.set_quote(Currency("EUR"), Currency("USD"), 1.10)
        >>> ctx.insert_fx(fx_matrix)
        >>> registry = create_standard_registry()
        >>> result = registry.price(fx_swap, "discounting", ctx)
        >>> result.value.currency.code
        'USD'

    Notes
    -----
    - FX swaps require domestic curve, foreign curve, and FX rates
    - Near leg is the spot transaction (typically T+2)
    - Far leg is the forward transaction (reverses the near leg)
    - Forward rate can be provided or derived from curves and spot rate
    - The swap effectively creates a synthetic deposit/loan in one currency

    Conventions
    -----------
    - FX rates are quoted as ``quote_currency per base_currency``.
    - Domestic/foreign curves correspond to quote/base currency respectively.
    - If ``near_rate``/``far_rate`` are omitted, the runtime derives rates using market FX and/or curve inputs.

    MarketContext Requirements
    -------------------------
    - Discount curves: ``domestic_curve`` and ``foreign_curve`` (required).
    - FX spot: ``FxMatrix`` in ``MarketContext`` (required when deriving rates).

    See Also
    --------
    :class:`FxSpot`: FX spot transactions
    :class:`FxOption`: FX options
    :class:`ForwardRateAgreement`: Interest rate FRAs

    Sources
    -------
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> "FxSwapBuilder":
        """Start a fluent builder (builder-only API)."""
        ...

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
    def domestic_curve(self) -> str: ...
    @property
    def foreign_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class FxSwapBuilder:
    """Fluent builder returned by :meth:`FxSwap.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def base_currency(self, base_currency: Currency) -> "FxSwapBuilder": ...
    def quote_currency(self, quote_currency: Currency) -> "FxSwapBuilder": ...
    def notional(self, notional: Money) -> "FxSwapBuilder": ...
    def near_date(self, near_date: date) -> "FxSwapBuilder": ...
    def far_date(self, far_date: date) -> "FxSwapBuilder": ...
    def domestic_curve(self, domestic_curve: str) -> "FxSwapBuilder": ...
    def foreign_curve(self, foreign_curve: str) -> "FxSwapBuilder": ...
    def near_rate(self, near_rate: float | None = ...) -> "FxSwapBuilder": ...
    def far_rate(self, far_rate: float | None = ...) -> "FxSwapBuilder": ...
    def build(self) -> "FxSwap": ...
