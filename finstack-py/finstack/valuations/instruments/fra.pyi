"""Forward rate agreement instrument (builder-only API)."""

from typing import Optional, Union
from datetime import date
from ...core.currency import Currency
from ...core.money import Money
from ...core.dates.daycount import DayCount
from ..common import InstrumentType

class ForwardRateAgreementBuilder:
    """Fluent builder returned by :meth:`ForwardRateAgreement.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def notional(self, amount: float) -> ForwardRateAgreementBuilder: ...
    def currency(self, currency: Union[str, Currency]) -> ForwardRateAgreementBuilder: ...
    def money(self, money: Money) -> ForwardRateAgreementBuilder: ...
    def fixed_rate(self, rate: float) -> ForwardRateAgreementBuilder: ...
    def fixing_date(self, fixing_date: date) -> ForwardRateAgreementBuilder: ...
    def start_date(self, start_date: date) -> ForwardRateAgreementBuilder: ...
    def end_date(self, end_date: date) -> ForwardRateAgreementBuilder: ...
    def disc_id(self, curve_id: str) -> ForwardRateAgreementBuilder: ...
    def fwd_id(self, curve_id: str) -> ForwardRateAgreementBuilder: ...
    def day_count(self, day_count: Union[DayCount, str]) -> ForwardRateAgreementBuilder: ...
    def reset_lag(self, reset_lag: int) -> ForwardRateAgreementBuilder: ...
    def pay_fixed(self, pay_fixed: bool) -> ForwardRateAgreementBuilder: ...
    def build(self) -> "ForwardRateAgreement": ...

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
        >>> fra = (
        ...     ForwardRateAgreement.builder("FRA-3M6M")
        ...     .money(Money(10_000_000, Currency("USD")))
        ...     .fixed_rate(0.035)
        ...     .fixing_date(date(2024, 6, 1))
        ...     .start_date(date(2024, 9, 1))
        ...     .end_date(date(2024, 12, 1))
        ...     .disc_id("USD-OIS")
        ...     .fwd_id("USD-SOFR-3M")
        ...     .build()
        ... )

    Price the FRA:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.term_structures import DiscountCurve, ForwardCurve
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import ForwardRateAgreement
        >>> from finstack.valuations.pricer import create_standard_registry
        >>> fra = (
        ...     ForwardRateAgreement.builder("FRA-3M6M")
        ...     .money(Money(5_000_000, Currency("USD")))
        ...     .fixed_rate(0.03)
        ...     .fixing_date(date(2024, 6, 1))
        ...     .start_date(date(2024, 9, 1))
        ...     .end_date(date(2024, 12, 1))
        ...     .disc_id("USD-OIS")
        ...     .fwd_id("USD-SOFR-3M")
        ...     .build()
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

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required).
    - Forward curve: ``forward_curve`` (required).

    See Also
    --------
    :class:`InterestRateSwap`: Multi-period interest rate swaps
    :class:`Deposit`: Money-market deposits
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - ISDA (2006) Definitions: see ``docs/REFERENCES.md#isda2006Definitions``.
    - Brigo & Mercurio (2006): see ``docs/REFERENCES.md#brigoMercurio2006``.
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> ForwardRateAgreementBuilder: ...
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
