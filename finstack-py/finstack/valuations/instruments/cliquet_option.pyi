"""Cliquet option instrument."""

from typing import List, Optional
from datetime import date
from ...core.money import Money

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
        ...     div_yield_id=None,
        ... )

    Notes
    -----
    - Cliquet options require discount curve, spot price, and volatility surface
    - Returns are calculated and locked at each reset date
    - Local caps/floors limit each period's return
    - Global caps/floors limit total return over all periods
    - Cliquets provide downside protection with upside participation

    See Also
    --------
    :class:`EquityOption`: Standard equity options
    :class:`AsianOption`: Asian options
    :class:`PricerRegistry`: Pricing entry point
    """

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        underlying_ticker: str,
        reset_dates: List[date],
        maturity: date,
        notional: Money,
        discount_curve: str,
        vol_surface: str,
        spot_id: str,
        *,
        local_cap: float = 0.0,
        local_floor: float = 0.0,
        global_cap: float = 0.0,
        global_floor: float = 0.0,
        div_yield_id: Optional[str] = None,
    ) -> "CliquetOption":
        """Create a cliquet option.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the option.
        underlying_ticker : str
            Underlying equity ticker symbol.
        reset_dates : List[date]
            Dates when returns are calculated and locked. Must be in ascending order.
        maturity : date
            Option maturity date (should be >= last reset date).
        notional : Money
            Notional amount.
        discount_curve : str
            Discount curve identifier in MarketContext.
        vol_surface : str
            Volatility surface identifier in MarketContext.
        spot_id : str
            Spot price identifier in MarketContext.
        local_cap : float, optional
            Local cap per period (default: 0.0 = no cap). E.g., 0.10 for 10% cap.
        local_floor : float, optional
            Local floor per period (default: 0.0 = no floor). E.g., -0.05 for -5% floor.
        global_cap : float, optional
            Global cap on total return (default: 0.0 = no cap). E.g., 0.30 for 30% cap.
        global_floor : float, optional
            Global floor on total return (default: 0.0 = no floor). E.g., 0.0 for 0% floor.
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
            ...     date(2024, 12, 31),
            ...     Money(1_000_000, Currency("USD")),
            ...     discount_curve="USD",
            ...     vol_surface="SPX-VOL",
            ...     spot_id="SPX",
            ...     local_cap=0.10,
            ...     global_cap=0.30,
            ... )
        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def underlying_ticker(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def maturity(self) -> date: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
