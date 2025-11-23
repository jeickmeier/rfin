"""Variance swap instrument."""

from typing import Optional, Union
from datetime import date
from ...core.money import Money
from ...core.dates.schedule import Frequency
from ...core.dates.daycount import DayCount
from ..common import InstrumentType

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
        >>> variance_swap = VarianceSwap.create(
        ...     "VAR-SWAP-SPX",
        ...     underlying_id="SPX",
        ...     notional=Money(1_000_000, Currency("USD")),
        ...     strike_variance=0.04,  # 20% vol squared (0.20^2)
        ...     start_date=date(2024, 1, 1),
        ...     maturity=date(2024, 12, 31),  # 1-year swap
        ...     discount_curve="USD-OIS",
        ...     observation_frequency=Frequency.DAILY,
        ...     realized_method=RealizedVarianceMethod.CLOSE_TO_CLOSE,
        ...     side=VarianceDirection.RECEIVE,  # Receive realized, pay strike
        ...     day_count=DayCount.ACT_365F,
        ... )

    Notes
    -----
    - Variance swaps require discount curve and underlying price observations
    - Strike variance is the fixed variance rate (volatility squared)
    - Realized variance is calculated from price observations
    - Observation frequency determines how often prices are sampled
    - Realized method determines variance calculation formula

    See Also
    --------
    :class:`EquityOption`: Equity options (implied volatility)
    :class:`VolSurface`: Volatility surfaces
    :class:`PricerRegistry`: Pricing entry point
    """

    @classmethod
    def create(
        cls,
        instrument_id: str,
        underlying_id: str,
        notional: Money,
        strike_variance: float,
        start_date: date,
        maturity: date,
        discount_curve: str,
        observation_frequency: Frequency,
        *,
        realized_method: Optional[RealizedVarianceMethod] = None,
        side: Optional[Union[VarianceDirection, str]] = None,
        day_count: Optional[DayCount] = None,
    ) -> "VarianceSwap": ...
    """Create a variance swap.

    Parameters
    ----------
    instrument_id : str
        Unique identifier for the swap (e.g., "VAR-SWAP-SPX").
    underlying_id : str
        Underlying asset identifier (e.g., "SPX", "AAPL").
    notional : Money
        Variance notional (typically in variance points, not currency).
    strike_variance : float
        Strike variance rate (volatility squared, e.g., 0.04 for 20% vol).
    start_date : date
        Swap start date (first observation date).
    maturity : date
        Swap maturity date (last observation date). Must be after start_date.
    discount_curve : str
        Discount curve identifier in MarketContext.
    observation_frequency : Frequency
        Frequency of price observations (e.g., Frequency.DAILY, Frequency.WEEKLY).
    realized_method : RealizedVarianceMethod, optional
        Method for calculating realized variance (default: CLOSE_TO_CLOSE).
    side : VarianceDirection or str, optional
        Swap side: RECEIVE (receive realized, pay strike) or PAY (pay realized,
        receive strike). Default: RECEIVE.
    day_count : DayCount, optional
        Day-count convention for time calculations.

    Returns
    -------
    VarianceSwap
        Configured variance swap ready for pricing.

    Raises
    ------
    ValueError
        If parameters are invalid (maturity <= start_date, strike_variance < 0,
        etc.) or if required market data is missing.

    Examples
    --------
        >>> from finstack.core.dates.schedule import Frequency
        >>> swap = VarianceSwap.create(
        ...     "VAR-SWAP-SPX",
        ...     "SPX",
        ...     Money(1_000_000, Currency("USD")),
        ...     0.04,  # 20% vol squared
        ...     date(2024, 1, 1),
        ...     date(2024, 12, 31),
        ...     discount_curve="USD",
        ...     observation_frequency=Frequency.DAILY
        ... )
    """

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
    def side(self) -> str: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
