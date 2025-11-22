"""Fixed-income bond instrument with convenience constructors."""

from typing import Optional, List, Tuple, Any, TypedDict
from datetime import date
from ...core.money import Money
from ...core.dates.schedule import Frequency, StubKind
from ...core.dates.daycount import DayCount
from ...core.dates.calendar import BusinessDayConvention
from ...core.cashflow.primitives import AmortizationSpec
from ...core.cashflow.builder import CashFlowSchedule
from ..common import InstrumentType

class CallScheduleItem(TypedDict):
    date: date
    price_pct: float

class PutScheduleItem(TypedDict):
    date: date
    price_pct: float

class Bond:
    """Fixed-income bond instrument with convenience constructors."""

    @classmethod
    def fixed_semiannual(
        cls, instrument_id: str, notional: Money, coupon_rate: float, issue: date, maturity: date, discount_curve: str
    ) -> Bond:
        """Create a semi-annual fixed-rate bond with 30/360 day count and Following BDC.

        Args:
            instrument_id: Instrument identifier or string-like object.
            notional: Notional principal as Money.
            coupon_rate: Annual coupon in decimal form.
            issue: Issue date of the bond.
            maturity: Maturity date of the bond.
            discount_curve: Discount curve identifier for valuation.

        Returns:
            Bond: Configured fixed-rate bond instrument.

        Raises:
            ValueError: If identifiers or dates cannot be parsed.

        Examples:
            >>> bond = Bond.fixed_semiannual(
            ...     "corp_1", Money("USD", 1_000_000), 0.045, date(2023, 1, 1), date(2028, 1, 1), "usd_discount"
            ... )
            >>> bond.coupon
            0.045
        """
        ...

    @classmethod
    def treasury(cls, instrument_id: str, notional: Money, coupon_rate: float, issue: date, maturity: date) -> Bond:
        """Create a U.S. Treasury-style bond with annual coupons and Act/Act ISMA day count.

        Args:
            instrument_id: Instrument identifier or string-like object.
            notional: Notional principal as Money.
            coupon_rate: Annual coupon in decimal form.
            issue: Issue date of the bond.
            maturity: Maturity date of the bond.

        Returns:
            Bond: Configured Treasury-style bond instrument.

        Raises:
            ValueError: If identifiers or dates cannot be parsed.

        Examples:
            >>> Bond.treasury("ust_5y", Money("USD", 1_000), 0.03, date(2024, 1, 1), date(2029, 1, 1))
            Bond(id='ust_5y', coupon=0.03, maturity='2029-01-01')
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
    ) -> Bond:
        """Create a bond via builder parameters. Supports amortization and call/put.

        Args:
            instrument_id: Instrument identifier or string-like object.
            notional: Notional amount as Money.
            issue: Issue date of the bond.
            maturity: Maturity date of the bond.
            discount_curve: Discount curve identifier for valuation.
            coupon_rate: Optional fixed coupon rate in decimal form.
            frequency: Optional payment frequency.
            day_count: Optional day-count convention.
            bdc: Optional business-day convention.
            calendar_id: Optional calendar identifier for scheduling.
            stub: Optional stub kind for schedule construction.
            amortization: Optional amortization specification.
            call_schedule: Optional list of (date, price %) call events.
            put_schedule: Optional list of (date, price %) put events.
            quoted_clean_price: Optional quoted clean price for overrides.
            forward_curve: Optional forward curve identifier for float spec.
            float_margin_bp: Optional floating margin in basis points.
            float_gearing: Optional gearing multiplier for float leg.
            float_reset_lag_days: Optional reset lag in days for float leg.

        Returns:
            Bond: Fully specified bond instrument.

        Raises:
            ValueError: If identifiers or dates cannot be parsed.
            RuntimeError: When the underlying builder detects invalid input.
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
