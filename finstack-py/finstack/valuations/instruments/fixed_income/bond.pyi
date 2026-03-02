"""Fixed-income bond instrument with convenience constructors."""

from __future__ import annotations
import datetime
from typing import List, Tuple, Any, TypedDict, overload
from ....core.money import Money
from ....core.currency import Currency
from ....core.dates.schedule import Frequency, StubKind
from ....core.dates.daycount import DayCount
from ....core.dates.calendar import BusinessDayConvention
from ....core.market_data.context import MarketContext
from ...cashflow.builder import AmortizationSpec, CashFlowSchedule
from ...common import InstrumentType
from ..credit.mc_config import MertonMcConfig, MertonMcResult

class CallScheduleItem(TypedDict):
    date: datetime.date
    price_pct: float

class PutScheduleItem(TypedDict):
    date: datetime.date
    price_pct: float

class BondSettlementConvention:
    """Bond settlement and ex-coupon conventions."""
    def __init__(
        self,
        settlement_days: int,
        ex_coupon_days: int = 0,
        ex_coupon_calendar_id: str | None = None,
    ) -> None: ...
    @property
    def settlement_days(self) -> int: ...
    @property
    def ex_coupon_days(self) -> int: ...
    @property
    def ex_coupon_calendar_id(self) -> str | None: ...
    def __repr__(self) -> str: ...

class AccrualMethod:
    """Accrual method for bond interest calculation."""

    LINEAR: AccrualMethod
    COMPOUNDED: AccrualMethod
    def __repr__(self) -> str: ...

class MakeWholeSpec:
    """Make-whole call specification."""
    def __init__(self, reference_curve_id: str, spread_bps: float) -> None: ...
    @property
    def reference_curve_id(self) -> str: ...
    @property
    def spread_bps(self) -> float: ...
    def __repr__(self) -> str: ...

class CallPut:
    """Call or put option on a bond."""
    def __init__(
        self,
        date: datetime.date,
        price_pct_of_par: float,
        end_date: datetime.date | None = None,
        make_whole: MakeWholeSpec | None = None,
    ) -> None: ...
    @property
    def date(self) -> datetime.date: ...
    @property
    def price_pct_of_par(self) -> float: ...
    @property
    def end_date(self) -> datetime.date | None: ...
    def __repr__(self) -> str: ...

class CallPutSchedule:
    """Schedule of call and put options for a bond."""
    def __init__(
        self,
        calls: list[CallPut] | None = None,
        puts: list[CallPut] | None = None,
    ) -> None: ...
    def has_options(self) -> bool: ...
    @property
    def call_count(self) -> int: ...
    @property
    def put_count(self) -> int: ...
    def __repr__(self) -> str: ...

class CashflowSpec:
    """Cashflow specification (fixed, floating, or amortizing)."""
    @property
    def spec_type(self) -> str: ...
    def __repr__(self) -> str: ...

class BondBuilder:
    """Fluent builder returned by :meth:`Bond.builder` when only an ID is provided."""

    def __init__(self, instrument_id: str) -> None: ...
    def notional(self, amount: float) -> BondBuilder: ...
    def currency(self, currency: str | Currency) -> BondBuilder: ...
    def money(self, money: Money) -> BondBuilder: ...
    def cashflows(self, schedule: CashFlowSchedule) -> BondBuilder: ...
    def issue(self, issue: datetime.date) -> BondBuilder: ...
    def maturity(self, maturity: datetime.date) -> BondBuilder: ...
    def disc_id(self, curve_id: str) -> BondBuilder: ...
    def credit_curve(self, curve_id: str | None = ...) -> BondBuilder: ...
    def coupon_rate(self, rate: float) -> BondBuilder: ...
    def coupon_type(self, coupon_type: str) -> BondBuilder:
        """Set coupon type: 'cash' (default) or 'pik'."""
        ...
    def frequency(self, frequency: Frequency | str | int) -> BondBuilder: ...
    def day_count(self, day_count: DayCount | str) -> BondBuilder: ...
    def bdc(self, bdc: BusinessDayConvention | str) -> BondBuilder: ...
    def stub(self, stub: StubKind | str) -> BondBuilder: ...
    def calendar(self, calendar_id: str | None = ...) -> BondBuilder: ...
    def amortization(self, amortization: AmortizationSpec | None = ...) -> BondBuilder: ...
    def settlement_convention(self, convention: BondSettlementConvention) -> BondBuilder: ...
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
    def issue(self) -> datetime.date:
        """Issue date for the bond.

        Returns:
            datetime.date: Issue date converted to Python.
        """
        ...

    @property
    def maturity(self) -> datetime.date:
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

    @property
    def settlement_days(self) -> int | None:
        """Number of settlement days (T+n)."""
        ...
    @property
    def ex_coupon_days(self) -> int | None:
        """Number of ex-coupon days before a coupon date."""
        ...
    @property
    def ex_coupon_calendar_id(self) -> str | None:
        """Calendar identifier for ex-coupon day counting."""
        ...
    @property
    def accrual_method(self) -> str:
        """Accrual method ('linear' or 'compounded')."""
        ...
    @property
    def has_call_put(self) -> bool:
        """Whether the bond has any call or put options."""
        ...
    def validate(self) -> None:
        """Validate all bond parameters.

        Raises
        ------
        ValueError
            If validation fails.
        """
        ...
    def get_full_schedule(self, market: MarketContext) -> CashFlowSchedule:
        """Generate the full cashflow schedule for this bond."""
        ...

    @property
    def cashflow_spec(self) -> CashflowSpec: ...
    def price_merton_mc(
        self,
        config: MertonMcConfig,
        discount_rate: float,
        as_of: datetime.date,
    ) -> MertonMcResult:
        """Price the bond using Monte Carlo simulation with structural credit model.

        Parameters
        ----------
        config : MertonMcConfig
            Monte Carlo configuration with Merton model and optional credit specs.
        discount_rate : float
            Risk-free discount rate.
        as_of : datetime.date
            Valuation date.

        Returns
        -------
        MertonMcResult

        Raises
        ------
        ValueError
            If the bond's cashflow spec is not supported or simulation fails.
        """
        ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
