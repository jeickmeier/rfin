"""Interest rate future instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.dates.daycount import DayCount
from ..common import InstrumentType

class InterestRateFuture:
    """Interest rate future for hedging and speculation on future rates.

    InterestRateFuture represents a futures contract on an interest rate,
    typically based on a 3-month rate (e.g., Eurodollar, SOFR futures).
    The future price is quoted as 100 minus the implied rate.

    Interest rate futures are used for hedging interest rate risk, speculating
    on rate movements, and creating synthetic positions. They require discount
    and forward curves for pricing.

    Examples
    --------
    Create an interest rate future:

        >>> from finstack.valuations.instruments import InterestRateFuture
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> future = InterestRateFuture.create(
        ...     "IR-FUTURE-DEC24",
        ...     notional=Money(1_000_000, Currency("USD")),
        ...     quoted_price=96.50,  # Implies 3.5% rate (100 - 96.50)
        ...     expiry=date(2024, 12, 15),
        ...     fixing_date=date(2024, 12, 16),
        ...     period_start=date(2024, 12, 18),
        ...     period_end=date(2025, 3, 18),  # 3-month period
        ...     discount_curve="USD",
        ...     forward_curve="USD-LIBOR-3M",
        ... )

    Notes
    -----
    - Interest rate futures require discount curve and forward curve
    - Quoted price = 100 - implied rate (e.g., 96.50 = 3.5% rate)
    - Face value is the contract size (typically $1M for Eurodollars)
    - Tick size is the minimum price movement (typically 0.0025 = 1bp)
    - Convexity adjustment accounts for futures vs forward rate differences
    - Position: "long" (default) or "short"

    See Also
    --------
    :class:`ForwardRateAgreement`: Forward rate agreements
    :class:`InterestRateSwap`: Interest rate swaps
    :class:`PricerRegistry`: Pricing entry point
    """

    @classmethod
    def create(
        cls,
        instrument_id: str,
        notional: Money,
        quoted_price: float,
        expiry: date,
        fixing_date: date,
        period_start: date,
        period_end: date,
        discount_curve: str,
        forward_curve: str,
        *,
        position: Optional[str] = "long",
        day_count: Optional[DayCount] = None,
        face_value: float = 1_000_000.0,
        tick_size: float = 0.0025,
        tick_value: Optional[float] = None,
        delivery_months: int = 3,
        convexity_adjustment: Optional[float] = None,
    ) -> "InterestRateFuture":
        """Create an interest rate future.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the future (e.g., "IR-FUTURE-DEC24").
        notional : Money
            Notional amount. Currency determines curve currency requirements.
        quoted_price : float
            Quoted futures price (typically 100 - rate, e.g., 96.50 for 3.5%).
            Must be in range [0, 100].
        expiry : date
            Future expiration date (last trading day).
        fixing_date : date
            Date when the underlying rate is fixed (typically expiry + 1 day).
        period_start : date
            Start date of the interest rate period.
        period_end : date
            End date of the interest rate period. Must be after period_start.
        discount_curve : str
            Discount curve identifier in MarketContext.
        forward_curve : str
            Forward curve identifier for the underlying rate.
        position : str, optional
            Position direction: "long" (default, benefit from rate increase) or
            "short" (benefit from rate decrease).
        day_count : DayCount, optional
            Day-count convention for the interest period (default: ACT/360).
        face_value : float, optional
            Contract face value (default: 1,000,000 for standard contracts).
        tick_size : float, optional
            Minimum price movement (default: 0.0025 = 1 basis point).
        tick_value : float, optional
            Dollar value per tick. If None, calculated from face_value and tick_size.
        delivery_months : int, optional
            Number of months in the delivery period (default: 3 for quarterly).
        convexity_adjustment : float, optional
            Convexity adjustment for futures vs forward rate (typically negative).
            If None, calculated automatically.

        Returns
        -------
        InterestRateFuture
            Configured interest rate future ready for pricing.

        Raises
        ------
        ValueError
            If dates are invalid, if quoted_price is not in [0, 100], or if
            required curves are not found in MarketContext.

        Examples
        --------
            >>> future = InterestRateFuture.create(
            ...     "IR-FUTURE-DEC24",
            ...     Money(1_000_000, Currency("USD")),
            ...     96.50,  # 3.5% implied rate
            ...     date(2024, 12, 15),
            ...     date(2024, 12, 16),
            ...     date(2024, 12, 18),
            ...     date(2025, 3, 18),
            ...     discount_curve="USD",
            ...     forward_curve="USD-LIBOR-3M",
            ... )
        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def quoted_price(self) -> float: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
