"""Equity option instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ..common import InstrumentType

class EquityOption:
    """Equity option instrument for pricing European and American options.

    EquityOption represents a call or put option on a single equity or equity
    index. Options are priced using Black-Scholes or similar models, requiring
    a discount curve, spot price, and volatility surface in the MarketContext.

    Options can be European (exercisable only at expiry) or American (exercisable
    at any time before expiry). The instrument supports standard market conventions
    and can be priced with various models (Black-Scholes, binomial, etc.).

    Examples
    --------
    Create a European call option:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import EquityOption
        >>> option = EquityOption.european_call(
        ...     "SPX-CALL-4500",
        ...     ticker="SPX",
        ...     strike=4500.0,
        ...     expiry=date(2024, 12, 20),
        ...     notional=Money(100_000, Currency("USD")),
        ...     contract_size=100.0,
        ... )

    Price the option:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.scalars import MarketScalar
        >>> from finstack.core.market_data.surfaces import VolSurface
        >>> from finstack.core.market_data.term_structures import DiscountCurve
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import EquityOption
        >>> from finstack.valuations.pricer import create_standard_registry
        >>> option = EquityOption.european_call(
        ...     "SPX-CALL-4500",
        ...     ticker="SPX",
        ...     strike=4500.0,
        ...     expiry=date(2024, 12, 20),
        ...     notional=Money(100_000, Currency("USD")),
        ...     contract_size=100.0,
        ... )
        >>> ctx = MarketContext()
        >>> ctx.insert_discount(DiscountCurve("USD-OIS", date(2024, 1, 1), [(0.0, 1.0), (1.0, 0.99)]))
        >>> expiries = [0.5, 1.0, 2.0]
        >>> strikes = [4000.0, 4500.0, 5000.0]
        >>> grid = [
        ...     [0.28, 0.27, 0.26],
        ...     [0.27, 0.26, 0.25],
        ...     [0.26, 0.25, 0.24],
        ... ]
        >>> ctx.insert_surface(VolSurface("EQUITY-VOL", expiries, strikes, grid))
        >>> spot_scalar = MarketScalar.price(Money(4400, Currency("USD")))
        >>> ctx.insert_price("SPX", spot_scalar)
        >>> ctx.insert_price("EQUITY-SPOT", spot_scalar)
        >>> registry = create_standard_registry()
        >>> pv = registry.price(option, "black76", ctx).value
        >>> pv.currency.code
        'USD'

    Notes
    -----
    - Options require discount curve, spot price, and volatility surface
    - Strike is in absolute terms (not moneyness)
    - Contract size multiplies the notional for position sizing
    - Dividend yield can be specified for dividend-paying stocks
    - Use :meth:`builder` for American options or custom configurations

    See Also
    --------
    :class:`Swaption`: Interest rate swaptions
    :class:`InterestRateOption`: Interest rate caps/floors
    :class:`PricerRegistry`: Pricing entry point
    """

    @classmethod
    def european_call(
        cls,
        instrument_id: str,
        ticker: str,
        strike: float,
        expiry: date,
        notional: Money,
        contract_size: Optional[float] = 1.0,
    ) -> "EquityOption":
        """Create a European call option with standard market conventions.

        Factory method for creating a European-style call option. The option gives
        the holder the right (but not obligation) to buy the underlying equity at
        the strike price on the expiry date.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the option (e.g., "SPX-CALL-4500-20241220").
        ticker : str
            Underlying equity ticker symbol (e.g., "SPX", "AAPL", "MSFT").
            Must match a spot price identifier in MarketContext.
        strike : float
            Strike price in absolute terms (e.g., 4500.0 for SPX, 150.0 for stock).
            Must be positive.
        expiry : date
            Option expiration date. The option can only be exercised on this date
            (European style).
        notional : Money
            Notional amount representing the position size. The currency should
            match the underlying equity's currency.
        contract_size : float, optional
            Number of shares per contract (default: 1.0). For standard equity
            options, this is typically 100. For index options, often 1.0.
            The effective notional is notional * contract_size.

        Returns
        -------
        EquityOption
            Configured European call option ready for pricing.

        Raises
        ------
        ValueError
            If strike <= 0, if expiry is in the past, or if notional amount
            is invalid.

        Examples
        --------
            >>> from finstack import Money, Currency
            >>> from datetime import date
            >>> option = EquityOption.european_call(
            ...     "AAPL-CALL-150",
            ...     ticker="AAPL",
            ...     strike=150.0,
            ...     expiry=date(2024, 12, 20),
            ...     notional=Money(10_000, Currency("USD")),
            ...     contract_size=100.0,
            ... )
            >>> option.strike
            Money(150.0, Currency("USD"))
            >>> option.option_type
            'call'
        """
        ...

    @classmethod
    def european_put(
        cls,
        instrument_id: str,
        ticker: str,
        strike: float,
        expiry: date,
        notional: Money,
        contract_size: Optional[float] = 1.0,
    ) -> "EquityOption":
        """Create a European put option with standard market conventions.

        Factory method for creating a European-style put option. The option gives
        the holder the right (but not obligation) to sell the underlying equity at
        the strike price on the expiry date.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the option (e.g., "SPX-PUT-4500-20241220").
        ticker : str
            Underlying equity ticker symbol. Must match a spot price identifier
            in MarketContext.
        strike : float
            Strike price in absolute terms. Must be positive.
        expiry : date
            Option expiration date (European exercise).
        notional : Money
            Notional amount representing the position size.
        contract_size : float, optional
            Number of shares per contract (default: 1.0). Typically 100 for
            equity options, 1.0 for index options.

        Returns
        -------
        EquityOption
            Configured European put option ready for pricing.

        Raises
        ------
        ValueError
            If strike <= 0, if expiry is invalid, or if notional is invalid.

        Examples
        --------
            >>> option = EquityOption.european_put(
            ...     "SPX-PUT-4500",
            ...     ticker="SPX",
            ...     strike=4500.0,
            ...     expiry=date(2024, 12, 20),
            ...     notional=Money(100_000, Currency("USD")),
            ... )
        """
        ...

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        ticker: str,
        strike: float,
        expiry: date,
        notional: Money,
        discount_curve: str,
        spot_id: str,
        vol_surface: str,
        *,
        div_yield_id: Optional[str] = None,
        contract_size: Optional[float] = 1.0,
    ) -> "EquityOption":
        """Create an equity option with explicit market data references.

        Builder method for creating equity options with full control over market
        data dependencies. Use this for American options, custom configurations,
        or when you need to specify exact curve/surface identifiers.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the option.
        ticker : str
            Underlying equity ticker symbol (for reference; spot comes from spot_id).
        strike : float
            Strike price in absolute terms. Must be positive.
        expiry : date
            Option expiration date.
        notional : Money
            Notional amount representing the position size.
        discount_curve : str
            Discount curve identifier in MarketContext for present value calculations.
        spot_id : str
            Spot price identifier in MarketContext (e.g., "SPX", "AAPL"). The
            MarketContext must contain a MarketScalar with this ID.
        vol_surface : str
            Volatility surface identifier in MarketContext. The surface must
            cover the option's expiry and strike.
        div_yield_id : str, optional
            Dividend yield identifier in MarketContext for dividend-paying stocks.
            If None, assumes no dividends. The yield should be a continuously
            compounded annual rate.
        contract_size : float, optional
            Number of shares per contract (default: 1.0).

        Returns
        -------
        EquityOption
            Configured option with explicit market data dependencies.

        Raises
        ------
        ValueError
            If parameters are invalid or if required market data identifiers
            are missing.

        Examples
        --------
        Option with dividend yield:

            >>> option = EquityOption.builder(
            ...     "AAPL-CALL-150",
            ...     ticker="AAPL",
            ...     strike=150.0,
            ...     expiry=date(2024, 12, 20),
            ...     notional=Money(10_000, Currency("USD")),
            ...     discount_curve="USD",
            ...     spot_id="AAPL",
            ...     vol_surface="AAPL-VOL",
            ...     div_yield_id="AAPL-DIV-YIELD",
            ... )

        Notes
        -----
        - Use factory methods (:meth:`european_call`, :meth:`european_put`) for
          simple European options
        - Use builder for American options or when you need explicit market data IDs
        - Volatility surface must cover the option's (expiry, strike) point
        - Dividend yield affects option pricing (reduces call value, increases put value)
        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def ticker(self) -> str: ...
    @property
    def strike(self) -> Money: ...
    @property
    def contract_size(self) -> float: ...
    @property
    def option_type(self) -> str: ...
    @property
    def exercise_style(self) -> str: ...
    @property
    def expiry(self) -> date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def vol_surface(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
