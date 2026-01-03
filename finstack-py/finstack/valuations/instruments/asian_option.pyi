"""Asian option instrument."""

from typing import Optional, List
from datetime import date
from ...core.money import Money

class AveragingMethod:
    """Averaging method enumeration."""

    ARITHMETIC: "AveragingMethod"
    GEOMETRIC: "AveragingMethod"

    @classmethod
    def from_name(cls, name: str) -> "AveragingMethod": ...
    @property
    def name(self) -> str: ...

class AsianOption:
    """Asian option with average price payoff.

    AsianOption represents an option whose payoff depends on the average price
    of the underlying over a set of fixing dates, rather than the spot price
    at expiry. This reduces volatility and makes Asian options cheaper than
    standard options.

    Asian options are used for hedging average price exposure and reducing
    option costs. They require discount curves, spot prices, and volatility
    surfaces.

    Examples
    --------
    Create an Asian call option:

        >>> from finstack.valuations.instruments import AsianOption
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> fixing_dates = [
        ...     date(2024, 1, 15),
        ...     date(2024, 2, 15),
        ...     date(2024, 3, 15),
        ...     date(2024, 4, 15),
        ...     date(2024, 5, 15),
        ...     date(2024, 6, 15),
        ... ]
        >>> asian_option = AsianOption.builder(
        ...     "ASIAN-AAPL-CALL",
        ...     ticker="AAPL",
        ...     strike=150.0,
        ...     expiry=date(2024, 6, 20),
        ...     fixing_dates=fixing_dates,
        ...     notional=Money(100_000, Currency("USD")),
        ...     discount_curve="USD",
        ...     spot_id="AAPL",
        ...     vol_surface="AAPL-VOL",
        ...     averaging_method="arithmetic",  # or "geometric"
        ...     option_type="call",
        ...     div_yield_id=None,
        ... )

    Notes
    -----
    - Asian options require discount curve, spot price, and volatility surface
    - Averaging method: "arithmetic" (default) or "geometric"
    - Payoff depends on average of fixing prices, not spot at expiry
    - Asian options are typically cheaper than standard options
    - Fixing dates should be evenly spaced for better pricing

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
    - Gobet & Miri (2014): see ``docs/REFERENCES.md#gobetMiri2014AveragedDiffusions``.
    """

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        ticker: str,
        strike: float,
        expiry: date,
        fixing_dates: List[date],
        notional: Money,
        discount_curve: str,
        spot_id: str,
        vol_surface: str,
        *,
        averaging_method: Optional[str] = "arithmetic",
        option_type: Optional[str] = "call",
        div_yield_id: Optional[str] = None,
    ) -> "AsianOption":
        """Create an Asian option with explicit parameters.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the option.
        ticker : str
            Underlying equity ticker symbol.
        strike : float
            Strike price. Must be > 0.
        expiry : date
            Option expiration date (should be >= last fixing date).
        fixing_dates : List[date]
            List of dates when underlying prices are observed for averaging.
            Must be in ascending order and before expiry.
        notional : Money
            Notional amount.
        discount_curve : str
            Discount curve identifier in MarketContext.
        spot_id : str
            Spot price identifier in MarketContext.
        vol_surface : str
            Volatility surface identifier in MarketContext.
        averaging_method : str, optional
            Averaging method: "arithmetic" (default) or "geometric".
        option_type : str, optional
            Option type: "call" (default) or "put".
        div_yield_id : str, optional
            Dividend yield identifier in MarketContext.

        Returns
        -------
        AsianOption
            Configured Asian option ready for pricing.

        Raises
        ------
        ValueError
            If parameters are invalid (fixing_dates empty, dates out of order,
            etc.) or if required market data is missing.

        Examples
        --------
            >>> fixing_dates = [date(2024, i, 15) for i in range(1, 7)]
            >>> option = AsianOption.builder(
            ...     "ASIAN-AAPL",
            ...     "AAPL",
            ...     strike=150.0,
            ...     expiry=date(2024, 6, 20),
            ...     fixing_dates=fixing_dates,
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
    def option_type(self) -> str: ...
    @property
    def averaging_method(self) -> str: ...
    @property
    def expiry(self) -> date: ...
    @property
    def fixing_dates(self) -> List[date]: ...
    @property
    def notional(self) -> Money: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def spot_id(self) -> str: ...
    @property
    def vol_surface(self) -> str: ...
    @property
    def div_yield_id(self) -> Optional[str]: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
