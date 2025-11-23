"""Money-market deposit with simple interest accrual."""

from typing import Optional
from datetime import date
from ...core.currency import Currency
from ...core.money import Money
from ...core.dates.daycount import DayCount
from ..common import InstrumentType

class Deposit:
    """Money-market deposit with simple interest accrual.

    Deposit represents a simple money-market instrument where funds are deposited
    at start_date and repaid with interest at end_date. The interest rate can
    be quoted (for market deposits) or derived from the discount curve.

    Deposits are the simplest interest rate instruments and are used as
    building blocks for curve construction and short-term cash management.

    Examples
    --------
    Create a deposit with quoted rate:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.dates.daycount import DayCount
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import Deposit
        >>> deposit = Deposit(
        ...     "DEPO-3M",
        ...     notional=Money(1_000_000, Currency("USD")),
        ...     start=date(2024, 1, 1),
        ...     end=date(2024, 4, 1),  # 3-month deposit
        ...     day_count=DayCount.ACT_360,
        ...     discount_curve="USD",
        ...     quote_rate=0.035,  # 3.5% quoted rate
        ... )

    Create a deposit using curve rate:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.dates.daycount import DayCount
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import Deposit
        >>> deposit = Deposit(
        ...     "DEPO-6M",
        ...     Money(1_000_000, Currency("USD")),
        ...     date(2024, 1, 1),
        ...     date(2024, 7, 1),
        ...     DayCount.ACT_360,
        ...     discount_curve="USD",
        ...     quote_rate=None,
        ... )

    Price the deposit:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.dates.daycount import DayCount
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.term_structures import DiscountCurve
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import Deposit
        >>> from finstack.valuations.pricer import create_standard_registry
        >>> ctx = MarketContext()
        >>> ctx.insert_discount(DiscountCurve("USD", date(2024, 1, 1), [(0.0, 1.0), (0.25, 0.995)]))
        >>> registry = create_standard_registry()
        >>> deposit = Deposit(
        ...     "DEPO-3M",
        ...     Money(1_000_000, Currency("USD")),
        ...     date(2024, 1, 1),
        ...     date(2024, 4, 1),
        ...     DayCount.ACT_360,
        ...     discount_curve="USD",
        ...     quote_rate=0.035,
        ... )
        >>> pv = registry.price(deposit, "discounting", ctx).value
        >>> pv.currency.code
        'USD'

    Notes
    -----
    - Deposits require a discount curve for pricing
    - If quote_rate is provided, it overrides the curve rate
    - Interest accrues from start_date to end_date
    - Day-count convention determines the interest calculation
    - Standard money-market conventions: ACT/360 for USD, ACT/365 for GBP
    - Deposits are typically used for curve bootstrapping

    See Also
    --------
    :class:`ForwardRateAgreement`: Forward rate agreements
    :class:`DiscountCurve`: Discount curves for pricing
    :class:`PricerRegistry`: Pricing entry point
    """

    def __init__(
        self,
        instrument_id: str,
        notional: Money,
        start: date,
        end: date,
        day_count: DayCount,
        discount_curve: str,
        quote_rate: Optional[float] = None,
    ) -> None: ...
    """Create a deposit with explicit start/end dates and optional quoted rate.

    Parameters
    ----------
    instrument_id : str
        Unique identifier for the deposit (e.g., "DEPO-3M", "DEPO-6M").
    notional : Money
        Principal amount deposited. The currency determines the
        discount curve currency requirement.
    start : date
        Deposit start date (funds deposited on this date).
    end : date
        Deposit end date (funds repaid with interest on this date).
        Must be after start_date.
    day_count : DayCount
        Day-count convention for interest accrual (e.g., ACT/360 for USD,
        ACT/365 for GBP).
    discount_curve : str
        Discount curve identifier in MarketContext for pricing. If quote_rate
        is None, the deposit rate is derived from this curve.
    quote_rate : float, optional
        Quoted market rate for the deposit, as a decimal (e.g., 0.035 for 3.5%).
        If provided, this rate is used instead of the curve rate. If None,
        the rate is implied from the discount curve.

    Returns
    -------
    Deposit
        Configured deposit ready for pricing.

    Raises
    ------
    ValueError
        If dates are invalid (end <= start), if quote_rate is negative,
        or if notional is invalid.

    Examples
    --------
        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.dates.daycount import DayCount
        >>> from finstack.core.money import Money
        >>> deposit = Deposit(
        ...     "DEPO-3M",
        ...     Money(1_000_000, Currency("USD")),
        ...     date(2024, 1, 1),
        ...     date(2024, 4, 1),  # 3-month deposit
        ...     DayCount.ACT_360,
        ...     discount_curve="USD",
        ...     quote_rate=0.035  # 3.5% quoted
        ... )
        >>> deposit.quote_rate
        0.035
    """

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def start(self) -> date: ...
    @property
    def end(self) -> date: ...
    @property
    def day_count(self) -> DayCount: ...
    @property
    def quote_rate(self) -> Optional[float]: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
