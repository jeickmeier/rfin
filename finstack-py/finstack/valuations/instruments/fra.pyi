"""Forward rate agreement instrument."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.dates.daycount import DayCount
from ..common import InstrumentType

class ForwardRateAgreement:
    """Forward Rate Agreement for locking in future interest rates.

    ForwardRateAgreement (FRA) is a contract to exchange a fixed rate for a
    floating rate over a future period. The FRA settles on the fixing_date
    based on the difference between the fixed rate and the floating rate
    observed on that date.

    FRAs are used to hedge or speculate on future interest rates. They are
    cash-settled on the fixing date, with the payment based on the rate
    difference and the notional amount.

    Examples
    --------
    Create an FRA:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import ForwardRateAgreement
        >>> fra = ForwardRateAgreement.create(
        ...     "FRA-3M6M",
        ...     notional=Money(10_000_000, Currency("USD")),
        ...     fixed_rate=0.035,
        ...     fixing_date=date(2024, 6, 1),
        ...     start_date=date(2024, 9, 1),
        ...     end_date=date(2024, 12, 1),
        ...     discount_curve="USD-OIS",
        ...     forward_curve="USD-SOFR-3M",
        ... )

    Price the FRA:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.term_structures import DiscountCurve, ForwardCurve
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import ForwardRateAgreement
        >>> from finstack.valuations.pricer import create_standard_registry
        >>> fra = ForwardRateAgreement.create(
        ...     "FRA-3M6M",
        ...     Money(5_000_000, Currency("USD")),
        ...     0.03,
        ...     date(2024, 6, 1),
        ...     date(2024, 9, 1),
        ...     date(2024, 12, 1),
        ...     discount_curve="USD-OIS",
        ...     forward_curve="USD-SOFR-3M",
        ... )
        >>> ctx = MarketContext()
        >>> ctx.insert_discount(DiscountCurve("USD-OIS", date(2024, 1, 1), [(0.0, 1.0), (1.0, 0.97)]))
        >>> ctx.insert_forward(
        ...     ForwardCurve("USD-SOFR-3M", 0.25, [(0.0, 0.03), (1.0, 0.031)], base_date=date(2024, 1, 1))
        ... )
        >>> registry = create_standard_registry()
        >>> pv = registry.price(fra, "discounting", ctx).value
        >>> pv.currency.code
        'USD'

    Notes
    -----
    - FRAs require discount curve and forward curve
    - Fixed rate is locked in at trade date
    - Floating rate is observed on fixing_date
    - Settlement occurs on fixing_date (cash settlement)
    - The period is from start_date to end_date
    - pay_fixed=True means paying fixed, receiving floating

    See Also
    --------
    :class:`InterestRateSwap`: Multi-period interest rate swaps
    :class:`Deposit`: Money-market deposits
    :class:`PricerRegistry`: Pricing entry point
    """

    @classmethod
    def create(
        cls,
        instrument_id: str,
        notional: Money,
        fixed_rate: float,
        fixing_date: date,
        start_date: date,
        end_date: date,
        discount_curve: str,
        forward_curve: str,
        *,
        day_count: Optional[DayCount] = None,
        reset_lag: int = 2,
        pay_fixed: bool = True,
    ) -> "ForwardRateAgreement":
        """Create a standard FRA referencing discount and forward curves.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the FRA (e.g., "FRA-3M6M").
        notional : Money
            Notional principal amount. The currency determines curve currency
            requirements.
        fixed_rate : float
            Fixed rate agreed at trade date, as a decimal (e.g., 0.035 for 3.5%).
            This rate is compared to the floating rate on fixing_date.
        fixing_date : date
            Date when the floating rate is observed and the FRA is settled.
            Typically 2 business days before start_date (reset_lag).
        start_date : date
            Start date of the interest rate period. The floating rate applies
            from this date.
        end_date : date
            End date of the interest rate period. Must be after start_date.
            The period length determines the interest calculation.
        discount_curve : str
            Discount curve identifier in MarketContext for present value calculations.
        forward_curve : str
            Forward curve identifier for projecting the floating rate on fixing_date.
        day_count : DayCount, optional
            Day-count convention for the interest period (default: ACT/360 for
            most money-market conventions).
        reset_lag : int, optional
            Number of days between fixing_date and start_date (default: 2 for T+2).
        pay_fixed : bool, optional
            If True (default), the holder pays fixed and receives floating.
            If False, the holder receives fixed and pays floating.

        Returns
        -------
        ForwardRateAgreement
            Configured FRA ready for pricing.

        Raises
        ------
        ValueError
            If dates are invalid (end_date <= start_date, fixing_date > start_date),
            if fixed_rate is negative, or if notional is invalid.

        Examples
        --------
            >>> from finstack import Money, Currency
            >>> from datetime import date
            >>> fra = ForwardRateAgreement.create(
            ...     "FRA-3M6M",
            ...     Money(10_000_000, Currency("USD")),
            ...     0.035,  # 3.5% fixed
            ...     date(2024, 6, 1),  # Fixing in 3 months
            ...     date(2024, 9, 1),  # Period starts in 6 months
            ...     date(2024, 12, 1),  # Period ends in 9 months (3M period)
            ...     discount_curve="USD",
            ...     forward_curve="USD-LIBOR-3M",
            ... )
            >>> fra.fixed_rate
            0.035
        """
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def fixed_rate(self) -> float: ...
    @property
    def day_count(self) -> DayCount: ...
    @property
    def reset_lag(self) -> int: ...
    @property
    def pay_fixed(self) -> bool: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def forward_curve(self) -> str: ...
    @property
    def fixing_date(self) -> date: ...
    @property
    def start_date(self) -> date: ...
    @property
    def end_date(self) -> date: ...
    @property
    def notional(self) -> Money: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
