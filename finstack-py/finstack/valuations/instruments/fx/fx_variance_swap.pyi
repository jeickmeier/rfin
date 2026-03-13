"""FX variance swap instrument."""

from __future__ import annotations

from datetime import date

from ....core.currency import Currency
from ....core.dates.daycount import DayCount
from ....core.market_data.context import MarketContext
from ....core.money import Money
from ...common import InstrumentType

class FxVarianceDirection:
    """Variance direction (pay or receive variance)."""

    PAY: FxVarianceDirection
    RECEIVE: FxVarianceDirection

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class FxRealizedVarianceMethod:
    """Realized variance calculation method."""

    CLOSE_TO_CLOSE: FxRealizedVarianceMethod
    PARKINSON: FxRealizedVarianceMethod
    GARMAN_KLASS: FxRealizedVarianceMethod
    ROGERS_SATCHELL: FxRealizedVarianceMethod
    YANG_ZHANG: FxRealizedVarianceMethod

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class FxVarianceSwapBuilder:
    """Fluent builder returned by :meth:`FxVarianceSwap.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def base_currency(self, ccy: str | Currency) -> FxVarianceSwapBuilder: ...
    def quote_currency(self, ccy: str | Currency) -> FxVarianceSwapBuilder: ...
    def spot_id(self, id: str) -> FxVarianceSwapBuilder: ...
    def notional(self, notional: Money) -> FxVarianceSwapBuilder: ...
    def strike_variance(self, variance: float) -> FxVarianceSwapBuilder: ...
    def start_date(self, date: date) -> FxVarianceSwapBuilder: ...
    def maturity(self, date: date) -> FxVarianceSwapBuilder: ...
    def observation_freq(self, freq: str) -> FxVarianceSwapBuilder: ...
    def realized_method(self, method: str | FxRealizedVarianceMethod) -> FxVarianceSwapBuilder: ...
    def open_series_id(self, series_id: str) -> FxVarianceSwapBuilder:
        """Set open price series ID (required for OHLC estimators)."""
        ...
    def high_series_id(self, series_id: str) -> FxVarianceSwapBuilder:
        """Set high price series ID (required for OHLC estimators)."""
        ...
    def low_series_id(self, series_id: str) -> FxVarianceSwapBuilder:
        """Set low price series ID (required for OHLC estimators)."""
        ...
    def close_series_id(self, series_id: str) -> FxVarianceSwapBuilder:
        """Set close price series ID. Defaults to spot_id when not set."""
        ...
    def side(self, direction: str | FxVarianceDirection) -> FxVarianceSwapBuilder: ...
    def domestic_discount_curve(self, curve_id: str) -> FxVarianceSwapBuilder: ...
    def foreign_discount_curve(self, curve_id: str) -> FxVarianceSwapBuilder: ...
    def vol_surface(self, surface_id: str) -> FxVarianceSwapBuilder: ...
    def day_count(self, dc: DayCount) -> FxVarianceSwapBuilder: ...
    def build(self) -> FxVarianceSwap: ...
    def __repr__(self) -> str: ...

class FxVarianceSwap:
    """FX variance swap — exchange of realized vs. fixed variance on an FX pair.

    An FX variance swap pays the difference between the realized variance of
    an exchange rate over the observation period and a pre-agreed strike
    variance, multiplied by the notional vega amount.  The payoff is:

    .. math::

        \\text{Payoff} = N \\cdot (\\sigma^2_{\\text{realized}} - K^2)

    where :math:`N` is the vega notional, :math:`\\sigma^2_{\\text{realized}}`
    is the annualized realized variance, and :math:`K^2` is the strike variance.

    Fair-value strike at inception equals the fair forward variance implied by
    the volatility surface (replication via log-contract, Carr-Madan).

    Examples
    --------
    Build a 3-month EUR/USD variance swap:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments.fx import FxVarianceSwap
        >>> var_swap = (
        ...     FxVarianceSwap
        ...     .builder("FXVS-001")
        ...     .base_currency("EUR")
        ...     .quote_currency("USD")
        ...     .notional(Money(100_000, Currency("USD")))
        ...     .strike_variance(0.04)
        ...     .start_date(date(2024, 6, 3))
        ...     .maturity(date(2024, 9, 3))
        ...     .side("pay")
        ...     .domestic_discount_curve("USD-OIS")
        ...     .foreign_discount_curve("EUR-OIS")
        ...     .vol_surface("EURUSD-VOL")
        ...     .build()
        ... )
        >>> var_swap.strike_variance
        0.04

    Attributes
    ----------
    instrument_id : str
        Unique trade identifier.
    base_currency : Currency
        Foreign (base) currency of the pair.
    quote_currency : Currency
        Domestic (quote) currency of the pair.
    notional : Money
        Vega notional (PnL per unit of variance).
    strike_variance : float
        Pre-agreed variance strike :math:`K^2`.
    start_date : date
        Start of the observation window.
    maturity : date
        End of the observation window and settlement date.
    side : FxVarianceDirection
        ``PAY`` (long variance) or ``RECEIVE`` (short variance).
    observation_freq : str
        Observation frequency (e.g. ``"daily"``).
    realized_var_method : FxRealizedVarianceMethod
        Method used to compute realized variance (e.g. ``CLOSE_TO_CLOSE``).
    spot_id : str or None
        Market data id for the FX spot rate time series (for historical fixing).
    domestic_discount_curve : str
        Discount curve id for the domestic (quote-currency) leg.
    foreign_discount_curve : str
        Discount curve id for the foreign (base-currency) leg.
    vol_surface : str
        Volatility surface id used to compute fair forward variance.
    day_count : DayCount
        Day count convention for annualisation.

    MarketContext Requirements
    -------------------------
    - Domestic and foreign discount curves.
    - FX volatility surface for the pair.
    - FX spot rate (for forward variance replication).
    - Historical fixing series (if ``spot_id`` is set and partial realized variance is needed).

    See Also
    --------
    :class:`FxOption` : FX vanilla option.
    :class:`FxBarrierOption` : FX barrier option.

    Sources
    -------
    - Carr & Madan (1998) "Towards a Theory of Volatility Trading":
      see ``docs/REFERENCES.md#carrMadanVariance1998``.
    - Demeterfi et al. (1999) "A Guide to Variance Swaps":
      see ``docs/REFERENCES.md#demeterfiVariance1999``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> FxVarianceSwapBuilder: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    @property
    def base_currency(self) -> Currency: ...
    @property
    def quote_currency(self) -> Currency: ...
    @property
    def notional(self) -> Money: ...
    @property
    def strike_variance(self) -> float: ...
    @property
    def start_date(self) -> date: ...
    @property
    def maturity(self) -> date: ...
    @property
    def side(self) -> FxVarianceDirection: ...
    @property
    def observation_freq(self) -> str: ...
    @property
    def realized_var_method(self) -> FxRealizedVarianceMethod: ...
    @property
    def spot_id(self) -> str | None: ...
    @property
    def open_series_id(self) -> str | None:
        """Open price series ID (required for OHLC estimators)."""
        ...
    @property
    def high_series_id(self) -> str | None:
        """High price series ID (required for OHLC estimators)."""
        ...
    @property
    def low_series_id(self) -> str | None:
        """Low price series ID (required for OHLC estimators)."""
        ...
    @property
    def close_series_id(self) -> str | None:
        """Close price series ID. Defaults to spot_id when not set."""
        ...
    @property
    def domestic_discount_curve(self) -> str: ...
    @property
    def foreign_discount_curve(self) -> str: ...
    @property
    def vol_surface(self) -> str: ...
    @property
    def day_count(self) -> DayCount: ...
    def value(self, market: MarketContext, as_of: date) -> Money: ...
    def payoff(self, realized_variance: float) -> Money: ...
    def observation_dates(self) -> list[date]: ...
    def annualization_factor(self) -> float: ...
    def partial_realized_variance(self, market: MarketContext, as_of: date) -> float: ...
    def remaining_forward_variance(self, market: MarketContext, as_of: date) -> float: ...
    def __repr__(self) -> str: ...
