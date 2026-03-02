"""Term loan instrument with typed builder and supporting types."""

from __future__ import annotations
import datetime
from typing import Sequence
from ....core.money import Money
from ....core.currency import Currency
from ...common import InstrumentType

class RateSpec:
    """Rate specification for term loans (fixed or floating).

    Examples
    --------
        >>> fixed = RateSpec.fixed(600)  # 6% fixed
        >>> floating = RateSpec.floating("USD-SOFR-3M", 300)
    """

    @classmethod
    def fixed(cls, rate_bp: int) -> RateSpec:
        """Create a fixed-rate specification.

        Parameters
        ----------
        rate_bp : int
            Annual rate in basis points (e.g. 600 = 6%).
        """
        ...

    @classmethod
    def floating(
        cls,
        index_id: str,
        spread_bp: float,
        reset_freq: str = "3M",
        reset_lag_days: int = 2,
        floor_bp: float | None = None,
        cap_bp: float | None = None,
    ) -> RateSpec:
        """Create a floating-rate specification.

        Parameters
        ----------
        index_id : str
            Forward curve / index identifier (e.g. ``"USD-SOFR-3M"``).
        spread_bp : float
            Spread over the index in basis points.
        reset_freq : str
            Reset frequency tenor (default ``"3M"``).
        reset_lag_days : int
            Observation lag in business days (default 2).
        floor_bp : float, optional
            Index floor in basis points.
        cap_bp : float, optional
            Index cap in basis points.
        """
        ...

    def __repr__(self) -> str: ...

class TermLoanAmortizationSpec:
    """Amortization schedule specification for term loans.

    Examples
    --------
        >>> bullet = TermLoanAmortizationSpec.none()
        >>> amort = TermLoanAmortizationSpec.percent_per_period(250)  # 2.5%/period
    """

    @classmethod
    def none(cls) -> TermLoanAmortizationSpec:
        """Bullet loan with no scheduled amortization."""
        ...

    @classmethod
    def linear(cls, start: datetime.date, end: datetime.date) -> TermLoanAmortizationSpec:
        """Linear amortization between *start* and *end*.

        Parameters
        ----------
        start : date
            Amortization start date.
        end : date
            Amortization end date (full repayment).
        """
        ...

    @classmethod
    def percent_per_period(cls, bp: int) -> TermLoanAmortizationSpec:
        """Percentage of current outstanding per period (geometric decay).

        Parameters
        ----------
        bp : int
            Basis points per period applied to current outstanding.
        """
        ...

    @classmethod
    def custom(cls, schedule: Sequence[tuple[datetime.date, Money]]) -> TermLoanAmortizationSpec:
        """Custom amortization schedule.

        Parameters
        ----------
        schedule : Sequence[tuple[date, Money]]
            List of ``(date, principal_payment)`` tuples.
        """
        ...

    def __repr__(self) -> str: ...

class CouponType:
    """Coupon type for term loans (Cash, PIK, or Split).

    Examples
    --------
        >>> ct = CouponType.CASH
        >>> ct = CouponType.split(0.7, 0.3)  # 70% cash / 30% PIK
    """

    CASH: CouponType
    PIK: CouponType

    @classmethod
    def split(cls, cash_pct: float, pik_pct: float) -> CouponType:
        """Split coupon between cash and PIK.

        Parameters
        ----------
        cash_pct : float
            Fraction paid in cash (e.g. 0.7).
        pik_pct : float
            Fraction capitalized as PIK (e.g. 0.3).
        """
        ...

    def __repr__(self) -> str: ...

class DrawEvent:
    """Draw event for delayed-draw term loans."""

    def __init__(self, date: datetime.date, amount: Money) -> None:
        """Create a draw event.

        Parameters
        ----------
        date : date
            Date of the draw.
        amount : Money
            Amount drawn.
        """
        ...

    @property
    def date(self) -> datetime.date: ...
    @property
    def amount(self) -> Money: ...
    def __repr__(self) -> str: ...

class CommitmentStepDown:
    """Commitment step-down event for DDTL facilities."""

    def __init__(self, date: datetime.date, new_limit: Money) -> None:
        """Create a step-down event.

        Parameters
        ----------
        date : date
            Effective date.
        new_limit : Money
            New (lower) commitment limit.
        """
        ...

    @property
    def date(self) -> datetime.date: ...
    @property
    def new_limit(self) -> Money: ...
    def __repr__(self) -> str: ...

class CommitmentFeeBase:
    """Basis for calculating commitment fees on undrawn portions."""

    UNDRAWN: CommitmentFeeBase
    COMMITMENT_MINUS_OUTSTANDING: CommitmentFeeBase

    def __repr__(self) -> str: ...

class MarginStepUp:
    """Margin step-up event (covenant penalty or scheduled increase)."""

    def __init__(self, date: datetime.date, delta_bp: int) -> None:
        """Create a margin step-up.

        Parameters
        ----------
        date : date
            Effective date.
        delta_bp : int
            Increase in margin (basis points).
        """
        ...

    @property
    def date(self) -> datetime.date: ...
    @property
    def delta_bp(self) -> int: ...
    def __repr__(self) -> str: ...

class PikToggle:
    """Payment-in-kind (PIK) toggle event."""

    def __init__(self, date: datetime.date, enable_pik: bool) -> None:
        """Create a PIK toggle.

        Parameters
        ----------
        date : date
            Date PIK feature is toggled.
        enable_pik : bool
            True to enable PIK, False to disable.
        """
        ...

    @property
    def date(self) -> datetime.date: ...
    @property
    def enable_pik(self) -> bool: ...
    def __repr__(self) -> str: ...

class CashSweepEvent:
    """Cash sweep event (mandatory prepayment)."""

    def __init__(self, date: datetime.date, amount: Money) -> None:
        """Create a cash sweep event.

        Parameters
        ----------
        date : date
            Date of the cash sweep.
        amount : Money
            Prepayment amount.
        """
        ...

    @property
    def date(self) -> datetime.date: ...
    @property
    def amount(self) -> Money: ...
    def __repr__(self) -> str: ...

class CovenantSpec:
    """Covenant-driven events for term loans."""

    def __init__(
        self,
        margin_stepups: Sequence[MarginStepUp] | None = None,
        pik_toggles: Sequence[PikToggle] | None = None,
        cash_sweeps: Sequence[CashSweepEvent] | None = None,
        draw_stop_dates: Sequence[datetime.date] | None = None,
    ) -> None:
        """Create a covenant specification.

        Parameters
        ----------
        margin_stepups : Sequence[MarginStepUp], optional
            Margin step-up events.
        pik_toggles : Sequence[PikToggle], optional
            PIK toggle events.
        cash_sweeps : Sequence[CashSweepEvent], optional
            Cash sweep events.
        draw_stop_dates : Sequence[date], optional
            Dates on which draws are prohibited.
        """
        ...

    def __repr__(self) -> str: ...

class DdtlSpec:
    """Delayed-draw term loan (DDTL) specification."""

    def __init__(
        self,
        commitment_limit: Money,
        availability_start: datetime.date,
        availability_end: datetime.date,
        draws: Sequence[DrawEvent] | None = None,
        commitment_step_downs: Sequence[CommitmentStepDown] | None = None,
        usage_fee_bp: int = 0,
        commitment_fee_bp: int = 0,
        fee_base: CommitmentFeeBase | None = None,
        oid_policy: OidPolicy | None = None,
    ) -> None:
        """Create a DDTL specification.

        Parameters
        ----------
        commitment_limit : Money
            Total commitment available for draws.
        availability_start : date
            First date draws are permitted.
        availability_end : date
            Last date draws are permitted.
        draws : Sequence[DrawEvent], optional
            Scheduled draw events.
        commitment_step_downs : Sequence[CommitmentStepDown], optional
            Step-down schedule.
        usage_fee_bp : int
            Usage fee in basis points on drawn amounts.
        commitment_fee_bp : int
            Commitment fee in basis points on undrawn amounts.
        fee_base : CommitmentFeeBase, optional
            Fee calculation basis (default ``UNDRAWN``).
        oid_policy : OidPolicy, optional
            Original issue discount policy.
        """
        ...

    @property
    def commitment_limit(self) -> Money: ...
    @property
    def usage_fee_bp(self) -> int: ...
    @property
    def commitment_fee_bp(self) -> int: ...
    def __repr__(self) -> str: ...

class OidEirSpec:
    """OID effective interest rate amortization settings."""

    def __init__(self, include_fees: bool = True) -> None:
        """Create an OID EIR spec.

        Parameters
        ----------
        include_fees : bool
            Include fee cashflows in the EIR schedule (default True).
        """
        ...

    @property
    def include_fees(self) -> bool: ...
    def __repr__(self) -> str: ...

class OidPolicy:
    """Original Issue Discount (OID) policy.

    Examples
    --------
        >>> oid = OidPolicy.withheld_pct(200)  # 2% OID withheld
    """

    @classmethod
    def withheld_pct(cls, pct_bp: int) -> OidPolicy:
        """Discount as percentage withheld from funded proceeds.

        Parameters
        ----------
        pct_bp : int
            Discount in basis points.
        """
        ...

    @classmethod
    def withheld_amount(cls, amount: Money) -> OidPolicy:
        """Fixed discount amount withheld from funded proceeds.

        Parameters
        ----------
        amount : Money
            Discount amount.
        """
        ...

    @classmethod
    def separate_pct(cls, pct_bp: int) -> OidPolicy:
        """Discount as percentage tracked separately.

        Parameters
        ----------
        pct_bp : int
            Discount in basis points.
        """
        ...

    @classmethod
    def separate_amount(cls, amount: Money) -> OidPolicy:
        """Fixed discount amount tracked separately.

        Parameters
        ----------
        amount : Money
            Discount amount.
        """
        ...

    def __repr__(self) -> str: ...

class LoanCallType:
    """Type of borrower call provision on a term loan.

    Examples
    --------
        >>> hard = LoanCallType.HARD
        >>> mw = LoanCallType.make_whole(50)  # T+50 bps
    """

    HARD: LoanCallType
    SOFT: LoanCallType

    @classmethod
    def make_whole(cls, treasury_spread_bp: int) -> LoanCallType:
        """Make-whole call at Treasury + spread.

        Parameters
        ----------
        treasury_spread_bp : int
            Spread over the reference rate in basis points.
        """
        ...

    def __repr__(self) -> str: ...

class LoanCall:
    """Borrower call option on a term loan."""

    def __init__(
        self,
        date: datetime.date,
        price_pct_of_par: float,
        call_type: LoanCallType | None = None,
    ) -> None:
        """Create a call option.

        Parameters
        ----------
        date : date
            Earliest prepayment date for this provision.
        price_pct_of_par : float
            Redemption price as percentage of par (e.g. 102.0).
        call_type : LoanCallType, optional
            Type of call (default ``HARD``).
        """
        ...

    @property
    def date(self) -> datetime.date: ...
    @property
    def price_pct_of_par(self) -> float: ...
    def __repr__(self) -> str: ...

class LoanCallSchedule:
    """Complete call schedule for callable term loans."""

    def __init__(self, calls: Sequence[LoanCall]) -> None:
        """Create a call schedule.

        Parameters
        ----------
        calls : Sequence[LoanCall]
            Ordered call provisions.
        """
        ...

    @property
    def call_count(self) -> int: ...
    def __repr__(self) -> str: ...

class TermLoanBuilder:
    """Fluent builder for :class:`TermLoan` instruments.

    Examples
    --------
        >>> loan = (
        ...     TermLoan
        ...     .builder("TL-001")
        ...     .currency("USD")
        ...     .notional(Money(10_000_000, "USD"))
        ...     .issue(date(2024, 1, 1))
        ...     .maturity(date(2029, 1, 1))
        ...     .rate(RateSpec.fixed(600))
        ...     .disc_id("USD-OIS")
        ...     .build()
        ... )
    """

    def __init__(self, instrument_id: str) -> None: ...
    def currency(self, currency: str | Currency) -> TermLoanBuilder: ...
    def notional(self, amount: Money) -> TermLoanBuilder: ...
    def issue(self, date: datetime.date) -> TermLoanBuilder: ...
    def maturity(self, date: datetime.date) -> TermLoanBuilder: ...
    def rate(self, spec: RateSpec) -> TermLoanBuilder: ...
    def frequency(self, freq: str) -> TermLoanBuilder: ...
    def day_count(self, dc: str) -> TermLoanBuilder: ...
    def bdc(self, bdc: str) -> TermLoanBuilder: ...
    def calendar(self, calendar_id: str | None = None) -> TermLoanBuilder: ...
    def stub(self, stub: str) -> TermLoanBuilder: ...
    def disc_id(self, curve_id: str) -> TermLoanBuilder: ...
    def credit_curve(self, curve_id: str | None = None) -> TermLoanBuilder: ...
    def amortization(self, spec: TermLoanAmortizationSpec) -> TermLoanBuilder: ...
    def coupon_type(self, ct: CouponType) -> TermLoanBuilder: ...
    def upfront_fee(self, fee: Money | None = None) -> TermLoanBuilder: ...
    def ddtl(self, spec: DdtlSpec | None = None) -> TermLoanBuilder: ...
    def covenants(self, spec: CovenantSpec | None = None) -> TermLoanBuilder: ...
    def oid_eir(self, spec: OidEirSpec | None = None) -> TermLoanBuilder: ...
    def call_schedule(self, schedule: LoanCallSchedule | None = None) -> TermLoanBuilder: ...
    def settlement_days(self, days: int) -> TermLoanBuilder: ...
    def build(self) -> TermLoan: ...
    def __repr__(self) -> str: ...

class TermLoan:
    """Term loan instrument with DDTL (Delayed Draw Term Loan) support.

    TermLoan represents a corporate loan with a fixed maturity and optional
    delayed draw features. Term loans are used for corporate financing and
    require discount curves and optionally credit curves for pricing.

    Term loans can include features like delayed drawdowns, amortization
    schedules, and prepayment options. They can be created via the typed
    builder or from a JSON specification.

    Examples
    --------
    Create via typed builder:

        >>> from finstack.valuations.instruments import TermLoan, RateSpec
        >>> from finstack.core.money import Money
        >>> from datetime import date
        >>>
        >>> loan = (
        ...     TermLoan
        ...     .builder("TL-001")
        ...     .currency("USD")
        ...     .notional(Money(10_000_000, "USD"))
        ...     .issue(date(2024, 1, 1))
        ...     .maturity(date(2029, 1, 1))
        ...     .rate(RateSpec.fixed(600))
        ...     .disc_id("USD-OIS")
        ...     .build()
        ... )

    Create from JSON:

        >>> term_loan = TermLoan.from_json(json_str)

    Notes
    -----
    - Term loans require discount curve and optionally credit curve
    - Can include delayed draw term loan (DDTL) features
    - Amortization schedules can be specified
    - Prepayment options affect cashflow timing

    See Also
    --------
    :class:`RevolvingCredit`: Revolving credit facilities
    :class:`Bond`: Bonds
    :class:`PricerRegistry`: Pricing entry point
    """

    @classmethod
    def builder(cls, instrument_id: str) -> TermLoanBuilder:
        """Start a fluent builder (``TermLoan.builder("ID")``).

        Parameters
        ----------
        instrument_id : str
            Unique identifier for the term loan.

        Returns
        -------
        TermLoanBuilder
            Builder instance for method chaining.
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> TermLoan:
        """Create a term loan from a JSON string specification.

        Parameters
        ----------
        json_str : str
            JSON string containing term loan specification.

        Returns
        -------
        TermLoan
            Configured term loan ready for pricing.

        Raises
        ------
        ValueError
            If JSON is invalid or required fields are missing.
        """
        ...

    def to_json(self) -> str:
        """Serialize the term loan to a JSON string."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def currency(self) -> Currency: ...
    @property
    def notional_limit(self) -> Money: ...
    @property
    def issue(self) -> datetime.date: ...
    @property
    def maturity(self) -> datetime.date: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
