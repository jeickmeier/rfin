"""FX instruments."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.currency import Currency
from ...core.dates.calendar import BusinessDayConvention
from ..common import InstrumentType

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
        >>> fx_spot = FxSpot.create(
        ...     "FX-EURUSD-SPOT",
        ...     base_currency=Currency("EUR"),
        ...     quote_currency=Currency("USD"),
        ...     notional=Money(1_000_000, Currency("EUR")),
        ...     spot_rate=1.10,
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
    def create(
        cls,
        instrument_id: str,
        base_currency: Currency,
        quote_currency: Currency,
        *,
        settlement: Optional[date] = None,
        settlement_lag_days: Optional[int] = None,
        spot_rate: Optional[float] = None,
        notional: Optional[Money] = None,
        bdc: Optional[BusinessDayConvention] = None,
        calendar: Optional[str] = None,
    ) -> "FxSpot":
        """Create an FX spot position with optional settlement overrides.

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

        Examples
        --------
            >>> from finstack import Currency, Money
            >>> fx_spot = FxSpot.create(
            ...     "FX-EURUSD",
            ...     Currency("EUR"),
            ...     Currency("USD"),
            ...     notional=Money(1_000_000, Currency("EUR")),
            ...     spot_rate=1.10,
            ... )
            >>> fx_spot.pair_name
            'EURUSD'
        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def base_currency(self) -> Currency: ...
    @property
    def quote_currency(self) -> Currency: ...
    @property
    def notional(self) -> Optional[Money]: ...
    @property
    def spot_rate(self) -> Optional[float]: ...
    @property
    def settlement(self) -> Optional[date]: ...
    @property
    def settlement_lag_days(self) -> Optional[int]: ...
    @property
    def business_day_convention(self) -> str: ...
    @property
    def calendar_id(self) -> Optional[str]: ...
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
        >>> fx_option = FxOption.european_call(
        ...     "FX-OPT-EURUSD-CALL",
        ...     base_currency=Currency("EUR"),
        ...     quote_currency=Currency("USD"),
        ...     strike=1.10,  # EUR/USD strike
        ...     expiry=date(2024, 12, 20),
        ...     notional=Money(1_000_000, Currency("EUR")),
        ...     vol_surface="EURUSD-VOL",
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
        >>> fx_option = FxOption.european_call(
        ...     "FX-OPT-EURUSD",
        ...     Currency("EUR"),
        ...     Currency("USD"),
        ...     strike=1.10,
        ...     expiry=date(2024, 12, 20),
        ...     notional=Money(1_000_000, Currency("EUR")),
        ...     vol_surface="EURUSD-VOL",
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
    def european_call(
        cls,
        instrument_id: str,
        base_currency: Currency,
        quote_currency: Currency,
        strike: float,
        expiry: date,
        notional: Money,
        vol_surface: str,
    ) -> "FxOption":
        """Create a European call option with explicit volatility surface.

        A call option gives the holder the right to buy base_currency (sell
        quote_currency) at the strike rate. The option can only be exercised
        at expiry (European style).

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the option (e.g., "FX-OPT-EURUSD-CALL").
        base_currency : Currency
            Base currency of the FX pair (currency being bought if exercised).
        quote_currency : Currency
            Quote currency of the FX pair (currency being sold if exercised).
        strike : float
            Strike exchange rate (quote_currency per base_currency). Must be > 0.
        expiry : date
            Option expiration date (European exercise only).
        notional : Money
            Notional amount in base currency.
        vol_surface : str
            Volatility surface identifier in MarketContext for FX option pricing.

        Returns
        -------
        FxOption
            Configured European FX call option ready for pricing.

        Raises
        ------
        ValueError
            If strike <= 0, if expiry is invalid, or if currencies are the same.

        Examples
        --------
            >>> from finstack import Currency, Money
            >>> from datetime import date
            >>> option = FxOption.european_call(
            ...     "FX-OPT-EURUSD",
            ...     Currency("EUR"),
            ...     Currency("USD"),
            ...     strike=1.10,
            ...     expiry=date(2024, 12, 20),
            ...     notional=Money(1_000_000, Currency("EUR")),
            ...     vol_surface="EURUSD-VOL",
            ... )
        """
        ...

    @classmethod
    def european_put(
        cls,
        instrument_id: str,
        base_currency: Currency,
        quote_currency: Currency,
        strike: float,
        expiry: date,
        notional: Money,
        vol_surface: str,
    ) -> "FxOption":
        """Create a European put option with explicit volatility surface.

        A put option gives the holder the right to sell base_currency (buy
        quote_currency) at the strike rate. The option can only be exercised
        at expiry (European style).

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the option.
        base_currency : Currency
            Base currency of the FX pair (currency being sold if exercised).
        quote_currency : Currency
            Quote currency of the FX pair (currency being bought if exercised).
        strike : float
            Strike exchange rate (quote_currency per base_currency). Must be > 0.
        expiry : date
            Option expiration date (European exercise only).
        notional : Money
            Notional amount in base currency.
        vol_surface : str
            Volatility surface identifier in MarketContext.

        Returns
        -------
        FxOption
            Configured European FX put option ready for pricing.

        Raises
        ------
        ValueError
            If parameters are invalid.

        Examples
        --------
            >>> option = FxOption.european_put(
            ...     "FX-OPT-EURUSD-PUT",
            ...     Currency("EUR"),
            ...     Currency("USD"),
            ...     strike=1.10,
            ...     expiry=date(2024, 12, 20),
            ...     notional=Money(1_000_000, Currency("EUR")),
            ...     vol_surface="EURUSD-VOL",
            ... )
        """
        ...

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        base_currency: Currency,
        quote_currency: Currency,
        strike: float,
        expiry: date,
        notional: Money,
        domestic_curve: str,
        foreign_curve: str,
        vol_surface: str,
        *,
        settlement: Optional[str] = "cash",
    ) -> "FxOption":
        """Create an FX option with explicit domestic/foreign curves and vol surface.

        Builder method for creating FX options with full control over market data
        dependencies. Use this when you need to specify exact curve identifiers
        or customize settlement.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the option.
        base_currency : Currency
            Base currency of the FX pair.
        quote_currency : Currency
            Quote currency of the FX pair.
        strike : float
            Strike exchange rate (quote_currency per base_currency). Must be > 0.
        expiry : date
            Option expiration date.
        notional : Money
            Notional amount in base currency.
        domestic_curve : str
            Domestic (quote currency) discount curve identifier in MarketContext.
        foreign_curve : str
            Foreign (base currency) discount curve identifier in MarketContext.
        vol_surface : str
            Volatility surface identifier in MarketContext.
        settlement : str, optional
            Settlement type: "cash" (default, cash settlement) or "physical"
            (physical currency exchange).

        Returns
        -------
        FxOption
            Configured FX option with explicit market data dependencies.

        Raises
        ------
        ValueError
            If parameters are invalid.

        Examples
        --------
            >>> option = FxOption.builder(
            ...     "FX-OPT-EURUSD",
            ...     Currency("EUR"),
            ...     Currency("USD"),
            ...     strike=1.10,
            ...     expiry=date(2024, 12, 20),
            ...     notional=Money(1_000_000, Currency("EUR")),
            ...     domestic_curve="USD",  # USD is domestic
            ...     foreign_curve="EUR",  # EUR is foreign
            ...     vol_surface="EURUSD-VOL",
            ... )

        Notes
        -----
        - Domestic curve is for the quote currency (USD in EUR/USD)
        - Foreign curve is for the base currency (EUR in EUR/USD)
        - Garman-Kohlhagen model uses both curves for pricing
        - Cash settlement pays the option's intrinsic value
        - Physical settlement exchanges currencies at strike rate
        """
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
        >>> fx_swap = FxSwap.create(
        ...     "FX-SWAP-EURUSD",
        ...     base_currency=Currency("EUR"),
        ...     quote_currency=Currency("USD"),
        ...     notional=Money(1_000_000, Currency("EUR")),
        ...     near_date=date(2024, 1, 3),  # Spot date (T+2)
        ...     far_date=date(2024, 7, 3),  # 6-month forward
        ...     domestic_curve="USD",
        ...     foreign_curve="EUR",
        ...     near_rate=1.10,  # Optional: spot rate
        ...     far_rate=1.12,  # Optional: forward rate
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
        >>> fx_swap = FxSwap.create(
        ...     "FX-SWAP-EURUSD",
        ...     base_currency=Currency("EUR"),
        ...     quote_currency=Currency("USD"),
        ...     notional=Money(1_000_000, Currency("EUR")),
        ...     near_date=date(2024, 1, 3),
        ...     far_date=date(2024, 7, 3),
        ...     domestic_curve="USD",
        ...     foreign_curve="EUR",
        ...     near_rate=1.10,
        ...     far_rate=1.12,
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
    def create(
        cls,
        instrument_id: str,
        base_currency: Currency,
        quote_currency: Currency,
        notional: Money,
        near_date: date,
        far_date: date,
        domestic_curve: str,
        foreign_curve: str,
        *,
        near_rate: Optional[float] = None,
        far_rate: Optional[float] = None,
    ) -> "FxSwap":
        """Create an FX swap specifying near/far legs and associated curves.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the FX swap (e.g., "FX-SWAP-EURUSD-6M").
        base_currency : Currency
            Base currency of the FX pair (currency exchanged on near_date).
        quote_currency : Currency
            Quote currency of the FX pair (currency received on near_date).
        notional : Money
            Notional amount in base currency.
        near_date : date
            Near leg settlement date (spot date, typically T+2).
        far_date : date
            Far leg settlement date (forward date, reverses the near leg).
            Must be after near_date.
        domestic_curve : str
            Domestic (quote currency) discount curve identifier in MarketContext.
        foreign_curve : str
            Foreign (base currency) discount curve identifier in MarketContext.
        near_rate : float, optional
            Near leg (spot) exchange rate. If None, retrieved from MarketContext.
        far_rate : float, optional
            Far leg (forward) exchange rate. If None, calculated from curves
            and spot rate using interest rate parity.

        Returns
        -------
        FxSwap
            Configured FX swap ready for pricing.

        Raises
        ------
        ValueError
            If dates are invalid (far_date <= near_date), if rates are <= 0,
            or if currencies are the same.

        Examples
        --------
            >>> from finstack import Currency, Money
            >>> from datetime import date
            >>> fx_swap = FxSwap.create(
            ...     "FX-SWAP-EURUSD-6M",
            ...     Currency("EUR"),
            ...     Currency("USD"),
            ...     Money(1_000_000, Currency("EUR")),
            ...     near_date=date(2024, 1, 3),
            ...     far_date=date(2024, 7, 3),
            ...     domestic_curve="USD",
            ...     foreign_curve="EUR",
            ...     near_rate=1.10,
            ...     far_rate=1.12,
            ... )
        """
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
    def near_rate(self) -> Optional[float]: ...
    @property
    def far_rate(self) -> Optional[float]: ...
    @property
    def domestic_curve(self) -> str: ...
    @property
    def foreign_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
