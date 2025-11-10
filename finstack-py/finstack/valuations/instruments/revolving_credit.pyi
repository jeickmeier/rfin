"""
Revolving credit facility instrument.

A revolving credit facility with support for deterministic and stochastic
utilization modeling, flexible draws/repayments, and comprehensive fee structures.
"""

from datetime import date
from typing import Any

from finstack.core.money import Money

class RevolvingCredit:
    """
    Revolving credit facility instrument.

    Supports both deterministic and stochastic cashflow modeling with flexible
    draws/repayments, interest payments, and comprehensive fee structures
    (commitment, usage, facility, and upfront fees).
    """

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        commitment_amount: Money,
        drawn_amount: Money,
        commitment_date: date,
        maturity_date: date,
        base_rate_spec: dict[str, Any],
        payment_frequency: str | None,
        fees: dict[str, Any],
        draw_repay_spec: dict[str, Any],
        discount_curve: str,
        hazard_curve: str | None = None,
        recovery_rate: float = 0.0,
    ) -> RevolvingCredit:
        """
        Create a revolving credit facility.

        Args:
            instrument_id: Instrument identifier.
            commitment_amount: Total committed amount.
            drawn_amount: Initial drawn amount.
            commitment_date: Date when facility becomes available.
            maturity_date: Date when facility expires.
            base_rate_spec: Base rate specification (dict with 'type' and params).
                - For fixed: {'type': 'fixed', 'rate': 0.05}
                - For floating: {'type': 'floating', 'index_id': 'USD-SOFR-3M', 
                                'margin_bp': 150.0, 'reset_freq': 'quarterly'}
            payment_frequency: Payment frequency (e.g., 'quarterly').
            fees: Fee structure dict with keys:
                - upfront_fee: Optional upfront fee (Money)
                - commitment_fee_bp: Commitment fee in basis points
                - usage_fee_bp: Usage fee in basis points
                - facility_fee_bp: Facility fee in basis points
            draw_repay_spec: Draw/repayment specification.
                - Deterministic: {'deterministic': [list of events]}
                - Stochastic: {'stochastic': {dict with utilization_process, num_paths, etc.}}
            discount_curve: Discount curve identifier.
            hazard_curve: Optional hazard curve identifier for credit risk (e.g., 'BORROWER-A').
            recovery_rate: Recovery rate on default (e.g., 0.40 for 40%). Defaults to 0.0.

        Returns:
            Configured revolving credit instrument.
        """
        ...

    def npv(self, market: Any, as_of: date) -> Money:
        """
        Calculate net present value of the facility.

        Args:
            market: Market context with curves.
            as_of: Valuation date.

        Returns:
            Present value as Money.
        """
        ...

    def utilization_rate(self) -> float:
        """
        Get current utilization rate.

        Returns:
            Utilization rate as decimal (0.0 to 1.0).
        """
        ...

    def build_schedule(self, market: Any, as_of: date) -> Any:
        """
        Generate cashflow schedule.

        Args:
            market: Market context with curves.
            as_of: Schedule build date.

        Returns:
            CashFlow schedule object.
        """
        ...

    def per_period_pv(
        self,
        periods: list[Any],
        market: Any,
        discount_curve_id: str | None = None,
        as_of: date | None = None,
    ) -> dict[str, float]:
        """
        Compute per-period present values.

        Args:
            periods: List of period objects.
            market: Market context.
            discount_curve_id: Optional discount curve ID override.
            as_of: Optional valuation date override.

        Returns:
            Dictionary mapping period codes to PV amounts.
        """
        ...

    def to_period_dataframe(
        self,
        periods: list[Any],
        market: Any,
        *,
        discount_curve_id: str | None = None,
        hazard_curve_id: str | None = None,
        forward_curve_id: str | None = None,
        as_of: date | None = None,
        day_count: str | None = None,
        facility_limit: Money | None = None,
        include_floating_decomposition: bool = False,
    ) -> dict[str, list[Any]]:
        """
        Export cashflows to period-aligned DataFrame.

        Args:
            periods: List of period objects.
            market: Market context.
            discount_curve_id: Optional curve ID override.
            hazard_curve_id: Optional hazard curve ID for credit adjustment.
            forward_curve_id: Optional forward curve ID for floating decomposition.
            as_of: Optional valuation date.
            day_count: Optional day count convention.
            facility_limit: Optional facility limit for unfunded amount.
            include_floating_decomposition: Include base rate and spread columns.

        Returns:
            Dictionary with column names as keys and lists as values.
        """
        ...

    @property
    def instrument_id(self) -> str:
        """Get instrument identifier."""
        ...

    def __repr__(self) -> str: ...

