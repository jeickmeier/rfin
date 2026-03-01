"""Revolving credit facility instrument."""

from __future__ import annotations
import datetime
from typing import List
from ....core.money import Money
from ....core.currency import Currency
from ...common import InstrumentType
from ...cashflow.builder import CashFlowSchedule

class FeeTier:
    """A single fee tier (utilization threshold -> basis points)."""

    def __init__(self, threshold: float, bps: float) -> None: ...
    @property
    def threshold(self) -> float: ...
    @property
    def bps(self) -> float: ...
    def __repr__(self) -> str: ...

class BaseRateSpec:
    """Base rate specification (fixed or floating)."""

    @classmethod
    def fixed(cls, rate: float) -> BaseRateSpec:
        """Create a fixed-rate spec."""
        ...

    @classmethod
    def floating(
        cls,
        index_id: str,
        spread_bp: float,
        reset_freq: str = "3M",
        day_count: str = "ACT360",
        calendar_id: str = "weekends_only",
    ) -> BaseRateSpec:
        """Create a floating-rate spec with simplified parameters."""
        ...

    @property
    def spec_type(self) -> str: ...
    @property
    def rate(self) -> float | None: ...
    def __repr__(self) -> str: ...

class RevolvingCreditFees:
    """Fee structure for a revolving credit facility."""

    def __init__(
        self,
        facility_fee_bp: float,
        commitment_fee_tiers: List[FeeTier] | None = None,
        usage_fee_tiers: List[FeeTier] | None = None,
        upfront_fee: Money | None = None,
    ) -> None: ...
    @classmethod
    def flat(
        cls,
        commitment_fee_bp: float,
        usage_fee_bp: float,
        facility_fee_bp: float,
    ) -> RevolvingCreditFees:
        """Create flat (non-tiered) fees."""
        ...

    @property
    def facility_fee_bp(self) -> float: ...
    @property
    def commitment_fee_tiers(self) -> List[FeeTier]: ...
    @property
    def usage_fee_tiers(self) -> List[FeeTier]: ...
    @property
    def upfront_fee(self) -> Money | None: ...
    def __repr__(self) -> str: ...

class DrawRepayEvent:
    """A single draw or repayment event."""

    def __init__(self, date: datetime.date, amount: float, currency: str | Currency, is_draw: bool) -> None: ...
    @property
    def date(self) -> datetime.date: ...
    @property
    def amount(self) -> Money: ...
    @property
    def is_draw(self) -> bool: ...
    def __repr__(self) -> str: ...

class UtilizationProcess:
    """Utilization process for stochastic simulation."""

    @classmethod
    def mean_reverting(cls, target_rate: float, speed: float, volatility: float) -> UtilizationProcess:
        """Create a mean-reverting Ornstein-Uhlenbeck utilization process."""
        ...

    @property
    def process_type(self) -> str: ...
    def __repr__(self) -> str: ...

class StochasticUtilizationSpec:
    """Stochastic utilization specification for Monte Carlo."""

    def __init__(
        self,
        process: UtilizationProcess,
        num_paths: int,
        seed: int | None = None,
        antithetic: bool = False,
        use_sobol_qmc: bool = False,
    ) -> None: ...
    @property
    def num_paths(self) -> int: ...
    @property
    def seed(self) -> int | None: ...
    @property
    def antithetic(self) -> bool: ...
    @property
    def use_sobol_qmc(self) -> bool: ...
    def __repr__(self) -> str: ...

class DrawRepaySpec:
    """Draw/repay specification (deterministic schedule or stochastic)."""

    @classmethod
    def deterministic(cls, events: List[DrawRepayEvent]) -> DrawRepaySpec:
        """Create a deterministic draw/repay specification from a list of events."""
        ...

    @classmethod
    def empty(cls) -> DrawRepaySpec:
        """Create a deterministic spec with an empty schedule."""
        ...

    @classmethod
    def stochastic(cls, spec: StochasticUtilizationSpec) -> DrawRepaySpec:
        """Create a stochastic draw/repay specification for Monte Carlo."""
        ...

    @property
    def spec_type(self) -> str: ...
    def __repr__(self) -> str: ...

class RevolvingCreditBuilder:
    """Fluent builder for constructing a RevolvingCredit instrument."""

    def __init__(self, instrument_id: str) -> None: ...
    def commitment_amount(self, amount: float) -> RevolvingCreditBuilder: ...
    def drawn_amount(self, amount: float) -> RevolvingCreditBuilder: ...
    def currency(self, currency: str | Currency) -> RevolvingCreditBuilder: ...
    def commitment_date(self, date: datetime.date) -> RevolvingCreditBuilder: ...
    def maturity(self, date: datetime.date) -> RevolvingCreditBuilder: ...
    def base_rate(self, spec: BaseRateSpec) -> RevolvingCreditBuilder: ...
    def day_count(self, day_count: str) -> RevolvingCreditBuilder: ...
    def frequency(self, frequency: str) -> RevolvingCreditBuilder: ...
    def fees(self, fees: RevolvingCreditFees) -> RevolvingCreditBuilder: ...
    def draw_repay(self, spec: DrawRepaySpec) -> RevolvingCreditBuilder: ...
    def disc_id(self, curve_id: str) -> RevolvingCreditBuilder: ...
    def credit_curve(self, curve_id: str | None = None) -> RevolvingCreditBuilder: ...
    def recovery_rate(self, rate: float) -> RevolvingCreditBuilder: ...
    def stub(self, stub: str) -> RevolvingCreditBuilder: ...
    def build(self) -> "RevolvingCredit": ...
    def __repr__(self) -> str: ...

class RevolvingCredit:
    """Revolving credit facility with flexible drawdown and repayment.

    RevolvingCredit represents a credit facility where the borrower can draw
    and repay funds within a commitment limit. Utilization can be deterministic
    (fixed schedule) or stochastic (modeled with Monte Carlo).

    Revolving credit facilities are used for working capital management and
    corporate financing. They require discount curves and optionally credit
    curves for pricing.

    Examples
    --------
    Create a revolving credit from JSON:

        >>> from finstack.valuations.instruments import RevolvingCredit
        >>> json_str = '''
        ... {
        ...     "id": "REVOLVER-001",
        ...     "commitment_amount": {"amount": 50000000, "currency": "USD"},
        ...     "drawn_amount": {"amount": 20000000, "currency": "USD"},
        ...     "commitment_date": "2024-01-01",
        ...     "maturity": "2029-01-01",
        ...     "base_rate_spec": {"Fixed": {"rate": 0.05}},
        ...     "day_count": "Act360",
        ...     "payment_frequency": {"count": 3, "unit": "months"},
        ...     "fees": {"facility_fee_bp": 0},
        ...     "draw_repay_spec": {"Deterministic": []},
        ...     "discount_curve_id": "USD",
        ...     "attributes": {"tags": [], "meta": {}}
        ... }
        ... '''
        >>> revolver = RevolvingCredit.from_json(json_str)
        >>> revolver.utilization_rate()  # 0.4 (20M / 50M)

    Create using the typed builder:

        >>> from datetime import date
        >>> rc = (
        ...     RevolvingCredit
        ...     .builder("RCF-001")
        ...     .commitment_amount(100_000_000)
        ...     .drawn_amount(50_000_000)
        ...     .currency("USD")
        ...     .commitment_date(date(2025, 1, 1))
        ...     .maturity(date(2030, 1, 1))
        ...     .base_rate(BaseRateSpec.fixed(0.055))
        ...     .fees(RevolvingCreditFees.flat(25, 10, 5))
        ...     .disc_id("USD-OIS")
        ...     .build()
        ... )

    Notes
    -----
    - Revolving credit requires discount curve and optionally credit curve
    - Commitment amount is the maximum available credit
    - Drawn amount is the current utilization
    - Utilization can be deterministic or stochastic
    - Commitment fees are paid on undrawn amounts
    - Interest is paid on drawn amounts

    MarketContext Requirements
    -------------------------
    - Discount curve: referenced by ``discount_curve_id`` in the JSON payload (required for pricing).
    - Optional hazard/credit curve: ``hazard_curve`` (used when set / when credit-sensitive pricing is selected).

    See Also
    --------
    :class:`TermLoan`: Term loans
    :class:`Bond`: Bonds
    :class:`PricerRegistry`: Pricing entry point

    Sources
    -------
    - Hull (text): see ``docs/REFERENCES.md#hullOptionsFuturesDerivatives``.
    - O'Kane (2008): see ``docs/REFERENCES.md#okane2008``.
    """

    @classmethod
    def from_json(cls, json_str: str) -> RevolvingCredit:
        """Create a revolving credit facility from a JSON string specification.

        Parameters
        ----------
        json_str : str
            JSON string containing revolving credit specification. Must include
            instrument_id, commitment_amount, drawn_amount, commitment_date,
            maturity_date, and discount_curve.

        Returns
        -------
        RevolvingCredit
            Configured revolving credit ready for pricing.

        Raises
        ------
        ValueError
            If JSON is invalid or required fields are missing.

        Examples
        --------
            >>> revolver = RevolvingCredit.from_json(json_str)
            >>> revolver.commitment_amount
            Money(50000000, Currency("USD"))
            >>> revolver.utilization_rate()
            0.4
        """
        ...

    @classmethod
    def builder(cls, instrument_id: str) -> RevolvingCreditBuilder:
        """Start a fluent builder for constructing a RevolvingCredit."""
        ...

    def to_json(self) -> str:
        """Serialize the revolving credit facility to a JSON string."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def commitment_amount(self) -> Money: ...
    @property
    def drawn_amount(self) -> Money: ...
    @property
    def commitment_date(self) -> datetime.date: ...
    @property
    def maturity_date(self) -> datetime.date: ...
    @property
    def currency(self) -> Currency: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def hazard_curve(self) -> str | None: ...
    @property
    def recovery_rate(self) -> float: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def utilization_rate(self) -> float:
        """Calculate current utilization rate (drawn / commitment)."""
        ...

    def undrawn_amount(self) -> Money:
        """Calculate current undrawn amount (available capacity)."""
        ...

    def is_deterministic(self) -> bool:
        """Check if the facility uses deterministic cashflows."""
        ...

    def is_stochastic(self) -> bool:
        """Check if the facility uses stochastic utilization."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class PathResult:
    """Single Monte Carlo path result."""

    @property
    def pv(self) -> Money: ...
    @property
    def cashflows(self) -> CashFlowSchedule: ...
    @property
    def path_data(self) -> ThreeFactorPathData | None: ...
    def __repr__(self) -> str: ...

class ThreeFactorPathData:
    """Three-factor utilization, spread, and rate paths."""

    @property
    def utilization_path(self) -> List[float]: ...
    @property
    def credit_spread_path(self) -> List[float]: ...
    @property
    def short_rate_path(self) -> List[float]: ...
    @property
    def time_points(self) -> List[float]: ...
    @property
    def payment_dates(self) -> List[datetime.date]: ...
    def __repr__(self) -> str: ...

class EnhancedMonteCarloResult:
    """Monte Carlo summary statistics with per-path details."""

    @property
    def mean(self) -> Money: ...
    @property
    def std_error(self) -> float: ...
    @property
    def ci_lower(self) -> Money: ...
    @property
    def ci_upper(self) -> Money: ...
    @property
    def num_paths(self) -> int: ...
    @property
    def path_results(self) -> List[PathResult]: ...
    def __repr__(self) -> str: ...
