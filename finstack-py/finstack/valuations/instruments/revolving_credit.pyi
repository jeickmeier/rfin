"""
Revolving credit facility instrument with deterministic and stochastic pricing.

Models a credit facility with draws/repayments, interest payments on drawn
amounts, and fees (commitment, usage, facility, upfront). Supports both
deterministic schedules and stochastic utilization via Monte Carlo.
"""

from datetime import date
from typing import Optional

from finstack.core.currency import Currency
from finstack.core.market_data import MarketContext
from finstack.core.money import Money
from finstack.valuations.cashflow import CashFlowSchedule
from finstack.valuations.results import ValuationResult

class RevolvingCredit:
    """
    Revolving credit facility instrument with deterministic and stochastic pricing.

    Models a credit facility with draws/repayments, interest payments on drawn
    amounts, and fees (commitment, usage, facility, upfront). Supports both
    deterministic schedules and stochastic utilization via Monte Carlo.

    Examples:
        >>> from finstack.valuations.instruments import RevolvingCredit
        >>> import json
        >>> # Create a simple fixed-rate revolver
        >>> facility_spec = {
        ...     "id": "RC001",
        ...     "commitment_amount": {"amount": 100_000_000, "currency": "USD"},
        ...     "drawn_amount": {"amount": 50_000_000, "currency": "USD"},
        ...     "commitment_date": "2025-01-01",
        ...     "maturity_date": "2030-01-01",
        ...     "base_rate_spec": {"Fixed": {"rate": 0.055}},
        ...     "day_count": "Act360",
        ...     "payment_frequency": {"months": 3},
        ...     "fees": {
        ...         "upfront_fee": {"amount": 500_000, "currency": "USD"},
        ...         "commitment_fee_tiers": [{"threshold": 0.0, "bps": 35}],
        ...         "usage_fee_tiers": [],
        ...         "facility_fee_bp": 10
        ...     },
        ...     "draw_repay_spec": {"Deterministic": []},
        ...     "discount_curve_id": "USD-OIS",
        ...     "attributes": {}
        ... }
        >>> rc = RevolvingCredit.from_json(json.dumps(facility_spec))
        >>> rc.instrument_id
        'RC001'
        >>> rc.utilization_rate()
        0.5
    """

    @classmethod
    def from_json(cls, json_str: str) -> RevolvingCredit:
        """
        Create a revolving credit facility from a JSON string specification.

        The JSON should match the RevolvingCredit schema from finstack-valuations.
        This is the recommended way to create facilities with complex features like
        stochastic utilization, tiered fees, and multi-factor Monte Carlo.

        Args:
            json_str: JSON string matching the RevolvingCredit schema.

        Returns:
            Configured revolving credit facility.

        Raises:
            ValueError: If JSON cannot be parsed or is invalid.

        Examples:
            >>> import json
            >>> spec = {
            ...     "id": "RC001",
            ...     "commitment_amount": {"amount": 100_000_000, "currency": "USD"},
            ...     "drawn_amount": {"amount": 0, "currency": "USD"},
            ...     "commitment_date": "2025-01-01",
            ...     "maturity_date": "2027-01-01",
            ...     "base_rate_spec": {"Fixed": {"rate": 0.05}},
            ...     "day_count": "Act360",
            ...     "payment_frequency": {"months": 3},
            ...     "fees": {"facility_fee_bp": 25},
            ...     "draw_repay_spec": {"Deterministic": []},
            ...     "discount_curve_id": "USD-OIS",
            ...     "attributes": {}
            ... }
            >>> rc = RevolvingCredit.from_json(json.dumps(spec))
        """
        ...

    def to_json(self) -> str:
        """
        Serialize the revolving credit facility to a JSON string.

        Returns:
            JSON representation of the facility.

        Examples:
            >>> json_str = rc.to_json()
            >>> # Can be saved to file or transmitted
        """
        ...

    @property
    def instrument_id(self) -> str:
        """
        Instrument identifier.

        Returns:
            Unique identifier assigned to the facility.
        """
        ...

    @property
    def commitment_amount(self) -> Money:
        """
        Total commitment amount (maximum drawable).

        Returns:
            Total commitment as Money.
        """
        ...

    @property
    def drawn_amount(self) -> Money:
        """
        Current drawn amount (initial utilization).

        Returns:
            Currently drawn amount as Money.
        """
        ...

    @property
    def commitment_date(self) -> date:
        """
        Commitment date (facility start date).

        Returns:
            Commitment date.
        """
        ...

    @property
    def maturity_date(self) -> date:
        """
        Maturity date (facility expiration).

        Returns:
            Maturity date.
        """
        ...

    @property
    def currency(self) -> Currency:
        """
        Currency for all cashflows.

        Returns:
            Currency object.
        """
        ...

    @property
    def discount_curve(self) -> str:
        """
        Discount curve identifier.

        Returns:
            Identifier for the discount curve.
        """
        ...

    @property
    def hazard_curve(self) -> Optional[str]:
        """
        Optional hazard curve identifier for credit risk modeling.

        Returns:
            Hazard curve ID if present, None otherwise.
        """
        ...

    @property
    def recovery_rate(self) -> float:
        """
        Recovery rate on default.

        Returns:
            Recovery rate (0.0 to 1.0).
        """
        ...

    @property
    def instrument_type(self) -> int:
        """
        Instrument type enum value.

        Returns:
            Enumeration value identifying the instrument family.
        """
        ...

    def utilization_rate(self) -> float:
        """
        Calculate current utilization rate (drawn / commitment).

        Returns:
            Utilization rate between 0.0 and 1.0.

        Examples:
            >>> rc.utilization_rate()
            0.5  # 50% utilized
        """
        ...

    def undrawn_amount(self) -> Money:
        """
        Calculate current undrawn amount (available capacity).

        Returns:
            Undrawn amount as Money.

        Raises:
            ValueError: If drawn amount exceeds commitment.

        Examples:
            >>> undrawn = rc.undrawn_amount()
            >>> print(f"Available: {undrawn}")
        """
        ...

    def is_deterministic(self) -> bool:
        """
        Check if the facility uses deterministic cashflows.

        Returns:
            True if using deterministic draw/repay schedule.
        """
        ...

    def is_stochastic(self) -> bool:
        """
        Check if the facility uses stochastic utilization.

        Returns:
            True if using Monte Carlo simulation.
        """
        ...

    def value(self, market: MarketContext, as_of: date) -> Money:
        """
        Price the facility using the standard value() method.

        For deterministic facilities, prices directly. For stochastic facilities,
        falls back to deterministic pricing with empty draw schedule (for fast path).
        Use price_with_paths() for full Monte Carlo with path capture.

        Args:
            market: Market context with required curves.
            as_of: Valuation date.

        Returns:
            Present value as Money.

        Raises:
            ValueError: If required curves are missing or valuation fails.

        Examples:
            >>> from datetime import date
            >>> pv = rc.value(market, date.today())
            >>> print(f"PV: {pv}")
        """
        ...

    def price_with_metrics(
        self, market: MarketContext, as_of: date, metrics: list[str]
    ) -> ValuationResult:
        """
        Price with requested risk metrics.

        Calculates present value along with requested metrics like DV01, CS01, etc.

        Args:
            market: Market context with required curves.
            as_of: Valuation date.
            metrics: List of metric identifiers (e.g., ["DV01", "CS01"]).

        Returns:
            Result with value and computed metrics.

        Raises:
            ValueError: If required curves are missing or valuation fails.

        Examples:
            >>> result = rc.price_with_metrics(market, date.today(), ["DV01", "CS01"])
            >>> print(f"PV: {result.value}")
            >>> print(f"DV01: {result.metrics['DV01']}")
        """
        ...

    def build_schedule(self, market: MarketContext, as_of: date) -> CashFlowSchedule:
        """
        Build cashflow schedule for deterministic facilities.

        Generates the complete cashflow schedule including interest payments,
        fees, draws, and repayments. Only works for deterministic specifications.

        Args:
            market: Market context with required curves.
            as_of: Valuation date.

        Returns:
            Detailed cashflow schedule.

        Raises:
            ValueError: If facility is stochastic or valuation fails.

        Examples:
            >>> schedule = rc.build_schedule(market, date.today())
            >>> for flow in schedule.flows:
            ...     print(f"{flow.date}: {flow.amount} - {flow.description}")
        """
        ...

    def price_deterministic(self, market: MarketContext, as_of: date) -> Money:
        """
        Price deterministically (explicit method for API clarity).

        Forces deterministic pricing even if the facility has a stochastic spec
        (treats as empty draw schedule). For true Monte Carlo, use price_with_paths().

        Args:
            market: Market context with required curves.
            as_of: Valuation date.

        Returns:
            Present value as Money.

        Raises:
            ValueError: If required curves are missing or valuation fails.
        """
        ...

    def price_with_paths(
        self, market: MarketContext, as_of: date
    ) -> EnhancedMonteCarloResult:
        """
        Price with full Monte Carlo path capture for distribution analysis.

        Runs Monte Carlo simulation and returns detailed results including
        individual path PVs, cashflows, and 3-factor path data for analysis.

        Only available when the facility uses a Stochastic draw/repay specification.

        Args:
            market: Market context with required curves.
            as_of: Valuation date.

        Returns:
            Full MC results with path details.

        Raises:
            ValueError: If facility is not stochastic or MC fails.

        Examples:
            >>> result = rc.price_with_paths(market, date.today())
            >>> print(f"Mean PV: {result.mean}")
            >>> print(f"Std Error: {result.std_error}")
            >>> # Analyze individual paths
            >>> for path in result.path_results[:10]:
            ...     print(f"Path PV: {path.pv}")
        """
        ...

    def __repr__(self) -> str: ...

class EnhancedMonteCarloResult:
    """
    Enhanced Monte Carlo result with full path details.

    Contains Monte Carlo statistics (mean, std error, confidence interval)
    along with individual path results for distribution analysis and visualization.
    """

    @property
    def mean(self) -> Money:
        """
        Mean present value across all paths.

        Returns:
            Mean PV estimate.
        """
        ...

    @property
    def std_error(self) -> float:
        """
        Standard error of the mean.

        Returns:
            Standard error in currency units.
        """
        ...

    @property
    def ci_lower(self) -> Money:
        """
        Lower bound of 95% confidence interval.

        Returns:
            Lower confidence bound.
        """
        ...

    @property
    def ci_upper(self) -> Money:
        """
        Upper bound of 95% confidence interval.

        Returns:
            Upper confidence bound.
        """
        ...

    @property
    def num_paths(self) -> int:
        """
        Number of simulated paths.

        Returns:
            Path count.
        """
        ...

    @property
    def path_results(self) -> list[PathResult]:
        """
        Individual path results for distribution analysis.

        Returns:
            List of path results with PV, cashflows, and factor data.
        """
        ...

    def __repr__(self) -> str: ...

class PathResult:
    """
    Individual path result from Monte Carlo simulation.

    Contains the present value, optional 3-factor path data, and cashflow schedule
    for a single simulated path.
    """

    @property
    def pv(self) -> Money:
        """
        Present value for this path.

        Returns:
            Path PV.
        """
        ...

    @property
    def cashflows(self) -> CashFlowSchedule:
        """
        Cashflow schedule for this path.

        Returns:
            Detailed cashflows.
        """
        ...

    @property
    def path_data(self) -> Optional[ThreeFactorPathData]:
        """
        Optional 3-factor path data (utilization, credit spread, short rate).

        Returns:
            Path data if available.
        """
        ...

    def __repr__(self) -> str: ...

class ThreeFactorPathData:
    """
    Three-factor path data from Monte Carlo simulation.

    Contains the simulated time series for utilization rate, credit spread,
    and short rate factors, along with time points and payment dates.
    """

    @property
    def utilization_path(self) -> list[float]:
        """
        Utilization rate path (0.0 to 1.0).

        Returns:
            Utilization rates at each time point.
        """
        ...

    @property
    def credit_spread_path(self) -> list[float]:
        """
        Credit spread path (annualized).

        Returns:
            Credit spreads at each time point.
        """
        ...

    @property
    def short_rate_path(self) -> list[float]:
        """
        Short rate path (annualized).

        Returns:
            Short rates at each time point.
        """
        ...

    @property
    def time_points(self) -> list[float]:
        """
        Time points (in years from as_of date).

        Returns:
            Time points for factor values.
        """
        ...

    @property
    def payment_dates(self) -> list[date]:
        """
        Payment dates corresponding to time points.

        Returns:
            Payment dates.
        """
        ...

    def __repr__(self) -> str: ...
