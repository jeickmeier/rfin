"""Valuations cash-flow builder exposing complex coupon windows, PIK splits, and amortization."""

from typing import List, Optional, Any, Dict, Tuple
from datetime import date
from ...core.currency import Currency
from ...core.money import Money
from ...core.dates.schedule import Frequency, StubKind
from ...core.dates.daycount import DayCount
from ...core.dates.calendar import BusinessDayConvention
from ...core.cashflow.primitives import CashFlow
from ...core.market_data.context import MarketContext

class AmortizationSpec:
    """Amortization specification for principal payments.

    Use class methods to create specific types.
    """

    @classmethod
    def none(cls) -> AmortizationSpec:
        """No amortization: principal remains until redemption."""
        ...

    @classmethod
    def linear_to(cls, final_notional: Money) -> AmortizationSpec:
        """Linear amortization to final notional.

        Args:
            final_notional: Final notional amount.
        """
        ...

    @classmethod
    def step_remaining(
        cls,
        schedule: List[Tuple[date | str, Money]],
    ) -> AmortizationSpec:
        """Step amortization with remaining notional.

        Args:
            schedule: List of (date, remaining_notional) pairs.
        """
        ...

    @classmethod
    def percent_per_period(cls, pct: float) -> AmortizationSpec:
        """Percentage amortization per period.

        Args:
            pct: Percentage per period (e.g., 0.05 = 5%).
        """
        ...

    @classmethod
    def custom_principal(
        cls,
        items: List[Tuple[date | str, Money]],
    ) -> AmortizationSpec:
        """Custom principal amortization.

        Args:
            items: List of (date, principal_amount) pairs.
        """
        ...

    def __repr__(self) -> str: ...

class CouponType:
    """Coupon split type (cash, PIK, split) mirroring valuations builder."""

    # Class attributes
    CASH: CouponType
    PIK: CouponType

    @classmethod
    def split(cls, cash_pct: float, pik_pct: float) -> CouponType:
        """Create a split coupon type with percentage weights summing to ~1.0."""
        ...

class ScheduleParams:
    """Schedule parameter bundle."""

    def __init__(
        self,
        freq: Frequency,
        day_count: DayCount,
        bdc: BusinessDayConvention,
        calendar_id: Optional[str] = None,
        stub: Optional[StubKind] = None,
    ) -> None:
        """Create schedule parameters.

        Args:
            freq: Payment frequency
            day_count: Day count convention
            bdc: Business day convention
            calendar_id: Optional calendar identifier
            stub: Optional stub kind
        """
        ...

    @classmethod
    def quarterly_act360(cls) -> ScheduleParams:
        """Quarterly payments with Act/360 day count."""
        ...

    @classmethod
    def semiannual_30360(cls) -> ScheduleParams:
        """Semi-annual payments with 30/360 day count."""
        ...

    @classmethod
    def annual_actact(cls) -> ScheduleParams:
        """Annual payments with Act/Act day count."""
        ...

    @classmethod
    def usd_standard(cls) -> ScheduleParams:
        """USD market standard: quarterly, Act/360, Modified Following, USD calendar.

        Returns:
            ScheduleParams: USD standard configuration
        """
        ...

    @classmethod
    def eur_standard(cls) -> ScheduleParams:
        """EUR market standard: semi-annual, 30/360, Modified Following, EUR calendar.

        Returns:
            ScheduleParams: EUR standard configuration
        """
        ...

    @classmethod
    def gbp_standard(cls) -> ScheduleParams:
        """GBP market standard: semi-annual, Act/365, Modified Following, GBP calendar.

        Returns:
            ScheduleParams: GBP standard configuration
        """
        ...

    @classmethod
    def jpy_standard(cls) -> ScheduleParams:
        """JPY market standard: semi-annual, Act/365, Modified Following, JPY calendar.

        Returns:
            ScheduleParams: JPY standard configuration
        """
        ...

class FixedCouponSpec:
    """Fixed coupon specification."""

    @classmethod
    def new(
        cls,
        rate: float,
        schedule: ScheduleParams,
        coupon_type: Optional[CouponType] = None,
    ) -> FixedCouponSpec:
        """Create fixed coupon specification.

        Args:
            rate: Fixed coupon rate
            schedule: Schedule parameters
            coupon_type: Optional coupon type (default: cash)
        """
        ...

class FloatCouponParams:
    """Floating coupon parameters and spec."""

    @classmethod
    def new(
        cls,
        index_id: str,
        margin_bp: float,
        *,
        gearing: float = 1.0,
        reset_lag_days: int = 2,
    ) -> FloatCouponParams:
        """Create floating coupon parameters.

        Args:
            index_id: Curve identifier for the floating rate index
            margin_bp: Margin in basis points
            gearing: Gearing factor (default: 1.0)
            reset_lag_days: Reset lag in days (default: 2)
        """
        ...

class FloatingCouponSpec:
    """Floating coupon specification."""

    @classmethod
    def new(
        cls,
        params: FloatCouponParams,
        schedule: ScheduleParams,
        coupon_type: Optional[CouponType] = None,
    ) -> FloatingCouponSpec:
        """Create floating coupon specification.

        Args:
            params: Floating rate parameters
            schedule: Schedule parameters
            coupon_type: Optional coupon type (default: cash)
        """
        ...

class CashflowBuilder:
    """Python wrapper for the composable valuations CashflowBuilder."""

    @classmethod
    def new(cls) -> CashflowBuilder:
        """Create a new cashflow builder."""
        ...

    def principal(self, amount: float, currency: Currency, issue: date, maturity: date) -> CashflowBuilder:
        """Add principal cashflow.

        Args:
            amount: Principal amount
            currency: Currency
            issue: Issue date
            maturity: Maturity date

        Returns:
            CashflowBuilder: Self for method chaining
        """
        ...

    def amortization(self, amortization: Optional[AmortizationSpec]) -> CashflowBuilder:
        """Add amortization specification.

        Args:
            amortization: Optional amortization spec

        Returns:
            CashflowBuilder: Self for method chaining
        """
        ...

    def fixed_cf(self, spec: FixedCouponSpec) -> CashflowBuilder:
        """Add fixed coupon cashflow.

        Args:
            spec: Fixed coupon specification

        Returns:
            CashflowBuilder: Self for method chaining
        """
        ...

    def floating_cf(self, spec: FloatingCouponSpec) -> CashflowBuilder:
        """Add floating coupon cashflow.

        Args:
            spec: Floating coupon specification

        Returns:
            CashflowBuilder: Self for method chaining
        """
        ...

    def fixed_stepup(
        self, steps: List[Tuple[date | str, float]], schedule: ScheduleParams, default_split: CouponType
    ) -> CashflowBuilder:
        """Fixed step-up program with boundaries steps=[(end_date, rate), ...].

        Args:
            steps: List of (end_date, rate) tuples
            schedule: Schedule parameters
            default_split: Default coupon type for splits

        Returns:
            CashflowBuilder: Self for method chaining
        """
        ...

    def payment_split_program(self, steps: List[Tuple[date | str, CouponType]]) -> CashflowBuilder:
        """Payment split program (end_date, split) where split is CouponType.

        Args:
            steps: List of (end_date, split) tuples

        Returns:
            CashflowBuilder: Self for method chaining
        """
        ...

    def build_with_curves(self, market: Optional[MarketContext] = None) -> CashFlowSchedule:
        """Build the cashflow schedule with market curves for floating rate computation.

        When a market context is provided, floating rate coupons include the forward rate
        from the curve: coupon = outstanding * (forward_rate * gearing + margin_bp * 1e-4) * year_fraction

        Without curves (or using build_with_curves(None)), only the margin is used:
        coupon = outstanding * (margin_bp * 1e-4 * gearing) * year_fraction

        Args:
            market: Optional market context with curves

        Returns:
            CashFlowSchedule: Built cashflow schedule with forward rates
        """
        ...

class CashFlowSchedule:
    """CashflowSchedule wrapper exposing holder-side flows and metadata."""

    @property
    def day_count(self) -> DayCount:
        """Day count convention used for the schedule."""
        ...

    @property
    def notional(self) -> Money:
        """Initial notional amount."""
        ...

    def flows(self) -> List[CashFlow]:
        """List of cashflows in the schedule."""
        ...

    def to_dataframe(
        self,
        market: Optional[MarketContext] = None,
        discount_curve_id: Optional[str] = None,
        as_of: Optional[date | str] = None,
    ) -> Any:
        """Convert the schedule into a Polars DataFrame.

        Returns a Polars DataFrame with columns: "start_date", "end_date", "kind", "amount",
        "accrual_factor", "reset_date", "outstanding", "rate", and optionally
        "outstanding_undrawn" (if facility limit exists), "discount_factor", "pv" (if market provided).
        """
        ...

class FeeBase:
    """Fee base for periodic basis point fees.

    Determines what balance is used to calculate periodic fees.

    Examples:
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.cashflow.builder import FeeBase
        >>> # Fee on drawn balance
        >>> FeeBase.drawn()
        >>> # Fee on undrawn (unused) facility
        >>> FeeBase.undrawn(Money(10_000_000, Currency("USD")))
    """

    @classmethod
    def drawn(cls) -> FeeBase:
        """Fee calculated on drawn (outstanding) balance.

        Returns:
            FeeBase: Drawn balance base
        """
        ...

    @classmethod
    def undrawn(cls, facility_limit: Money) -> FeeBase:
        """Fee calculated on undrawn (unused) facility.

        Args:
            facility_limit: Total facility size as Money

        Returns:
            FeeBase: Undrawn balance base (facility_limit - outstanding)
        """
        ...

    def __repr__(self) -> str: ...

class FeeSpec:
    """Fee specification for cashflow schedules.

    Supports both fixed one-time fees and periodic fees calculated as
    basis points on drawn or undrawn balances.

    Examples:
        >>> from datetime import date
        >>> from finstack.core.currency import Currency
        >>> from finstack.core.money import Money
        >>> from finstack.valuations.cashflow.builder import FeeBase, FeeSpec, ScheduleParams
        >>> # One-time fixed fee
        >>> FeeSpec.fixed(date(2025, 6, 15), Money(50_000, Currency("USD")))
        >>> # Periodic commitment fee on undrawn balance
        >>> FeeSpec.periodic_bps(
        ...     FeeBase.undrawn(Money(10_000_000, Currency("USD"))),
        ...     25.0,
        ...     ScheduleParams.quarterly_act360(),
        ... )
    """

    @classmethod
    def fixed(cls, date: date, amount: Money) -> FeeSpec:
        """Create a fixed one-time fee.

        Args:
            date: Payment date
            amount: Fee amount as Money

        Returns:
            FeeSpec: Fixed fee specification
        """
        ...

    @classmethod
    def periodic_bps(
        cls, base: FeeBase, bps: float, schedule: ScheduleParams, *, calendar: Optional[str] = None, stub: str = "none"
    ) -> FeeSpec:
        """Create a periodic fee calculated as basis points on a balance.

        Args:
            base: Fee base (drawn or undrawn balance)
            bps: Fee rate in basis points (e.g., 25.0 for 0.25%)
            schedule: Schedule parameters (frequency, day count, BDC)
            calendar: Optional calendar identifier
            stub: Optional stub kind (default: "none")

        Returns:
            FeeSpec: Periodic fee specification
        """
        ...

    def __repr__(self) -> str: ...

class FixedWindow:
    """Fixed coupon window for rate step-up programs.

    Defines a period with a specific fixed rate and schedule.

    Examples:
        >>> from finstack.valuations.cashflow.builder import FixedWindow, ScheduleParams
        >>> window = FixedWindow(rate=0.05, schedule=ScheduleParams.quarterly_act360())
    """

    def __init__(self, rate: float, schedule: ScheduleParams) -> None:
        """Create a fixed coupon window.

        Args:
            rate: Fixed coupon rate (annual decimal)
            schedule: Schedule parameters defining frequency and conventions

        Returns:
            FixedWindow: Window specification
        """
        ...

    @property
    def rate(self) -> float:
        """Fixed coupon rate."""
        ...

    def __repr__(self) -> str: ...

class FloatWindow:
    """Floating coupon window for floating rate periods.

    Defines a period with floating rate parameters and schedule.

    Examples:
        >>> from finstack.valuations.cashflow.builder import FloatCouponParams, FloatWindow, ScheduleParams
        >>> params = FloatCouponParams.new("USD-SOFR", 50.0, 1.0, 2)
        >>> window = FloatWindow(params=params, schedule=ScheduleParams.quarterly_act360())
    """

    def __init__(self, params: FloatCouponParams, schedule: ScheduleParams) -> None:
        """Create a floating coupon window.

        Args:
            params: Floating rate parameters (index, margin, gearing)
            schedule: Schedule parameters defining frequency and conventions

        Returns:
            FloatWindow: Window specification
        """
        ...

    def __repr__(self) -> str: ...
