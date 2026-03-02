"""Valuations cash-flow builder exposing complex coupon windows, PIK splits, and amortization."""

from __future__ import annotations
from typing import Iterator, List, Any, Dict, Tuple
from datetime import date as _Date
from ...core.currency import Currency
from ...core.money import Money
from ...core.dates.schedule import Frequency, StubKind
from ...core.dates.daycount import DayCount
from ...core.dates.calendar import BusinessDayConvention
from ...core.cashflow.primitives import CashFlow, CFKind
from ...core.market_data.context import MarketContext
from ...core.market_data.term_structures import DiscountCurve
from ...core.dates.periods import Period, PeriodPlan

# ---------------------------------------------------------------------------
# Amortization
# ---------------------------------------------------------------------------

class AmortizationSpec:
    """Amortization specification for principal payments."""

    @classmethod
    def none(cls) -> AmortizationSpec:
        """No amortization: principal remains until redemption."""
        ...

    @classmethod
    def linear_to(cls, final_notional: Money) -> AmortizationSpec:
        """Linear amortization to final notional."""
        ...

    @classmethod
    def step_remaining(
        cls,
        schedule: List[Tuple[_Date | str, Money]],
    ) -> AmortizationSpec:
        """Step amortization with remaining notional."""
        ...

    @classmethod
    def percent_per_period(cls, pct: float) -> AmortizationSpec:
        """Percentage amortization per period (e.g., 0.05 = 5%)."""
        ...

    @classmethod
    def custom_principal(
        cls,
        items: List[Tuple[_Date | str, Money]],
    ) -> AmortizationSpec:
        """Custom principal amortization."""
        ...

    def __repr__(self) -> str: ...

# ---------------------------------------------------------------------------
# Notional
# ---------------------------------------------------------------------------

class Notional:
    """Principal notional with optional amortization schedule."""

    @classmethod
    def par(cls, amount: float, currency: Currency) -> Notional:
        """Create a par notional (no amortization)."""
        ...

    @property
    def initial(self) -> Money: ...
    @property
    def amort(self) -> AmortizationSpec: ...
    def validate(self) -> None: ...
    def __repr__(self) -> str: ...

# ---------------------------------------------------------------------------
# Coupon types
# ---------------------------------------------------------------------------

class CouponType:
    """Coupon split type (cash, PIK, split)."""

    CASH: CouponType
    PIK: CouponType

    @classmethod
    def split(cls, cash_pct: float, pik_pct: float) -> CouponType:
        """Create a split coupon type with percentage weights summing to ~1.0."""
        ...

# ---------------------------------------------------------------------------
# Overnight compounding
# ---------------------------------------------------------------------------

class OvernightCompoundingMethod:
    """Overnight rate compounding method for SOFR/SONIA-style indices."""

    SIMPLE_AVERAGE: OvernightCompoundingMethod
    COMPOUNDED_IN_ARREARS: OvernightCompoundingMethod

    @classmethod
    def compounded_with_lookback(cls, lookback_days: int) -> OvernightCompoundingMethod:
        """Compounded in arrears with lookback."""
        ...

    @classmethod
    def compounded_with_lockout(cls, lockout_days: int) -> OvernightCompoundingMethod:
        """Compounded in arrears with lockout."""
        ...

    @classmethod
    def compounded_with_observation_shift(cls, shift_days: int) -> OvernightCompoundingMethod:
        """Compounded with observation shift."""
        ...

    def __repr__(self) -> str: ...

# ---------------------------------------------------------------------------
# Schedule parameters
# ---------------------------------------------------------------------------

class ScheduleParams:
    """Schedule parameter bundle."""

    @classmethod
    def new(
        cls,
        freq: Frequency,
        day_count: DayCount,
        bdc: BusinessDayConvention,
        calendar_id: str,
        stub: StubKind | None = None,
        end_of_month: bool = False,
        payment_lag_days: int = 0,
    ) -> ScheduleParams:
        """Create schedule parameters.

        Args:
            freq: Payment frequency
            day_count: Day count convention
            bdc: Business day convention
            calendar_id: Calendar identifier
            stub: Optional stub kind
            end_of_month: End-of-month rule (default: False)
            payment_lag_days: Payment lag in days (default: 0)
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
        """USD market standard: quarterly, Act/360, Modified Following, USD calendar."""
        ...

    @classmethod
    def eur_standard(cls) -> ScheduleParams:
        """EUR market standard: semi-annual, 30/360, Modified Following, EUR calendar."""
        ...

    @classmethod
    def gbp_standard(cls) -> ScheduleParams:
        """GBP market standard: semi-annual, Act/365, Modified Following, GBP calendar."""
        ...

    @classmethod
    def jpy_standard(cls) -> ScheduleParams:
        """JPY market standard: semi-annual, Act/365, Modified Following, JPY calendar."""
        ...

# ---------------------------------------------------------------------------
# Coupon specs
# ---------------------------------------------------------------------------

class FixedCouponSpec:
    """Fixed coupon specification."""

    @classmethod
    def new(
        cls,
        rate: float,
        schedule: ScheduleParams,
        coupon_type: CouponType | None = None,
    ) -> FixedCouponSpec:
        """Create fixed coupon specification."""
        ...

class FloatCouponParams:
    """Floating coupon parameters."""

    @classmethod
    def new(
        cls,
        index_id: str,
        margin_bp: float,
        *,
        gearing: float = 1.0,
        reset_lag_days: int = 2,
    ) -> FloatCouponParams:
        """Create floating coupon parameters."""
        ...

class FloatingRateSpec:
    """Full floating rate specification with caps, floors, and compounding."""

    @classmethod
    def new(
        cls,
        index_id: str,
        spread_bp: float,
        schedule: ScheduleParams,
        *,
        gearing: float = 1.0,
        gearing_includes_spread: bool = True,
        floor_bp: float | None = None,
        all_in_floor_bp: float | None = None,
        cap_bp: float | None = None,
        index_cap_bp: float | None = None,
        reset_lag_days: int = 2,
        fixing_calendar_id: str | None = None,
        overnight_compounding: OvernightCompoundingMethod | None = None,
    ) -> FloatingRateSpec:
        """Create full floating rate specification."""
        ...

    @property
    def index_id(self) -> str: ...
    @property
    def spread_bp(self) -> float: ...
    @property
    def gearing(self) -> float: ...
    @property
    def floor_bp(self) -> float | None: ...
    @property
    def cap_bp(self) -> float | None: ...
    def __repr__(self) -> str: ...

class FloatingCouponSpec:
    """Floating coupon specification."""

    @classmethod
    def new(
        cls,
        params: FloatCouponParams,
        schedule: ScheduleParams,
        coupon_type: CouponType | None = None,
    ) -> FloatingCouponSpec:
        """Create from simplified FloatCouponParams (no caps/floors/compounding)."""
        ...

    @classmethod
    def from_rate_spec(
        cls,
        rate_spec: FloatingRateSpec,
        schedule: ScheduleParams,
        coupon_type: CouponType | None = None,
    ) -> FloatingCouponSpec:
        """Create from full FloatingRateSpec (with caps, floors, compounding)."""
        ...

# ---------------------------------------------------------------------------
# Fees
# ---------------------------------------------------------------------------

class FeeBase:
    """Fee base for periodic basis point fees."""

    @classmethod
    def drawn(cls) -> FeeBase:
        """Fee calculated on drawn (outstanding) balance."""
        ...

    @classmethod
    def undrawn(cls, facility_limit: Money) -> FeeBase:
        """Fee calculated on undrawn (unused) facility."""
        ...

    def __repr__(self) -> str: ...

class FeeSpec:
    """Fee specification for cashflow schedules."""

    @classmethod
    def fixed(cls, date: _Date, amount: Money) -> FeeSpec:
        """Create a fixed one-time fee."""
        ...

    @classmethod
    def periodic_bps(
        cls,
        base: FeeBase,
        bps: float,
        schedule: ScheduleParams,
        *,
        calendar: str | None = None,
        stub: str | None = None,
    ) -> FeeSpec:
        """Create a periodic fee calculated as basis points on a balance."""
        ...

    def __repr__(self) -> str: ...

class FeeTier:
    """Fee tier defining a utilization threshold and corresponding basis point fee."""

    @classmethod
    def from_bps(cls, threshold: float, bps: float) -> FeeTier:
        """Create a fee tier from utilization threshold (0.0-1.0) and fee in bps."""
        ...

    @property
    def threshold(self) -> float: ...
    @property
    def bps(self) -> float: ...
    def __repr__(self) -> str: ...

def evaluate_fee_tiers(tiers: List[FeeTier], utilization: float) -> float:
    """Evaluate tiered fees given utilization level."""
    ...

# ---------------------------------------------------------------------------
# Windows
# ---------------------------------------------------------------------------

class FixedWindow:
    """Fixed coupon window for rate step-up programs."""

    def __init__(self, rate: float, schedule: ScheduleParams) -> None: ...
    @property
    def rate(self) -> float: ...
    def __repr__(self) -> str: ...

class FloatWindow:
    """Floating coupon window for floating rate periods."""

    def __init__(self, params: FloatCouponParams, schedule: ScheduleParams) -> None: ...
    def __repr__(self) -> str: ...

# ---------------------------------------------------------------------------
# Principal events
# ---------------------------------------------------------------------------

class PrincipalEvent:
    """A principal draw/repay event that adjusts outstanding balance."""

    @classmethod
    def new(
        cls,
        date: _Date,
        delta: Money,
        cash: Money,
        kind: CFKind,
    ) -> PrincipalEvent:
        """Create a principal event."""
        ...

    @property
    def date(self) -> _Date: ...
    @property
    def delta(self) -> Money: ...
    @property
    def cash(self) -> Money: ...
    @property
    def kind(self) -> CFKind: ...
    def __repr__(self) -> str: ...

# ---------------------------------------------------------------------------
# Prepayment / Default / Recovery
# ---------------------------------------------------------------------------

class PrepaymentCurve:
    """Prepayment curve shape."""

    CONSTANT: PrepaymentCurve

    @classmethod
    def psa(cls, speed_multiplier: float) -> PrepaymentCurve:
        """PSA prepayment curve."""
        ...

    @classmethod
    def cmbs_lockout(cls, lockout_months: int) -> PrepaymentCurve:
        """CMBS lockout curve."""
        ...

    def __repr__(self) -> str: ...

class PrepaymentModelSpec:
    """Prepayment model specification with CPR and optional curve."""

    @classmethod
    def constant_cpr(cls, cpr: float) -> PrepaymentModelSpec:
        """Constant CPR prepayment model."""
        ...

    @classmethod
    def psa(cls, speed_multiplier: float) -> PrepaymentModelSpec:
        """PSA prepayment model."""
        ...

    @classmethod
    def psa_100(cls) -> PrepaymentModelSpec:
        """PSA 100% standard prepayment model."""
        ...

    @classmethod
    def cmbs_with_lockout(cls, lockout_months: int, post_lockout_cpr: float) -> PrepaymentModelSpec:
        """CMBS with lockout: no prepayment during lockout, constant CPR after."""
        ...

    def smm(self, seasoning_months: int) -> float:
        """Compute the Single Monthly Mortality (SMM) rate for a given month."""
        ...

    @property
    def cpr(self) -> float: ...
    def __repr__(self) -> str: ...

class DefaultCurve:
    """Default curve shape."""

    CONSTANT: DefaultCurve

    @classmethod
    def sda(cls, speed_multiplier: float) -> DefaultCurve:
        """SDA default curve."""
        ...

    def __repr__(self) -> str: ...

class DefaultModelSpec:
    """Default model specification with CDR and optional curve."""

    @classmethod
    def constant_cdr(cls, cdr: float) -> DefaultModelSpec:
        """Constant CDR default model."""
        ...

    @classmethod
    def sda(cls, speed_multiplier: float) -> DefaultModelSpec:
        """SDA default model."""
        ...

    @classmethod
    def cdr_2pct(cls) -> DefaultModelSpec:
        """Standard 2% CDR default model."""
        ...

    def mdr(self, seasoning_months: int) -> float:
        """Compute the Monthly Default Rate (MDR) for a given month."""
        ...

    @property
    def cdr(self) -> float: ...
    def __repr__(self) -> str: ...

class DefaultEvent:
    """A specific default event with date, amount, and recovery parameters."""

    @classmethod
    def new(
        cls,
        default_date: _Date,
        defaulted_amount: float,
        recovery_rate: float,
        recovery_lag: int,
        *,
        recovery_bdc: BusinessDayConvention | None = None,
        recovery_calendar_id: str | None = None,
    ) -> DefaultEvent: ...
    def validate(self) -> None: ...
    @property
    def default_date(self) -> _Date: ...
    @property
    def defaulted_amount(self) -> float: ...
    @property
    def recovery_rate(self) -> float: ...
    @property
    def recovery_lag(self) -> int: ...
    def __repr__(self) -> str: ...

class RecoveryModelSpec:
    """Recovery model specification with rate and lag."""

    @classmethod
    def with_lag(cls, rate: float, recovery_lag: int) -> RecoveryModelSpec:
        """Create a recovery model with rate (0.0-1.0) and lag in months."""
        ...

    def validate(self) -> None: ...
    @property
    def rate(self) -> float: ...
    @property
    def recovery_lag(self) -> int: ...
    def __repr__(self) -> str: ...

# ---------------------------------------------------------------------------
# Builder
# ---------------------------------------------------------------------------

class CashFlowBuilder:
    """Composable cashflow schedule builder."""

    @classmethod
    def new(cls) -> CashFlowBuilder:
        """Create a new cashflow builder."""
        ...

    def principal(self, amount: float, currency: Currency, issue: _Date, maturity: _Date) -> CashFlowBuilder:
        """Add principal cashflow."""
        ...

    def amortization(self, amortization: AmortizationSpec | None) -> CashFlowBuilder:
        """Add amortization specification."""
        ...

    def fixed_cf(self, spec: FixedCouponSpec) -> CashFlowBuilder:
        """Add fixed coupon cashflow."""
        ...

    def floating_cf(self, spec: FloatingCouponSpec) -> CashFlowBuilder:
        """Add floating coupon cashflow."""
        ...

    def fee(self, spec: FeeSpec) -> CashFlowBuilder:
        """Add a fee specification to the schedule."""
        ...

    def principal_events(self, events: List[PrincipalEvent]) -> CashFlowBuilder:
        """Add custom principal events (draws/repays)."""
        ...

    def add_principal_event(self, date: _Date, delta: Money, cash: Money, kind: CFKind) -> CashFlowBuilder:
        """Add a single principal event."""
        ...

    def add_fixed_coupon_window(
        self,
        start: _Date,
        end: _Date,
        rate: float,
        schedule: ScheduleParams,
        split: CouponType,
    ) -> CashFlowBuilder:
        """Add a fixed coupon window with explicit start/end dates."""
        ...

    def add_float_coupon_window(
        self,
        start: _Date,
        end: _Date,
        params: FloatCouponParams,
        schedule: ScheduleParams,
        split: CouponType,
    ) -> CashFlowBuilder:
        """Add a floating coupon window with explicit start/end dates."""
        ...

    def add_payment_window(self, start: _Date, end: _Date, split: CouponType) -> CashFlowBuilder:
        """Add a payment window (PIK toggle) with explicit start/end dates."""
        ...

    def fixed_stepup(
        self,
        steps: List[Tuple[_Date | str, float]],
        schedule: ScheduleParams,
        default_split: CouponType,
    ) -> CashFlowBuilder:
        """Fixed step-up program with boundaries steps=[(end_date, rate), ...]."""
        ...

    def float_margin_stepup(
        self,
        steps: List[Tuple[_Date | str, float]],
        base_params: FloatCouponParams,
        schedule: ScheduleParams,
        default_split: CouponType,
    ) -> CashFlowBuilder:
        """Floating margin step-up program with boundaries steps=[(end_date, margin_bp), ...]."""
        ...

    def fixed_to_float(
        self,
        switch: _Date,
        fixed_win: FixedWindow,
        float_win: FloatWindow,
        default_split: CouponType,
    ) -> CashFlowBuilder:
        """Fixed-to-float switch at a given date."""
        ...

    def payment_split_program(self, steps: List[Tuple[_Date | str, CouponType]]) -> CashFlowBuilder:
        """Payment split program (end_date, split) where split is CouponType."""
        ...

    def build_with_curves(self, market: MarketContext | None = None) -> CashFlowSchedule:
        """Build the cashflow schedule with optional market curves for floating rate computation."""
        ...

# ---------------------------------------------------------------------------
# Schedule
# ---------------------------------------------------------------------------

class CashFlowSchedule:
    """Cashflow schedule with flows, metadata, and analytics."""

    @property
    def day_count(self) -> DayCount: ...
    @property
    def notional(self) -> Money: ...
    def flows(self) -> List[CashFlow]:
        """List of all cashflows in the schedule."""
        ...

    def dates(self) -> List[_Date]:
        """Unique payment dates from the schedule."""
        ...

    def coupons(self) -> List[CashFlow]:
        """Only coupon cashflows from the schedule."""
        ...

    def outstanding_path_per_flow(self) -> List[Tuple[_Date, Money]]:
        """Outstanding balance path per cashflow as (date, Money) pairs."""
        ...

    def outstanding_by_date(self) -> List[Tuple[_Date, Money]]:
        """Outstanding balance by date as (date, Money) pairs."""
        ...

    def npv(
        self,
        market_or_curve: MarketContext | DiscountCurve,
        *,
        discount_curve_id: str | None = None,
        as_of: _Date | str | None = None,
        day_count: DayCount | None = None,
    ) -> float:
        """Compute the net present value of all cashflows."""
        ...

    def per_period_pv(
        self,
        periods: List[Period] | PeriodPlan,
        market_or_curve: MarketContext | DiscountCurve,
        *,
        discount_curve_id: str | None = None,
        hazard_curve_id: str | None = None,
        as_of: _Date | str | None = None,
        day_count: DayCount | None = None,
    ) -> Dict[str, float]:
        """Compute present values aggregated by period."""
        ...

    def to_dataframe(
        self,
        *,
        market: MarketContext,
        discount_curve_id: str,
        as_of: _Date | str | None = None,
        credit_curve_id: str | None = None,
        forward_curve_id: str | None = None,
        include_floating_decomposition: bool = False,
        day_count: DayCount | None = None,
        discount_day_count: DayCount | None = None,
        facility_limit: Money | None = None,
    ) -> Any:
        """Convert the cashflow schedule to a Polars DataFrame."""
        ...

    def __len__(self) -> int: ...
    def __getitem__(self, index: int) -> CashFlow: ...
    def __iter__(self) -> Iterator[CashFlow]: ...

# ---------------------------------------------------------------------------
# Utility functions
# ---------------------------------------------------------------------------

def cpr_to_smm(cpr: float) -> float:
    """Convert annual CPR to Single Monthly Mortality (SMM)."""
    ...

def smm_to_cpr(smm: float) -> float:
    """Convert Single Monthly Mortality (SMM) to annual CPR."""
    ...

def compute_compounded_rate(
    daily_rates: List[Tuple[float, int]],
    total_days: int,
    day_count_basis: float,
) -> float:
    """Compute a compounded rate from daily rate observations."""
    ...

def compute_simple_average_rate(
    daily_rates: List[Tuple[float, int]],
    total_days: int,
) -> float:
    """Compute a simple average rate from daily rate observations."""
    ...

def compute_overnight_rate(
    method: OvernightCompoundingMethod,
    daily_rates: List[Tuple[float, int]],
    total_days: int,
    day_count_basis: float,
) -> float:
    """Compute an overnight compounding rate using the specified method."""
    ...
