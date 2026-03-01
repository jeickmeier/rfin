"""FX variance swap instrument."""

from __future__ import annotations

from datetime import date

from ...core.currency import Currency
from ...core.dates.daycount import DayCount
from ...core.market_data.context import MarketContext
from ...core.money import Money
from ..common import InstrumentType


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

    def base_currency(self, ccy: str | Currency) -> FxVarianceSwapBuilder: ...
    def quote_currency(self, ccy: str | Currency) -> FxVarianceSwapBuilder: ...
    def spot_id(self, id: str) -> FxVarianceSwapBuilder: ...
    def notional(self, notional: Money) -> FxVarianceSwapBuilder: ...
    def strike_variance(self, variance: float) -> FxVarianceSwapBuilder: ...
    def start_date(self, date: date) -> FxVarianceSwapBuilder: ...
    def maturity(self, date: date) -> FxVarianceSwapBuilder: ...
    def observation_freq(self, freq: str) -> FxVarianceSwapBuilder: ...
    def realized_method(self, method: str | FxRealizedVarianceMethod) -> FxVarianceSwapBuilder: ...
    def side(self, direction: str | FxVarianceDirection) -> FxVarianceSwapBuilder: ...
    def domestic_discount_curve(self, curve_id: str) -> FxVarianceSwapBuilder: ...
    def foreign_discount_curve(self, curve_id: str) -> FxVarianceSwapBuilder: ...
    def vol_surface(self, surface_id: str) -> FxVarianceSwapBuilder: ...
    def day_count(self, dc: DayCount) -> FxVarianceSwapBuilder: ...
    def build(self) -> FxVarianceSwap: ...
    def __repr__(self) -> str: ...


class FxVarianceSwap:
    """FX variance swap instrument.

    A variance swap on an FX rate pair. The payoff is based on the realized variance
    of FX rate returns over the observation period.

    Payoff = Notional x (Realized Variance - Strike Variance)

    Before maturity, the contract is valued by combining partial realized variance
    from observed FX rates, implied forward variance from volatility surface,
    and discounting to present value.

    Examples
    --------
    Create a 1-year EUR/USD variance swap:

        >>> from finstack.valuations.instruments import FxVarianceSwap
        >>> var_swap = (
        ...     FxVarianceSwap.builder("FXVAR-EURUSD-1Y")
        ...     .base_currency("EUR")
        ...     .quote_currency("USD")
        ...     .notional(Money.from_code(1_000_000, "USD"))
        ...     .strike_variance(0.04)
        ...     .start_date(date(2024, 1, 2))
        ...     .maturity(date(2025, 1, 2))
        ...     .observation_freq("daily")
        ...     .realized_method("close_to_close")
        ...     .side("receive")
        ...     .domestic_discount_curve("USD-OIS")
        ...     .foreign_discount_curve("EUR-OIS")
        ...     .vol_surface("EURUSD-VOL")
        ...     .build()
        ... )

    See Also
    --------
    :class:`VarianceSwap`: Equity variance swap
    :class:`FxOption`: Vanilla FX option
    """

    @classmethod
    def builder(cls, instrument_id: str) -> FxVarianceSwapBuilder:
        """Create a builder for an FX variance swap."""
        ...
    @property
    def instrument_id(self) -> str:
        """Instrument identifier."""
        ...
    @property
    def instrument_type(self) -> InstrumentType:
        """Instrument type."""
        ...
    @property
    def base_currency(self) -> Currency:
        """Base currency (foreign)."""
        ...
    @property
    def quote_currency(self) -> Currency:
        """Quote currency (domestic)."""
        ...
    @property
    def notional(self) -> Money:
        """Variance notional."""
        ...
    @property
    def strike_variance(self) -> float:
        """Strike variance (annualized)."""
        ...
    @property
    def start_date(self) -> date:
        """Start date of observation period."""
        ...
    @property
    def maturity(self) -> date:
        """Maturity/settlement date."""
        ...
    @property
    def side(self) -> FxVarianceDirection:
        """Variance direction (pay or receive)."""
        ...
    def value(self, market: MarketContext, as_of: date) -> Money:
        """Calculate present value of the FX variance swap."""
        ...
    def payoff(self, realized_variance: float) -> Money:
        """Calculate payoff given realized variance."""
        ...
    def observation_dates(self) -> list[date]:
        """Get observation dates based on frequency."""
        ...
    def annualization_factor(self) -> float:
        """Calculate annualization factor based on observation frequency."""
        ...
    def __repr__(self) -> str: ...
