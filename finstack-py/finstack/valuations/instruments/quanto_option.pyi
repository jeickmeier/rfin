"""Quanto option instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.currency import Currency

class QuantoOption:
    """Quanto option with FX risk eliminated.

    QuantoOption represents an option on a foreign asset where the payoff
    is converted to a domestic currency at a fixed exchange rate, eliminating
    FX risk. The correlation between equity and FX affects pricing.

    Quanto options are used for international equity exposure without FX risk.
    They require discount curves for both currencies, equity prices, and
    correlation parameters.

    Examples
    --------
    Create a quanto call option:

        >>> from finstack.valuations.instruments import QuantoOption
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> quanto = QuantoOption.builder(
        ...     "QUANTO-NIKKEI-CALL",
        ...     ticker="NIKKEI",
        ...     equity_strike=30000.0,
        ...     option_type="call",
        ...     expiry=date(2024, 12, 20),
        ...     notional=Money(1_000_000, Currency("USD")),  # Payoff in USD
        ...     domestic_currency=Currency("USD"),
        ...     foreign_currency=Currency("JPY"),
        ...     correlation=-0.3,  # Negative correlation (equity down, JPY up)
        ...     discount_curve="USD",
        ...     foreign_discount_curve="JPY",
        ...     spot_id="NIKKEI",
        ...     vol_surface="NIKKEI-VOL",
        ...     div_yield_id=None,
        ...     fx_rate_id=None,
        ...     fx_vol_id=None,
        ... )

    Notes
    -----
    - Quanto options require domestic and foreign discount curves
    - Correlation between equity and FX affects quanto adjustment
    - Payoff is in domestic currency at fixed FX rate
    - Quanto adjustment compensates for FX risk elimination
    - Negative correlation typically increases quanto option value

    See Also
    --------
    :class:`EquityOption`: Standard equity options
    :class:`FxOption`: FX options
    :class:`PricerRegistry`: Pricing entry point
    """

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        ticker: str,
        equity_strike: float,
        option_type: str,
        expiry: date,
        notional: Money,
        domestic_currency: Currency,
        foreign_currency: Currency,
        correlation: float,
        discount_curve: str,
        foreign_discount_curve: str,
        spot_id: str,
        vol_surface: str,
        *,
        div_yield_id: Optional[str] = None,
        fx_rate_id: Optional[str] = None,
        fx_vol_id: Optional[str] = None,
    ) -> "QuantoOption":
        """Create a quanto option.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the option.
        ticker : str
            Underlying equity ticker symbol (foreign asset).
        equity_strike : float
            Strike price in foreign currency. Must be > 0.
        option_type : str
            Option type: "call" or "put".
        expiry : date
            Option expiration date.
        notional : Money
            Notional amount in domestic currency.
        domestic_currency : Currency
            Currency for payoff (e.g., Currency("USD")).
        foreign_currency : Currency
            Currency of underlying asset (e.g., Currency("JPY")).
        correlation : float
            Correlation between equity returns and FX returns (typically -0.2 to -0.5).
        discount_curve : str
            Domestic discount curve identifier in MarketContext.
        foreign_discount_curve : str
            Foreign discount curve identifier in MarketContext.
        spot_id : str
            Equity spot price identifier in MarketContext.
        vol_surface : str
            Equity volatility surface identifier in MarketContext.
        div_yield_id : str, optional
            Dividend yield identifier in MarketContext.
        fx_rate_id : str, optional
            FX rate identifier in MarketContext (default: uses currency pair).
        fx_vol_id : str, optional
            FX volatility identifier for quanto adjustment.

        Returns
        -------
        QuantoOption
            Configured quanto option ready for pricing.

        Raises
        ------
        ValueError
            If parameters are invalid or if required market data is missing.

        Examples
        --------
            >>> option = QuantoOption.builder(
            ...     "QUANTO-NIKKEI",
            ...     "NIKKEI",
            ...     30000.0,
            ...     "call",
            ...     date(2024, 12, 20),
            ...     Money(1_000_000, Currency("USD")),
            ...     Currency("USD"),
            ...     Currency("JPY"),
            ...     -0.3,
            ...     discount_curve="USD",
            ...     foreign_discount_curve="JPY",
            ...     spot_id="NIKKEI",
            ...     vol_surface="NIKKEI-VOL",
            ... )
        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def ticker(self) -> str: ...
    @property
    def equity_strike(self) -> Money: ...
    @property
    def option_type(self) -> str: ...
    @property
    def expiry(self) -> date: ...
    @property
    def notional(self) -> Money: ...
    @property
    def correlation(self) -> float: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
