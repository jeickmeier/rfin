"""Fixed-income bond instrument with convenience constructors."""

from __future__ import annotations
from typing import List, Tuple, Any, TypedDict, overload
from datetime import date
from ....core.money import Money
from ....core.currency import Currency
from ....core.dates.schedule import Frequency, StubKind
from ....core.dates.daycount import DayCount
from ....core.dates.calendar import BusinessDayConvention
from ...cashflow.builder import AmortizationSpec, CashFlowSchedule
from ...common import InstrumentType

class CallScheduleItem(TypedDict):
    date: date
    price_pct: float

class PutScheduleItem(TypedDict):
    date: date
    price_pct: float

class BondBuilder:
    """Fluent builder returned by :meth:`Bond.builder` when only an ID is provided."""

    def __init__(self, instrument_id: str) -> None: ...
    def notional(self, amount: float) -> BondBuilder: ...
    def currency(self, currency: str | Currency) -> BondBuilder: ...
    def money(self, money: Money) -> BondBuilder: ...
    def cashflows(self, schedule: CashFlowSchedule) -> BondBuilder: ...
    def issue(self, issue: date) -> BondBuilder: ...
    def maturity(self, maturity: date) -> BondBuilder: ...
    def disc_id(self, curve_id: str) -> BondBuilder: ...
    def credit_curve(self, curve_id: str | None = ...) -> BondBuilder: ...
    def coupon_rate(self, rate: float) -> BondBuilder: ...
    def frequency(self, frequency: Frequency | str | int) -> BondBuilder: ...
    def day_count(self, day_count: DayCount | str) -> BondBuilder: ...
    def bdc(self, bdc: BusinessDayConvention | str) -> BondBuilder: ...
    def stub(self, stub: StubKind | str) -> BondBuilder: ...
    def calendar(self, calendar_id: str | None = ...) -> BondBuilder: ...
    def amortization(self, amortization: AmortizationSpec | None = ...) -> BondBuilder: ...
    def call_schedule(self, schedule: List[CallScheduleItem]) -> BondBuilder: ...
    def put_schedule(self, schedule: List[PutScheduleItem]) -> BondBuilder: ...
    def quoted_clean_price(self, price: float | None = ...) -> BondBuilder: ...
    def forward_curve(self, curve_id: str | None = ...) -> BondBuilder: ...
    def float_margin_bp(self, margin_bp: float) -> BondBuilder: ...
    def float_gearing(self, gearing: float) -> BondBuilder: ...
    def float_reset_lag_days(self, lag_days: int) -> BondBuilder: ...
    def build(self) -> "Bond": ...

class Bond:
    """Fixed-income bond instrument for pricing and risk analysis.

    Bond represents a fixed or floating-rate debt instrument with scheduled
    coupon payments and principal repayment. It supports various bond types
    including fixed-rate, floating-rate, zero-coupon, callable, puttable,
    and amortizing bonds.

    Bonds are priced using discount curves (and optionally forward curves for
    floating-rate bonds) stored in a MarketContext. The instrument generates
    cashflows based on its schedule and can be valued using various pricing
    models (discounting, credit-adjusted, etc.).

    Examples
    --------
    Create a simple fixed-rate bond:

        >>> from finstack.valuations.instruments import Bond
        >>> from finstack import Money, Currency
        >>> from datetime import date
        >>> bond = (
        ...     Bond
        ...     .builder("CORP-001")
        ...     .money(Money(1_000_000, Currency("USD")))
        ...     .coupon_rate(0.045)  # 4.5% annual coupon
        ...     .issue(date(2023, 1, 1))
        ...     .maturity(date(2028, 1, 1))
        ...     .disc_id("USD")  # Discount curve ID
        ...     .build()
        ... )
        >>> print(bond.coupon)
        0.045

    Price the bond:

        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.market_data.context import MarketContext
        >>> from finstack.core.market_data.term_structures import DiscountCurve
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.instruments import Bond
        >>> from finstack.valuations.pricer import create_standard_registry
        >>> bond = (
        ...     Bond
        ...     .builder("CORP-001")
        ...     .money(Money(1_000_000, Currency("USD")))
        ...     .coupon_rate(0.045)
        ...     .issue(date(2023, 1, 1))
        ...     .maturity(date(2028, 1, 1))
        ...     .disc_id("USD")
        ...     .build()
        ... )
        >>> ctx = MarketContext()
        >>> curve = DiscountCurve("USD", date(2024, 1, 1), [(0.0, 1.0), (5.0, 0.97)])
        >>> ctx.insert_discount(curve)
        >>> registry = create_standard_registry()
        >>> result = registry.price(bond, "discounting", ctx)
        >>> result.value.currency.code
        'USD'

    Notes
    -----
    - Bonds require a discount curve in MarketContext for pricing
    - Floating-rate bonds also require a forward curve
    - Use :meth:`builder` for full customization (amortization, calls, puts)
    - Credit-sensitive pricing requires a hazard curve in MarketContext
    - Bond cashflows are generated based on the payment schedule

    MarketContext Requirements
    -------------------------
    - Discount curve: ``discount_curve`` (required).
    - Forward curve: ``forward_curve`` (required for floating-rate bonds).
    - Hazard/credit curve: ``hazard_curve`` (required for credit-sensitive pricing, when set).

    See Also
    --------
    :class:`InterestRateSwap`: Interest rate swap instruments
    :class:`PricerRegistry`: Pricing entry point
    :class:`MarketContext`: Market data container

    Sources
    -------
    - ISDA day count conventions: see ``docs/REFERENCES.md#isdaDayCount``.
    - ISDA (2006) Definitions: see ``docs/REFERENCES.md#isda2006Definitions``.
    """

    @classmethod
    def builder(cls, instrument_id: str) -> BondBuilder:
        """Start a fluent builder (builder-only API)."""
        ...

    @property
    def id(self) -> str: ...
    @property
    def instrument_id(self) -> str:
        """Instrument identifier.

        Returns:
            str: Unique identifier assigned to the instrument.
        """
        ...

    @property
    def notional(self) -> Money:
        """Notional principal amount.

        Returns:
            Money: Notional wrapped as Money.
        """
        ...

    @property
    def coupon(self) -> float:
        """Annual coupon rate in decimal form.

        Returns:
            float: Annual coupon rate.
        """
        ...

    @property
    def issue(self) -> date:
        """Issue date for the bond.

        Returns:
            datetime.date: Issue date converted to Python.
        """
        ...

    @property
    def maturity(self) -> date:
        """Maturity date.

        Returns:
            datetime.date: Maturity date converted to Python.
        """
        ...

    @property
    def discount_curve(self) -> str:
        """Discount curve identifier.

        Returns:
            str: Identifier for the discount curve.
        """
        ...

    @property
    def hazard_curve(self) -> str | None:
        """Optional hazard curve identifier enabling credit-sensitive pricing.

        Returns:
            str | None: Hazard curve identifier when provided.
        """
        ...

    @property
    def instrument_type(self) -> InstrumentType:
        """Instrument type enum (InstrumentType.BOND).

        Returns:
            InstrumentType: Enumeration value identifying the instrument family.
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
