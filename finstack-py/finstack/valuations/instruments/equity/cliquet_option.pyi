"""Cliquet option instrument."""

from __future__ import annotations
from typing import List
from datetime import date
from ....core.money import Money

class CliquetOption:
    """Cliquet option with periodic resets and capped/floor returns.

    CliquetOption represents an option with periodic resets where returns
    are locked in at each reset date. Local caps/floors apply to each period,
    while global caps/floors apply to the total return.

    Cliquet options are used in structured products and provide downside
    protection with upside participation. They require discount curves,
    spot prices, and volatility surfaces.

    Examples
    --------
    Create a cliquet option:

        >>> from finstack.valuations.instruments import CliquetOption
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> reset_dates = [date(2024, 3, 31), date(2024, 6, 30), date(2024, 9, 30), date(2024, 12, 31)]
        >>> cliquet = CliquetOption.builder(
        ...     "CLIQUET-SPX",
        ...     ticker="SPX",
        ...     reset_dates=reset_dates,
        ...     local_cap=0.10,  # 10% cap per period (positional)
        ...     global_cap=0.30,  # 30% total cap (positional)
        ...     notional=Money(1_000_000, Currency("USD")),
        ...     discount_curve="USD",
        ...     spot_id="SPX",
        ...     vol_surface="SPX-VOL",
        ...     maturity=date(2024, 12, 31),
        ...     div_yield_id=None,
        ... )

    Notes
    -----
    - Cliquet options require discount curve, spot price, and volatility surface
    - Returns are calculated and locked at each reset date
    - Local caps/floors limit each period's return
    - Global caps/floors limit total return over all periods
    - Cliquets provide downside protection with upside participation

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
    """

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        ticker: str,
        reset_dates: List[date],
        local_cap: float,
        global_cap: float,
        notional: Money,
        discount_curve: str,
        spot_id: str,
        vol_surface: str,
        *,
        local_floor: float = 0.0,
        global_floor: float = 0.0,
        payoff_type: str | None = None,
        maturity: date | None = None,
        div_yield_id: str | None = None,
    ) -> "CliquetOption":
        """Create a cliquet option.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the option.
        ticker : str
            Underlying equity ticker symbol.
        reset_dates : List[date]
            Dates when returns are calculated and locked. Must be in ascending order.
        local_cap : float
            Local cap per period. E.g., 0.10 for 10% cap.
        global_cap : float
            Global cap on total return. E.g., 0.30 for 30% cap.
        notional : Money
            Notional amount.
        discount_curve : str
            Discount curve identifier in MarketContext.
        spot_id : str
            Spot price identifier in MarketContext.
        vol_surface : str
            Volatility surface identifier in MarketContext.
        local_floor : float, optional
            Local floor per period (default: 0.0). E.g., -0.05 for -5% floor.
        global_floor : float, optional
            Global floor on total return (default: 0.0). E.g., 0.0 for 0% floor.
        payoff_type : str, optional
            Payoff aggregation type: "additive" (default) or "multiplicative".
        maturity : date, optional
            Option maturity date (defaults to last reset date).
        div_yield_id : str, optional
            Dividend yield identifier in MarketContext.

        Returns
        -------
        CliquetOption
            Configured cliquet option ready for pricing.

        Raises
        ------
        ValueError
            If parameters are invalid (reset_dates out of order, maturity < last reset,
            etc.) or if required market data is missing.

        Examples
        --------
            >>> option = CliquetOption.builder(
            ...     "CLIQUET-SPX",
            ...     "SPX",
            ...     reset_dates,
            ...     0.10,
            ...     0.30,
            ...     Money(1_000_000, Currency("USD")),
            ...     discount_curve="USD",
            ...     spot_id="SPX",
            ...     vol_surface="SPX-VOL",
            ...     local_floor=-0.05,
            ...     global_floor=0.0,
            ...     payoff_type="additive",
            ... )
        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def ticker(self) -> str: ...
    @property
    def local_cap(self) -> float: ...
    @property
    def global_cap(self) -> float: ...
    @property
    def notional(self) -> Money: ...
    @property
    def reset_dates(self) -> List[date]: ...
    @property
    def expiry(self) -> date: ...
    @property
    def local_floor(self) -> float: ...
    @property
    def global_floor(self) -> float: ...
    @property
    def payoff_type(self) -> str: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
