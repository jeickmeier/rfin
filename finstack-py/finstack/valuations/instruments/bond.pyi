"""Fixed-income bond instrument with convenience constructors."""

from typing import Optional, List, Tuple, Any, TypedDict, Union, overload
from datetime import date
from ...core.money import Money
from ...core.currency import Currency
from ...core.dates.schedule import Frequency, StubKind
from ...core.dates.daycount import DayCount
from ...core.dates.calendar import BusinessDayConvention
from ..cashflow.builder import AmortizationSpec, CashFlowSchedule
from ..common import InstrumentType

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
    def currency(self, currency: Union[str, Currency]) -> BondBuilder: ...
    def money(self, money: Money) -> BondBuilder: ...
    def issue(self, issue: date) -> BondBuilder: ...
    def maturity(self, maturity: date) -> BondBuilder: ...
    def disc_id(self, curve_id: str) -> BondBuilder: ...
    def credit_curve(self, curve_id: Optional[str] = ...) -> BondBuilder: ...
    def coupon_rate(self, rate: float) -> BondBuilder: ...
    def frequency(self, frequency: Union[Frequency, str, int]) -> BondBuilder: ...
    def day_count(self, day_count: Union[DayCount, str]) -> BondBuilder: ...
    def bdc(self, bdc: Union[BusinessDayConvention, str]) -> BondBuilder: ...
    def stub(self, stub: Union[StubKind, str]) -> BondBuilder: ...
    def calendar(self, calendar_id: Optional[str] = ...) -> BondBuilder: ...
    def amortization(self, amortization: Optional[AmortizationSpec] = ...) -> BondBuilder: ...
    def call_schedule(self, schedule: List[CallScheduleItem]) -> BondBuilder: ...
    def put_schedule(self, schedule: List[PutScheduleItem]) -> BondBuilder: ...
    def quoted_clean_price(self, price: Optional[float] = ...) -> BondBuilder: ...
    def forward_curve(self, curve_id: Optional[str] = ...) -> BondBuilder: ...
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
        >>> bond = Bond.fixed_semiannual(
        ...     "CORP-001",
        ...     Money(1_000_000, Currency("USD")),
        ...     0.045,  # 4.5% annual coupon
        ...     date(2023, 1, 1),
        ...     date(2028, 1, 1),
        ...     "USD",  # Discount curve ID
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
        >>> bond = Bond.fixed_semiannual(
        ...     "CORP-001",
        ...     Money(1_000_000, Currency("USD")),
        ...     0.045,
        ...     date(2023, 1, 1),
        ...     date(2028, 1, 1),
        ...     "USD",
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
    def fixed_semiannual(
        cls, instrument_id: str, notional: Money, coupon_rate: float, issue: date, maturity: date, discount_curve: str
    ) -> Bond:
        """Create a semi-annual fixed-rate bond with standard conventions.

        Factory method for creating a fixed-rate bond with semi-annual coupon
        payments, 30/360 day count, and Following business day convention.
        This is the most common bond structure for corporate and government bonds.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the bond (e.g., "CORP-001", "UST-5Y").
        notional : Money
            Principal amount of the bond. The currency determines the discount
            curve currency requirement.
        coupon_rate : float
            Annual coupon rate as a decimal (e.g., 0.045 for 4.5%).
        issue : date
            Issue date of the bond (first accrual date).
        maturity : date
            Maturity date when principal is repaid. Must be after issue date.
        discount_curve : str
            Identifier of the discount curve in MarketContext used for valuation.
            The curve currency should match the bond's currency.

        Returns
        -------
        Bond
            Configured fixed-rate bond with semi-annual payments.

        Raises
        ------
        ValueError
            If dates are invalid (maturity <= issue), if coupon_rate is negative,
            or if notional amount is invalid.

        Examples
        --------
            >>> from finstack import Money, Currency
            >>> from datetime import date
            >>> bond = Bond.fixed_semiannual(
            ...     "CORP-001",
            ...     Money(1_000_000, Currency("USD")),
            ...     0.045,  # 4.5% coupon
            ...     date(2023, 1, 1),
            ...     date(2028, 1, 1),  # 5-year bond
            ...     "USD",
            ... )
            >>> bond.coupon
            0.045
            >>> bond.maturity
            datetime.date(2028, 1, 1)

        Sources
        -------
        - ISDA day count conventions: see ``docs/REFERENCES.md#isdaDayCount``.
        """
        ...

    @classmethod
    def treasury(cls, instrument_id: str, notional: Money, coupon_rate: float, issue: date, maturity: date) -> Bond:
        """Create a U.S. Treasury-style bond with annual coupons and Act/Act ISMA day count.

        Factory method for creating a bond matching U.S. Treasury conventions:
        annual coupon payments, Act/Act ISMA day count, and Following business
        day convention. The discount curve defaults to the bond's currency.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the bond (e.g., "UST-5Y", "T-2030").
        notional : Money
            Principal amount. Typically USD for U.S. Treasuries.
        coupon_rate : float
            Annual coupon rate as a decimal (e.g., 0.03 for 3%).
        issue : date
            Issue date of the bond.
        maturity : date
            Maturity date when principal is repaid.

        Returns
        -------
        Bond
            Configured Treasury-style bond with annual payments and Act/Act
        day count.

        Raises
        ------
        ValueError
            If dates are invalid or coupon_rate is negative.

        Examples
        --------
            >>> from finstack import Money, Currency
            >>> from datetime import date
            >>> bond = Bond.treasury(
            ...     "UST-5Y",
            ...     Money(1_000_000, Currency("USD")),
            ...     0.03,  # 3% coupon
            ...     date(2024, 1, 1),
            ...     date(2029, 1, 1),
            ... )
            >>> bond.coupon
            0.03
        """
        ...

    @classmethod
    def zero_coupon(cls, instrument_id: str, notional: Money, issue: date, maturity: date, discount_curve: str) -> Bond:
        """Create a zero-coupon bond discounted off discount_curve.

        Args:
            instrument_id: Instrument identifier or string-like object.
            notional: Redemption amount as Money.
            issue: Issue date of the bond.
            maturity: Maturity date of the bond.
            discount_curve: Discount curve identifier for valuation.

        Returns:
            Bond: Configured zero-coupon bond instrument.

        Raises:
            ValueError: If identifiers or dates cannot be parsed.
        """
        ...

    @classmethod
    @overload
    def builder(cls, instrument_id: str) -> BondBuilder: ...
    @classmethod
    @overload
    def builder(
        cls,
        instrument_id: str,
        notional: Money,
        issue: date,
        maturity: date,
        discount_curve: str,
        *,
        coupon_rate: Optional[float] = None,
        frequency: Optional[Frequency] = None,
        day_count: Optional[DayCount] = None,
        bdc: Optional[BusinessDayConvention] = None,
        calendar_id: Optional[str] = None,
        stub: Optional[StubKind] = None,
        amortization: Optional[AmortizationSpec] = None,
        call_schedule: Optional[List[CallScheduleItem]] = None,
        put_schedule: Optional[List[PutScheduleItem]] = None,
        quoted_clean_price: Optional[float] = None,
        forward_curve: Optional[str] = None,
        float_margin_bp: Optional[float] = None,
        float_gearing: Optional[float] = None,
        float_reset_lag_days: Optional[int] = None,
    ) -> Bond: ...
    @classmethod
    def builder(
        cls,
        instrument_id: str,
        notional: Optional[Money] = ...,
        issue: Optional[date] = ...,
        maturity: Optional[date] = ...,
        discount_curve: Optional[str] = ...,
        *,
        coupon_rate: Optional[float] = None,
        frequency: Optional[Frequency] = None,
        day_count: Optional[DayCount] = None,
        bdc: Optional[BusinessDayConvention] = None,
        calendar_id: Optional[str] = None,
        stub: Optional[StubKind] = None,
        amortization: Optional[AmortizationSpec] = None,
        call_schedule: Optional[List[CallScheduleItem]] = None,
        put_schedule: Optional[List[PutScheduleItem]] = None,
        quoted_clean_price: Optional[float] = None,
        forward_curve: Optional[str] = None,
        float_margin_bp: Optional[float] = None,
        float_gearing: Optional[float] = None,
        float_reset_lag_days: Optional[int] = None,
    ) -> Union[BondBuilder, Bond]:
        """Create a fully customizable bond with advanced features.

        Calling :meth:`Bond.builder` with only ``instrument_id`` returns a
        :class:`BondBuilder` for fluent chaining::

            >>> bond = (
            ...     Bond.builder("CORP-2029")
            ...     .notional(1_000_000.0)
            ...     .currency("USD")
            ...     .coupon_rate(0.045)
            ...     .issue(date(2024, 1, 1))
            ...     .maturity(date(2029, 1, 1))
            ...     .disc_id("USD-OIS")
            ...     .build()
            ... )

        Builder method supporting all bond features including amortization,
        call/put schedules, floating-rate specifications, and custom conventions.
        Use this when the convenience constructors (:meth:`fixed_semiannual`,
        :meth:`treasury`, etc.) don't meet your requirements.

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the bond.
        notional : Money
            Principal amount of the bond.
        issue : date
            Issue date (first accrual date).
        maturity : date
            Maturity date (must be after issue).
        discount_curve : str
            Discount curve identifier for valuation.
        coupon_rate : float, optional
            Fixed coupon rate in decimal form. If None, creates a zero-coupon
            bond (unless floating-rate spec is provided).
        frequency : Frequency, optional
            Coupon payment frequency (e.g., Frequency.SEMI_ANNUAL, Frequency.QUARTERLY).
            Defaults to semi-annual if not specified.
        day_count : DayCount, optional
            Day-count convention for accrual calculations. Defaults to 30/360.
        bdc : BusinessDayConvention, optional
            Business day convention for payment dates. Defaults to Following.
        calendar_id : str, optional
            Holiday calendar identifier for business day adjustments (e.g., "USNY").
        stub : StubKind, optional
            Stub period handling for first/last periods (e.g., StubKind.SHORT_FIRST).
        amortization : AmortizationSpec, optional
            Amortization schedule for principal repayment over time (not bullet).
        call_schedule : List[CallScheduleItem], optional
            List of call dates and prices. Each item is a dict with "date" and
            "price_pct" keys. The issuer can call the bond at these dates.
        put_schedule : List[PutScheduleItem], optional
            List of put dates and prices. Each item is a dict with "date" and
            "price_pct" keys. The holder can put the bond back at these dates.
        quoted_clean_price : float, optional
            Market clean price override (as a percentage of par, e.g., 101.5 for 101.5%).
            Used for yield calculations and market-relative pricing.
        forward_curve : str, optional
            Forward curve identifier for floating-rate bonds. Required for FRNs.
        float_margin_bp : float, optional
            Floating margin in basis points (e.g., 25.0 for 25bp). Added to the
            forward rate for each reset period.
        float_gearing : float, optional
            Gearing multiplier for floating leg (e.g., 1.5 for 150% of the index rate).
            Defaults to 1.0.
        float_reset_lag_days : int, optional
            Number of days between reset date and payment date (settlement lag).
            Defaults to 2 (T+2) for most markets.

        Returns
        -------
        Bond
            Fully specified bond instrument with all requested features.

        Raises
        ------
        ValueError
            If dates are invalid, if required parameters are missing, or if
            schedules are malformed.
        RuntimeError
            If the underlying builder detects invalid input combinations.

        Examples
        --------
        Callable bond:

            >>> from finstack.core.dates.schedule import Frequency
            >>> from finstack.core.dates.daycount import DayCount
            >>> call_schedule = [
            ...     {"date": date(2026, 1, 1), "price_pct": 100.0},
            ...     {"date": date(2027, 1, 1), "price_pct": 100.0},
            ... ]
            >>> bond = Bond.builder(
            ...     "CALLABLE-001",
            ...     Money(1_000_000, Currency("USD")),
            ...     date(2023, 1, 1),
            ...     date(2028, 1, 1),
            ...     "USD",
            ...     coupon_rate=0.05,
            ...     frequency=Frequency.SEMI_ANNUAL,
            ...     call_schedule=call_schedule,
            ... )

        Amortizing bond:

            >>> from finstack.core.cashflow.primitives import AmortizationSpec
            >>> amort = AmortizationSpec.linear(maturity, 0.2)  # 20% per year
            >>> bond = Bond.builder(
            ...     "AMORT-001",
            ...     Money(1_000_000, Currency("USD")),
            ...     date(2023, 1, 1),
            ...     date(2028, 1, 1),
            ...     "USD",
            ...     coupon_rate=0.04,
            ...     amortization=amort,
            ... )

        Notes
        -----
        - Use convenience methods (:meth:`fixed_semiannual`, :meth:`treasury`) for
          simple bonds
        - Call/put schedules affect optionality and require appropriate pricing
        - Amortization affects principal cashflows over time
        - Floating-rate bonds require both discount_curve and forward_curve
        - Quoted clean price is used for yield-to-maturity calculations
        """
        ...

    @classmethod
    def from_cashflows(
        cls,
        instrument_id: str,
        schedule: CashFlowSchedule,
        discount_curve: str,
        quoted_clean: Optional[float] = None,
        forward_curve: Optional[str] = None,
        float_margin_bp: Optional[float] = None,
        float_gearing: Optional[float] = None,
        float_reset_lag_days: Optional[int] = None,
    ) -> Bond:
        """Create a bond from a pre-built CashFlowSchedule (supports PIK, amort, custom coupons).
        Optionally attach a floating-rate spec (forward curve + margin) so ASW metrics
        can build a custom-swap on the same schedule.
        """
        ...

    @classmethod
    def floating(
        cls,
        instrument_id: str,
        notional: Money,
        issue: date,
        maturity: date,
        discount_curve: str,
        forward_curve: str,
        margin_bp: float,
    ) -> Bond:
        """Create a floating-rate note (FRN) using SOFR-like conventions."""
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
    def hazard_curve(self) -> Optional[str]:
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
