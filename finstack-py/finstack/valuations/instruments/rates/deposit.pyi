"""Money-market deposit with simple interest accrual (builder-only API)."""

from __future__ import annotations
from datetime import date
from ....core.currency import Currency
from ....core.money import Money
from ....core.dates.daycount import DayCount
from ....core.dates.calendar import BusinessDayConvention
from ...common import InstrumentType

class DepositBuilder:
    """Fluent builder returned by :meth:`Deposit.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def notional(self, amount: float) -> DepositBuilder: ...
    def currency(self, currency: str | Currency) -> DepositBuilder: ...
    def money(self, money: Money) -> DepositBuilder: ...
    def start(self, start: date) -> DepositBuilder: ...
    def maturity(self, maturity: date) -> DepositBuilder: ...
    def day_count(self, day_count: DayCount | str) -> DepositBuilder: ...
    def discount_curve(self, curve_id: str) -> DepositBuilder: ...
    def disc_id(self, curve_id: str) -> DepositBuilder:
        """Deprecated: use :meth:`discount_curve` instead."""
        ...
    def quote_rate(self, quote_rate: float | None = ...) -> DepositBuilder: ...
    def spot_lag_days(self, spot_lag_days: int | None = ...) -> DepositBuilder: ...
    def bdc(self, bdc: BusinessDayConvention | str) -> DepositBuilder: ...
    def calendar(self, calendar: str | None = ...) -> DepositBuilder: ...
    def build(self) -> "Deposit": ...

class Deposit:
    """Money-market deposit with simple interest accrual.

    Deposit represents a simple money-market instrument where funds are deposited
    at start_date and repaid with interest at maturity. The interest rate can
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
        >>> deposit = (
        ...     Deposit
        ...     .builder("DEPO-3M")
        ...     .money(Money(1_000_000, Currency("USD")))
        ...     .start(date(2024, 1, 1))
        ...     .maturity(date(2024, 4, 1))  # 3-month deposit
        ...     .day_count(DayCount.ACT_360)
        ...     .discount_curve("USD")
        ...     .quote_rate(0.035)  # 3.5% quoted rate
        ...     .build()
        ... )

    Create a deposit using curve rate:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.dates.daycount import DayCount
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import Deposit
        >>> deposit = (
        ...     Deposit
        ...     .builder("DEPO-6M")
        ...     .money(Money(1_000_000, Currency("USD")))
        ...     .start(date(2024, 1, 1))
        ...     .maturity(date(2024, 7, 1))
        ...     .day_count(DayCount.ACT_360)
        ...     .discount_curve("USD")
        ...     .quote_rate(None)
        ...     .build()
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
        >>> deposit = (
        ...     Deposit
        ...     .builder("DEPO-3M")
        ...     .money(Money(1_000_000, Currency("USD")))
        ...     .start(date(2024, 1, 1))
        ...     .maturity(date(2024, 4, 1))
        ...     .day_count(DayCount.ACT_360)
        ...     .discount_curve("USD")
        ...     .quote_rate(0.035)
        ...     .build()
        ... )
        >>> pv = registry.price(deposit, "discounting", ctx).value
        >>> pv.currency.code
        'USD'

    Notes
    -----
    - Deposits require a discount curve for pricing
    - If quote_rate is provided, it overrides the curve rate
    - Interest accrues from start_date to maturity
    - Day-count convention determines the interest calculation
    - Standard money-market conventions: ACT/360 for USD, ACT/365 for GBP
    - Deposits are typically used for curve bootstrapping

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required for pricing; used when ``quote_rate`` is None and for discounting).

    See Also
    --------
    :class:`ForwardRateAgreement`: Forward rate agreements
    :class:`DiscountCurve`: Discount curves for pricing
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - Brigo & Mercurio (2006): see ``docs/REFERENCES.md#brigoMercurio2006``.
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> DepositBuilder: ...
    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def start(self) -> date: ...
    @property
    def maturity(self) -> date: ...
    @property
    def day_count(self) -> DayCount: ...
    @property
    def quote_rate(self) -> float | None: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
