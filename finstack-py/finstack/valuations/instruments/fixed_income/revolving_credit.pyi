"""Revolving credit facility instrument."""

from __future__ import annotations
from typing import Dict, Any, List
from datetime import date
from ....core.money import Money
from ....core.currency import Currency
from ...common import InstrumentType
from ...cashflow.builder import CashFlowSchedule

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
    def from_json(cls, json_str: str) -> "RevolvingCredit":
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
    def commitment_date(self) -> date: ...
    @property
    def maturity_date(self) -> date: ...
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
    def path_data(self) -> "ThreeFactorPathData" | None: ...
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
    def payment_dates(self) -> List[date]: ...
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
