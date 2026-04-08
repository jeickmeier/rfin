"""Variance swap instrument."""

from __future__ import annotations
from typing import List, Self
from datetime import date
from ....core.currency import Currency
from ....core.money import Money
from ....core.dates.schedule import Frequency
from ....core.dates.daycount import DayCount
from ....core.market_data.context import MarketContext
from ...common import InstrumentType

class VarianceDirection:
    """Pay/receive wrapper for variance swap payoffs."""

    PAY: "VarianceDirection"
    RECEIVE: "VarianceDirection"

class RealizedVarianceMethod:
    """Realized variance calculation method wrapper."""

    CLOSE_TO_CLOSE: "RealizedVarianceMethod"
    PARKINSON: "RealizedVarianceMethod"
    GARMAN_KLASS: "RealizedVarianceMethod"
    ROGERS_SATCHELL: "RealizedVarianceMethod"
    YANG_ZHANG: "RealizedVarianceMethod"

class VarianceSwapBuilder:
    """Fluent builder returned by :meth:`VarianceSwap.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def underlying_id(self, underlying_id: str) -> VarianceSwapBuilder: ...
    def notional(self, amount: float) -> VarianceSwapBuilder: ...
    def currency(self, currency: str | Currency) -> VarianceSwapBuilder: ...
    def money(self, money: Money) -> VarianceSwapBuilder: ...
    def strike_variance(self, strike_variance: float) -> VarianceSwapBuilder: ...
    def start_date(self, start_date: date) -> VarianceSwapBuilder: ...
    def maturity(self, maturity: date) -> VarianceSwapBuilder: ...
    def disc_id(self, curve_id: str) -> VarianceSwapBuilder: ...
    def observation_frequency(self, observation_frequency: Frequency) -> VarianceSwapBuilder: ...
    def realized_method(self, realized_method: RealizedVarianceMethod | None = ...) -> VarianceSwapBuilder: ...
    def open_series_id(self, series_id: str) -> VarianceSwapBuilder:
        """Set the open price series ID.

        Required for Parkinson, GarmanKlass, RogersSatchell, and YangZhang estimators.
        """
        ...
    def high_series_id(self, series_id: str) -> VarianceSwapBuilder:
        """Set the high price series ID.

        Required for Parkinson, GarmanKlass, RogersSatchell, and YangZhang estimators.
        """
        ...
    def low_series_id(self, series_id: str) -> VarianceSwapBuilder:
        """Set the low price series ID.

        Required for Parkinson, GarmanKlass, RogersSatchell, and YangZhang estimators.
        """
        ...
    def close_series_id(self, series_id: str) -> VarianceSwapBuilder:
        """Set the close price series ID.

        Defaults to ``underlying_id`` when not set.
        """
        ...
    def side(self, side: VarianceDirection | str | None = ...) -> VarianceSwapBuilder: ...
    def day_count(self, day_count: DayCount) -> VarianceSwapBuilder: ...
    def build(self) -> "VarianceSwap": ...

class VarianceSwap:
    """Variance swap for volatility trading.

    VarianceSwap represents a swap where one party pays realized variance
    and receives strike variance (or vice versa). Variance swaps allow direct
    trading of volatility without delta hedging.

    Variance swaps are used for volatility trading, hedging vega exposure,
    and creating volatility strategies. They require discount curves and
    underlying price observations.

    Examples
    --------
    Create a variance swap:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.dates.daycount import DayCount
        >>> from finstack.core.dates.schedule import Frequency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import (
        ...     VarianceDirection,
        ...     VarianceSwap,
        ...     RealizedVarianceMethod,
        ... )
        >>> variance_swap = (
        ...     VarianceSwap
        ...     .builder("VAR-SWAP-SPX")
        ...     .underlying_id("SPX")
        ...     .money(Money(1_000_000, Currency("USD")))
        ...     .strike_variance(0.04)  # 20% vol squared (0.20^2)
        ...     .start_date(date(2024, 1, 1))
        ...     .maturity(date(2024, 12, 31))  # 1-year swap
        ...     .disc_id("USD-OIS")
        ...     .observation_frequency(Frequency.DAILY)
        ...     .realized_method(RealizedVarianceMethod.CLOSE_TO_CLOSE)
        ...     .side(VarianceDirection.RECEIVE)  # Receive realized, pay strike
        ...     .day_count(DayCount.ACT_365F)
        ...     .build()
        ... )

    Notes
    -----
    - Variance swaps require discount curve and underlying price observations
    - Strike variance is the fixed variance rate (volatility squared)
    - Realized variance is calculated from price observations
    - Observation frequency determines how often prices are sampled
    - Realized method determines variance calculation formula

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required).
    - Underlying price observations: referenced by ``underlying_id`` (required for
      ``CLOSE_TO_CLOSE``).
    - OHLC series: ``open_series_id``, ``high_series_id``, ``low_series_id``,
      and ``close_series_id`` (required for Parkinson, GarmanKlass, RogersSatchell,
      YangZhang estimators).

    See Also
    --------
    :class:`EquityOption`: Equity options (implied volatility)
    :class:`VolSurface`: Volatility surfaces
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - Demeterfi et al. (1999): see ``docs/REFERENCES.md#demeterfiVarianceSwaps1999``.
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> VarianceSwapBuilder: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    @property
    def strike_variance(self) -> float: ...
    @property
    def observation_frequency(self) -> str: ...
    @property
    def realized_method(self) -> str: ...
    @property
    def open_series_id(self) -> str | None:
        """Open price series identifier (required for OHLC-based estimators)."""
        ...
    @property
    def high_series_id(self) -> str | None:
        """High price series identifier (required for OHLC-based estimators)."""
        ...
    @property
    def low_series_id(self) -> str | None:
        """Low price series identifier (required for OHLC-based estimators)."""
        ...
    @property
    def close_series_id(self) -> str | None:
        """Close price series identifier. Defaults to ``underlying_ticker`` when not set."""
        ...
    @property
    def side(self) -> str: ...
    @property
    def underlying_ticker(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def start_date(self) -> date: ...
    @property
    def maturity(self) -> date: ...
    @property
    def day_count(self) -> DayCount: ...
    @property
    def discount_curve_id(self) -> str: ...
    @staticmethod
    def vega_to_variance_notional(vega_notional: float, strike_vol: float) -> float:
        """Convert vega notional to variance notional.

        Parameters
        ----------
        vega_notional : float
            Vega notional amount.
        strike_vol : float
            Strike volatility.

        Returns
        -------
        float
            Variance notional.
        """
        ...

    @staticmethod
    def variance_to_vega_notional(variance_notional: float, strike_vol: float) -> float:
        """Convert variance notional to vega notional.

        Parameters
        ----------
        variance_notional : float
            Variance notional amount.
        strike_vol : float
            Strike volatility.

        Returns
        -------
        float
            Vega notional.
        """
        ...

    def value(self, market: MarketContext, as_of: date) -> Money:
        """Compute present value.

        Parameters
        ----------
        market : MarketContext
            Market data context.
        as_of : date
            Valuation date.

        Returns
        -------
        Money
            Present value.
        """
        ...

    def payoff(self, realized_variance: float) -> Money:
        """Compute payoff for a given realized variance.

        Parameters
        ----------
        realized_variance : float
            Realized variance.

        Returns
        -------
        Money
            Payoff amount.
        """
        ...

    def observation_dates(self) -> List[date]:
        """Return the schedule of observation dates.

        Returns
        -------
        List[date]
            Observation dates.
        """
        ...

    def annualization_factor(self) -> float:
        """Return the annualization factor for the swap.

        Returns
        -------
        float
            Annualization factor.
        """
        ...

    def time_elapsed_fraction(self, as_of: date) -> float:
        """Fraction of total swap time elapsed as of a given date.

        Parameters
        ----------
        as_of : date
            Reference date.

        Returns
        -------
        float
            Fraction of time elapsed (0 to 1).
        """
        ...

    def realized_fraction_by_observations(self, as_of: date) -> float:
        """Fraction of observations realized as of a given date.

        Parameters
        ----------
        as_of : date
            Reference date.

        Returns
        -------
        float
            Fraction of observations realized (0 to 1).
        """
        ...

    def partial_realized_variance(self, market: MarketContext, as_of: date) -> float:
        """Compute partial realized variance from historical prices.

        Parameters
        ----------
        market : MarketContext
            Market data context with historical prices.
        as_of : date
            Reference date.

        Returns
        -------
        float
            Partial realized variance.
        """
        ...

    def remaining_forward_variance(self, market: MarketContext, as_of: date) -> float:
        """Compute remaining forward variance from the vol surface.

        Parameters
        ----------
        market : MarketContext
            Market data context.
        as_of : date
            Reference date.

        Returns
        -------
        float
            Remaining forward variance.
        """
        ...

    def get_historical_prices(self, market: MarketContext, as_of: date) -> List[float]:
        """Retrieve historical prices for realized variance calculation.

        Parameters
        ----------
        market : MarketContext
            Market data context with price history.
        as_of : date
            Reference date.

        Returns
        -------
        List[float]
            Historical prices.
        """
        ...

    def to_json(self) -> str:
        """Serialize to JSON in envelope format.

        Returns:
            str: JSON string with schema version and tagged instrument spec.
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> "Self":
        """Deserialize from JSON in envelope format.

        Args:
            json_str: JSON string in envelope format.

        Returns:
            The deserialized instrument.

        Raises:
            ValueError: If JSON is malformed or contains a different instrument type.
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
