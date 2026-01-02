"""Interest rate swap instrument."""

from typing import Optional, Union, overload
from datetime import date
from ...core.money import Money
from ...core.currency import Currency
from ...core.dates.schedule import Frequency, StubKind
from ...core.dates.daycount import DayCount
from ...core.dates.calendar import BusinessDayConvention
from ..common import InstrumentType

class PayReceive:
    """Pay/receive direction for swap fixed-leg cashflows."""

    PAY_FIXED: "PayReceive"
    RECEIVE_FIXED: "PayReceive"

    @classmethod
    def from_name(cls, name: str) -> "PayReceive": ...
    @property
    def name(self) -> str: ...

class InterestRateSwap:
    """Plain-vanilla interest rate swap with fixed-for-floating legs.

    InterestRateSwap represents a standard interest rate swap where one
    party pays a fixed rate and receives a floating rate (or vice versa)
    on a specified notional over a given term. Swaps are priced using
    discount curves and forward curves stored in a MarketContext.

    Swaps are the most liquid interest rate derivatives and are used for
    hedging, speculation, and asset-liability management. The fixed rate
    is typically set such that the swap has zero value at inception (par swap).

    Examples
    --------
    Create a USD SOFR swap (pay fixed, receive floating):

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import InterestRateSwap
        >>> swap = InterestRateSwap.usd_pay_fixed(
        ...     "SWAP-001",
        ...     Money(10_000_000, Currency("USD")),
        ...     0.035,
        ...     date(2024, 1, 1),
        ...     date(2029, 1, 1),
        ... )

    Price the swap:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.term_structures import DiscountCurve, ForwardCurve
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import InterestRateSwap
        >>> from finstack.valuations.pricer import create_standard_registry
        >>> swap = InterestRateSwap.usd_pay_fixed(
        ...     "SWAP-EXAMPLE",
        ...     Money(5_000_000, Currency("USD")),
        ...     0.03,
        ...     date(2024, 1, 1),
        ...     date(2029, 1, 1),
        ... )
        >>> ctx = MarketContext()
        >>> ctx.insert_discount(DiscountCurve("USD-OIS", date(2024, 1, 1), [(0.0, 1.0), (5.0, 0.95)]))
        >>> ctx.insert_forward(
        ...     ForwardCurve("USD-SOFR-3M", 0.25, [(0.0, 0.03), (5.0, 0.032)], base_date=date(2024, 1, 1))
        ... )
        >>> registry = create_standard_registry()
        >>> pv = registry.price(swap, "discounting", ctx).value
        >>> pv.currency.code
        'USD'

    Notes
    -----
    - Swaps require both discount and forward curves in MarketContext
    - Fixed leg uses the specified fixed_rate
    - Floating leg uses forward rates from the forward_curve plus any spread
    - Use :meth:`builder` for non-USD swaps or custom conventions
    - Par swap rate can be calculated by solving for fixed_rate = 0 PV

    See Also
    --------
    :class:`Bond`: Fixed-income bond instruments
    :class:`PricerRegistry`: Pricing entry point
    :class:`MarketContext`: Market data container
    """

    @classmethod
    def usd_pay_fixed(
        cls,
        instrument_id: str,
        notional: Money,
        fixed_rate: float,
        start: date,
        end: date,
    ) -> "InterestRateSwap":
        """Create a USD SOFR swap where the caller pays fixed and receives floating.

        Factory method for creating a standard USD interest rate swap using SOFR
        conventions: quarterly floating payments, semi-annual fixed payments,
        30/360 day count, and Following business day convention.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the swap (e.g., "SWAP-001", "IRS-5Y").
        notional : Money
            Notional principal amount. Must be in USD for this factory method.
        fixed_rate : float
            Fixed rate paid by the caller, as a decimal (e.g., 0.035 for 3.5%).
            This is typically the par swap rate at inception.
        start : date
            Swap start date (first accrual date).
        end : date
            Swap end date (last payment date). Must be after start date.

        Returns
        -------
        InterestRateSwap
            Configured swap where the caller pays fixed and receives floating.

        Raises
        ------
        ValueError
            If dates are invalid (end <= start), if fixed_rate is negative,
            or if notional currency is not USD.

        Examples
        --------
            >>> from finstack import Money, Currency
            >>> from datetime import date
            >>> swap = InterestRateSwap.usd_pay_fixed(
            ...     "SWAP-5Y",
            ...     Money(10_000_000, Currency("USD")),
            ...     0.035,  # 3.5% fixed rate
            ...     date(2024, 1, 1),
            ...     date(2029, 1, 1),  # 5-year swap
            ... )
            >>> swap.fixed_rate
            0.035
            >>> swap.side
            PayReceive.PAY_FIXED
        """
        ...

    @classmethod
    def usd_receive_fixed(
        cls,
        instrument_id: str,
        notional: Money,
        fixed_rate: float,
        start: date,
        end: date,
    ) -> "InterestRateSwap":
        """Create a USD SOFR swap where the caller receives fixed."""
        ...

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        notional: Optional[Money] = ...,
        fixed_rate: Optional[float] = ...,
        start: Optional[date] = ...,
        end: Optional[date] = ...,
        side: Optional[Union[PayReceive, str]] = ...,
        discount_curve: Optional[str] = ...,
        forward_curve: Optional[str] = ...,
        *,
        fixed_frequency: Optional[Frequency] = None,
        float_frequency: Optional[Frequency] = None,
        fixed_day_count: Optional[DayCount] = None,
        float_day_count: Optional[DayCount] = None,
        business_day_convention: Optional[BusinessDayConvention] = None,
        float_spread_bp: Optional[float] = None,
        reset_lag_days: Optional[int] = None,
        calendar: Optional[str] = None,
        stub: Optional[StubKind] = None,
    ) -> Union[InterestRateSwapBuilder, "InterestRateSwap"]: ...

class InterestRateSwapBuilder:
    """Fluent builder returned by :meth:`InterestRateSwap.builder` when only an ID is provided."""

    def __init__(self, instrument_id: str) -> None: ...
    def notional(self, amount: float) -> InterestRateSwapBuilder: ...
    def currency(self, currency: Union[str, Currency]) -> InterestRateSwapBuilder: ...
    def money(self, money: Money) -> InterestRateSwapBuilder: ...
    def side(self, side: Union[PayReceive, str]) -> InterestRateSwapBuilder: ...
    def fixed_rate(self, rate: float) -> InterestRateSwapBuilder: ...
    def float_spread_bp(self, spread_bp: float) -> InterestRateSwapBuilder: ...
    def start(self, start: date) -> InterestRateSwapBuilder: ...
    def maturity(self, maturity: date) -> InterestRateSwapBuilder: ...
    def disc_id(self, curve_id: str) -> InterestRateSwapBuilder: ...
    def fwd_id(self, curve_id: str) -> InterestRateSwapBuilder: ...
    def fixed_frequency(self, frequency: Union[Frequency, str, int]) -> InterestRateSwapBuilder: ...
    def float_frequency(self, frequency: Union[Frequency, str, int]) -> InterestRateSwapBuilder: ...
    def frequency(self, frequency: Union[Frequency, str, int]) -> InterestRateSwapBuilder: ...
    def fixed_day_count(self, day_count: Union[DayCount, str]) -> InterestRateSwapBuilder: ...
    def float_day_count(self, day_count: Union[DayCount, str]) -> InterestRateSwapBuilder: ...
    def bdc(self, bdc: Union[BusinessDayConvention, str]) -> InterestRateSwapBuilder: ...
    def stub(self, stub: Union[StubKind, str]) -> InterestRateSwapBuilder: ...
    def calendar(self, calendar_id: Optional[str] = ...) -> InterestRateSwapBuilder: ...
    def reset_lag_days(self, days: int) -> InterestRateSwapBuilder: ...
    def build(self) -> InterestRateSwap: ...
    @property
    def id(self) -> str: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def forward_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
