"""Barrier option instrument."""

from __future__ import annotations
from datetime import date
from ...core.money import Money

class BarrierType:
    """Barrier type enumeration."""

    UP_AND_OUT: "BarrierType"
    UP_AND_IN: "BarrierType"
    DOWN_AND_OUT: "BarrierType"
    DOWN_AND_IN: "BarrierType"

    @classmethod
    def from_name(cls, name: str) -> "BarrierType": ...
    @property
    def name(self) -> str: ...

class BarrierOption:
    """Barrier option with path-dependent payoff.

    BarrierOption represents an option whose payoff depends on whether the
    underlying price crosses a barrier level during the option's life. Barrier
    options are cheaper than standard options but have path-dependent payoffs.

    Barrier options are used for cost-effective hedging and volatility trading.
    They require discount curves, spot prices, and volatility surfaces.

    Examples
    --------
    Create a down-and-out call barrier option:

        >>> from finstack.valuations.instruments import BarrierOption, BarrierType
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> barrier_option = BarrierOption.builder(
        ...     "BARRIER-AAPL-DO-CALL",
        ...     ticker="AAPL",
        ...     strike=150.0,
        ...     barrier=140.0,  # Barrier level
        ...     option_type="call",
        ...     barrier_type="down_and_out",  # Knocked out if price goes below barrier
        ...     expiry=date(2024, 12, 20),
        ...     notional=Money(100_000, Currency("USD")),
        ...     discount_curve="USD",
        ...     spot_id="AAPL",
        ...     vol_surface="AAPL-VOL",
        ...     div_yield_id=None,
        ...     use_gobet_miri=False,
        ... )

    Notes
    -----
    - Barrier options require discount curve, spot price, and volatility surface
    - Barrier types: "up_and_out", "up_and_in", "down_and_out", "down_and_in"
    - Out options are knocked out if barrier is crossed
    - In options only pay if barrier is crossed
    - Barrier options are typically cheaper than standard options

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required).
    - Spot price: ``spot_id`` (required).
    - Volatility surface: ``vol_surface`` (required).
    - Dividend yield: ``div_yield_id`` (optional; used when provided).

    See Also
    --------
    :class:`EquityOption`: Standard equity options
    :class:`AsianOption`: Asian options
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    - Gobet (2009): see ``docs/REFERENCES.md#gobet2009BarrierMC``.
    """

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        ticker: str,
        strike: float,
        barrier: float,
        option_type: str,
        barrier_type: str,
        expiry: date,
        notional: Money,
        discount_curve: str,
        spot_id: str,
        vol_surface: str,
        *,
        div_yield_id: str | None = None,
        use_gobet_miri: bool | None = False,
    ) -> "BarrierOption":
        """Create a barrier option.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the option.
        ticker : str
            Underlying equity ticker symbol.
        strike : float
            Strike price. Must be > 0.
        barrier : float
            Barrier level. Must be > 0 and typically different from strike.
        option_type : str
            Option type: "call" or "put".
        barrier_type : str
            Barrier type: "up_and_out", "up_and_in", "down_and_out", "down_and_in".
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
        use_gobet_miri : bool, optional
            Use Gobet-Miri approximation for barrier pricing (default: False).

        Returns
        -------
        BarrierOption
            Configured barrier option ready for pricing.

        Raises
        ------
        ValueError
            If parameters are invalid or if required market data is missing.

        Examples
        --------
            >>> option = BarrierOption.builder(
            ...     "BARRIER-AAPL",
            ...     "AAPL",
            ...     strike=150.0,
            ...     barrier=140.0,
            ...     option_type="call",
            ...     barrier_type="down_and_out",
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
    def strike(self) -> Money: ...
    @property
    def barrier(self) -> Money: ...
    @property
    def option_type(self) -> str: ...
    @property
    def barrier_type(self) -> str: ...
    @property
    def expiry(self) -> date: ...
    @property
    def notional(self) -> Money: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
