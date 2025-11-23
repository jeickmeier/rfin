"""Basis swap instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.dates.daycount import DayCount
from ...core.dates.calendar import BusinessDayConvention
from ..common import InstrumentType

class BasisSwapLeg:
    """Basis swap leg specification."""
    def __init__(
        self,
        forward_curve: str,
        *,
        frequency: Optional[str] = "quarterly",
        day_count: Optional[DayCount] = None,
        business_day_convention: Optional[BusinessDayConvention] = None,
        spread: float = 0.0,
    ) -> None: ...
    @property
    def forward_curve(self) -> str: ...
    @property
    def spread(self) -> float: ...

class BasisSwap:
    """Basis swap for exchanging two floating interest rates.

    BasisSwap represents a swap where both legs pay floating rates, typically
    based on different reference rates (e.g., LIBOR vs SOFR, 3M vs 6M). The
    difference between the two floating rates is the basis spread.

    Basis swaps are used to hedge basis risk, convert between different floating
    rate indices, and manage funding costs. They require forward curves for
    both legs and a discount curve.

    Examples
    --------
    Create a basis swap (LIBOR 3M vs SOFR):

        >>> from finstack.valuations.instruments import BasisSwap, BasisSwapLeg
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> primary_leg = BasisSwapLeg(forward_curve="USD-LIBOR-3M", frequency="quarterly", spread=0.0)
        >>> reference_leg = BasisSwapLeg(
        ...     forward_curve="USD-SOFR",
        ...     frequency="quarterly",
        ...     spread=10.0,  # 10bp basis spread
        ... )
        >>> basis_swap = BasisSwap.create(
        ...     "BASIS-LIBOR-SOFR",
        ...     Money(10_000_000, Currency("USD")),
        ...     start_date=date(2024, 1, 1),
        ...     maturity=date(2029, 1, 1),  # 5-year swap
        ...     primary_leg=primary_leg,
        ...     reference_leg=reference_leg,
        ...     discount_curve="USD",
        ... )

    Notes
    -----
    - Basis swaps require forward curves for both legs
    - Primary leg typically pays the higher rate
    - Reference leg pays the lower rate plus basis spread
    - Basis spread compensates for differences in credit risk, liquidity, etc.
    - Both legs use floating rates (no fixed leg)

    See Also
    --------
    :class:`InterestRateSwap`: Fixed-for-floating swaps
    :class:`ForwardCurve`: Forward rate curves
    :class:`PricerRegistry`: Pricing entry point
    """

    @classmethod
    def create(
        cls,
        instrument_id: str,
        notional: Money,
        start_date: date,
        maturity: date,
        primary_leg: BasisSwapLeg,
        reference_leg: BasisSwapLeg,
        discount_curve: str,
        *,
        calendar: Optional[str] = None,
        stub: Optional[str] = "none",
    ) -> "BasisSwap": ...
    """Create a floating-for-floating basis swap with two legs.

    Parameters
    ----------
    instrument_id : str
        Unique identifier for the basis swap (e.g., "BASIS-LIBOR-SOFR").
    notional : Money
        Notional principal amount. Currency determines curve currency requirements.
    start_date : date
        Swap start date (first accrual date).
    maturity : date
        Swap maturity date (last payment date). Must be after start_date.
    primary_leg : BasisSwapLeg
        Primary leg specification (forward curve, frequency, spread).
        Typically pays the higher floating rate.
    reference_leg : BasisSwapLeg
        Reference leg specification (forward curve, frequency, spread).
        Typically pays the lower floating rate plus basis spread.
    discount_curve : str
        Discount curve identifier in MarketContext for present value calculations.
    calendar : str, optional
        Holiday calendar identifier for payment date adjustments.
    stub : str, optional
        Stub period handling: "none" (default), "short_first", "short_last".

    Returns
    -------
    BasisSwap
        Configured basis swap ready for pricing.

    Raises
    ------
    ValueError
        If dates are invalid (maturity <= start_date), if notional is invalid,
        or if forward curves are not found in MarketContext.

    Examples
    --------
        >>> basis_swap = BasisSwap.create(
        ...     "BASIS-3M-6M",
        ...     Money(10_000_000, Currency("USD")),
        ...     date(2024, 1, 1),
        ...     date(2029, 1, 1),
        ...     primary_leg,
        ...     reference_leg,
        ...     discount_curve="USD"
        ... )
    """

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
