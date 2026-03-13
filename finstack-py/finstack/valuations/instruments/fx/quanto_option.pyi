"""Quanto option instrument."""

from __future__ import annotations

from datetime import date

from ....core.currency import Currency
from ....core.dates.daycount import DayCount
from ....core.market_data.context import MarketContext
from ....core.money import Money
from ...common import InstrumentType

class QuantoOptionBuilder:
    """Fluent builder returned by :meth:`QuantoOption.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def base_currency(self, ccy: str | Currency) -> QuantoOptionBuilder: ...
    def quote_currency(self, ccy: str | Currency) -> QuantoOptionBuilder: ...
    def ticker(self, ticker: str) -> QuantoOptionBuilder: ...
    def equity_strike(self, strike: float) -> QuantoOptionBuilder: ...
    def option_type(self, option_type: str) -> QuantoOptionBuilder: ...
    def expiry(self, date: date) -> QuantoOptionBuilder: ...
    def notional(self, notional: Money) -> QuantoOptionBuilder: ...
    def correlation(self, correlation: float) -> QuantoOptionBuilder: ...
    def domestic_discount_curve(self, curve_id: str) -> QuantoOptionBuilder: ...
    def foreign_discount_curve(self, curve_id: str) -> QuantoOptionBuilder: ...
    def spot_id(self, id: str) -> QuantoOptionBuilder: ...
    def vol_surface(self, surface_id: str) -> QuantoOptionBuilder: ...
    def div_yield_id(self, curve_id: str) -> QuantoOptionBuilder: ...
    def fx_rate_id(self, rate_id: str) -> QuantoOptionBuilder: ...
    def fx_vol_id(self, vol_id: str) -> QuantoOptionBuilder: ...
    def day_count(self, dc: DayCount) -> QuantoOptionBuilder: ...
    def underlying_quantity(self, qty: float) -> QuantoOptionBuilder: ...
    def payoff_fx_rate(self, rate: float) -> QuantoOptionBuilder: ...
    def build(self) -> QuantoOption: ...
    def __repr__(self) -> str: ...

class QuantoOption:
    """Quanto option — equity option with FX-fixed payoff in a foreign currency.

    A quanto option pays the equity option payoff in a domestic currency at
    a pre-fixed exchange rate, removing the FX risk from the payoff.  The
    underlying asset is typically a foreign equity or index, but the payoff
    is delivered in the domestic (investor) currency.

    Pricing uses the Garman-Kohlhagen-extended Black-Scholes formula
    adjusted for the quanto drift correction, which depends on the
    correlation between the underlying asset and the FX rate:

    .. math::

        \\mu^q = r_d - q - \\rho \\sigma_S \\sigma_{FX}

    where :math:`\\rho` is the equity-FX correlation, :math:`\\sigma_S` is
    the equity volatility, and :math:`\\sigma_{FX}` is the FX volatility.

    Examples
    --------
    Build a quanto call on Nikkei 225 (USD payoff):

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments.fx import QuantoOption
        >>> opt = (
        ...     QuantoOption
        ...     .builder("QUANTO-001")
        ...     .base_currency("JPY")
        ...     .quote_currency("USD")
        ...     .ticker("NKY")
        ...     .equity_strike(40_000.0)
        ...     .option_type("call")
        ...     .notional(Money(1_000_000, Currency("USD")))
        ...     .expiry(date(2024, 12, 20))
        ...     .correlation(-0.20)
        ...     .domestic_discount_curve("USD-OIS")
        ...     .foreign_discount_curve("JPY-OIS")
        ...     .spot_id("NKY-SPOT")
        ...     .vol_surface("NKY-VOL")
        ...     .fx_rate_id("USDJPY-SPOT")
        ...     .fx_vol_id("USDJPY-VOL")
        ...     .build()
        ... )
        >>> opt.correlation
        -0.2

    Attributes
    ----------
    instrument_id : str
        Unique trade identifier.
    base_currency : Currency
        Currency of the underlying asset (foreign currency).
    quote_currency : Currency
        Domestic (payoff) currency.
    ticker : str
        Underlying asset or index identifier.
    equity_strike : Money
        Option strike as a monetary amount in the base currency.
    option_type : str
        ``"call"`` or ``"put"``.
    notional : Money
        Notional in the domestic (quote) currency.
    expiry : date
        Option expiration date.
    correlation : float
        Equity-FX correlation :math:`\\rho` used for quanto drift correction.
    domestic_discount_curve : str
        Discount curve id for the domestic (quote-currency) leg.
    foreign_discount_curve : str
        Discount curve id for the foreign (base-currency) leg.
    spot_id : str
        Market data id for the underlying equity spot price.
    vol_surface : str
        Volatility surface id for the underlying equity.
    div_yield_id : str or None
        Dividend yield curve id.
    fx_rate_id : str or None
        Market data id for the FX spot rate.
    fx_vol_id : str or None
        Volatility id for the FX rate.
    day_count : DayCount
        Day count convention for time-to-expiry calculation.

    MarketContext Requirements
    -------------------------
    - Domestic and foreign discount curves.
    - Equity volatility surface.
    - Equity spot price.
    - FX spot rate.
    - FX volatility (for quanto drift).
    - Dividend yield curve (optional).

    See Also
    --------
    :class:`FxOption` : FX vanilla option (Garman-Kohlhagen).
    :class:`FxVarianceSwap` : FX variance swap.

    Sources
    -------
    - Garman & Kohlhagen (1983): see ``docs/REFERENCES.md#garmanKohlhagen1983``.
    - Derman & Kani (1993) "The ins and outs of barrier options":
      see ``docs/REFERENCES.md#dermanKaniQuanto1993``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> QuantoOptionBuilder: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    @property
    def base_currency(self) -> Currency: ...
    @property
    def quote_currency(self) -> Currency: ...
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
    @property
    def domestic_discount_curve(self) -> str: ...
    @property
    def foreign_discount_curve(self) -> str: ...
    @property
    def spot_id(self) -> str: ...
    @property
    def vol_surface(self) -> str: ...
    @property
    def div_yield_id(self) -> str | None: ...
    @property
    def fx_rate_id(self) -> str | None: ...
    @property
    def fx_vol_id(self) -> str | None: ...
    @property
    def day_count(self) -> DayCount: ...
    def value(self, market: MarketContext, as_of: date) -> Money: ...
    def __repr__(self) -> str: ...
