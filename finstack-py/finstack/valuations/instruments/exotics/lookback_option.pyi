"""Lookback option instrument."""

from __future__ import annotations
from datetime import date
from ....core.money import Money
from ....core.dates.daycount import DayCount

class LookbackType:
    """Lookback option type."""

    FIXED_STRIKE: "LookbackType"
    FLOATING_STRIKE: "LookbackType"
    @classmethod
    def from_name(cls, name: str) -> "LookbackType": ...
    @property
    def name(self) -> str: ...

class LookbackOption:
    """Lookback option with path-dependent payoff based on extreme prices.

    LookbackOption represents an option whose payoff depends on the maximum
    or minimum price reached during the option's life, rather than the spot
    price at expiry. This provides better payoffs but higher premiums.

    Lookback options are used for capturing best-case scenarios and hedging
    path-dependent exposures. They require discount curves, spot prices, and
    volatility surfaces.

    Examples
    --------
    Create a floating strike lookback call:

        >>> from finstack.valuations.instruments import LookbackOption, LookbackType
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> lookback = LookbackOption.builder(
        ...     "LOOKBACK-AAPL",
        ...     ticker="AAPL",
        ...     strike=None,
        ...     option_type="call",
        ...     lookback_type="floating_strike",
        ...     expiry=date(2024, 12, 20),
        ...     notional=Money(100_000, Currency("USD")),
        ...     discount_curve="USD",
        ...     spot_id="AAPL",
        ...     vol_surface="AAPL-VOL",
        ... )

    Notes
    -----
    - Lookback options require discount curve, spot price, and volatility surface
    - Fixed strike: payoff based on extreme price vs fixed strike
    - Floating strike: strike set to extreme price, payoff based on final price
    - Lookback options are more expensive than standard options
    - Path-dependent pricing requires more complex models

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required).
    - Spot price: ``spot_id`` (required).
    - Volatility surface: ``vol_surface`` (required).
    - Dividend yield: ``div_yield_id`` (optional; used when provided).

    See Also
    --------
    :class:`EquityOption`: Standard equity options
    :class:`BarrierOption`: Barrier options
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    """

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        ticker: str,
        strike: float | None,
        option_type: str,
        lookback_type: str,
        expiry: date,
        notional: Money,
        discount_curve: str,
        spot_id: str,
        vol_surface: str,
        *,
        div_yield_id: str | None = None,
        observed_min: Money | None = None,
        observed_max: Money | None = None,
    ) -> "LookbackOption":
        """Create a lookback option.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the option.
        ticker : str
            Underlying equity ticker symbol.
        strike : float, optional
            Strike price for fixed_strike type. None for floating_strike type.
        option_type : str
            Option type: "call" or "put".
        lookback_type : str
            Lookback type: "fixed_strike" or "floating_strike".
        expiry : date
            Option expiration date.
        notional : Money
            Notional amount.
        discount_curve : str
            Discount curve identifier in MarketContext.
        spot_id : str
            Spot price identifier in MarketContext.
        vol_surface : str
            Volatility surface identifier in MarketContext.
        div_yield_id : str, optional
            Dividend yield identifier in MarketContext.
        observed_min : Money, optional
            Observed minimum price for seasoned options.
        observed_max : Money, optional
            Observed maximum price for seasoned options.

        Returns
        -------
        LookbackOption
            Configured lookback option ready for pricing.

        Raises
        ------
        ValueError
            If parameters are invalid or if required market data is missing.

        Examples
        --------
            >>> option = LookbackOption.builder(
            ...     "LOOKBACK-AAPL",
            ...     "AAPL",
            ...     strike=None,
            ...     option_type="call",
            ...     lookback_type="floating_strike",
            ...     expiry=date(2024, 12, 20),
            ...     notional=Money(100_000, Currency("USD")),
            ...     discount_curve="USD",
            ...     spot_id="AAPL",
            ...     vol_surface="AAPL-VOL",
            ... )
        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def ticker(self) -> str: ...
    @property
    def strike(self) -> float | None: ...
    @property
    def option_type(self) -> str: ...
    @property
    def lookback_type(self) -> str: ...
    @property
    def expiry(self) -> date: ...
    @property
    def notional(self) -> Money: ...
    @property
    def day_count(self) -> DayCount: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def spot_id(self) -> str: ...
    @property
    def vol_surface(self) -> str: ...
    @property
    def div_yield_id(self) -> str | None: ...
    @property
    def observed_min(self) -> Money | None: ...
    @property
    def observed_max(self) -> Money | None: ...
    def __repr__(self) -> str: ...
