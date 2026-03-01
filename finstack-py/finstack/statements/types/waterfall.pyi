"""Waterfall type bindings."""

from __future__ import annotations
from enum import Enum
from typing import List

class PaymentPriority(Enum):
    """Payment priority levels in the waterfall.

    Attributes:
        Fees: Fees (commitment fees, facility fees, etc.)
        Interest: Cash interest payments
        Amortization: Scheduled amortization
        MandatoryPrepayment: Mandatory prepayments
        VoluntaryPrepayment: Voluntary prepayments
        Sweep: Excess cash flow sweep
        Equity: Equity distributions
    """

    Fees = "fees"
    Interest = "interest"
    Amortization = "amortization"
    MandatoryPrepayment = "mandatory_prepayment"
    VoluntaryPrepayment = "voluntary_prepayment"
    Sweep = "sweep"
    Equity = "equity"

class EcfSweepSpec:
    """Excess Cash Flow (ECF) sweep specification.

    Defines how to calculate ECF and what percentage to sweep to pay down debt.
    """

    def __init__(
        self,
        ebitda_node: str,
        sweep_percentage: float,
        taxes_node: str | None = None,
        capex_node: str | None = None,
        working_capital_node: str | None = None,
        target_instrument_id: str | None = None,
    ) -> None:
        """Create an ECF sweep specification.

        Args:
            ebitda_node: Node reference for EBITDA
            sweep_percentage: Sweep percentage (e.g., 0.5 for 50%)
            taxes_node: Node reference for taxes
            capex_node: Node reference for capex
            working_capital_node: Node reference for WC change
            target_instrument_id: Target instrument ID for sweep payments
        """
        ...

    @property
    def ebitda_node(self) -> str: ...
    @property
    def sweep_percentage(self) -> float: ...
    @property
    def taxes_node(self) -> str | None: ...
    @property
    def capex_node(self) -> str | None: ...
    @property
    def working_capital_node(self) -> str | None: ...
    @property
    def target_instrument_id(self) -> str | None: ...
    def __repr__(self) -> str: ...

class PikToggleSpec:
    """PIK toggle specification.

    Defines conditions for switching between cash and PIK interest modes.
    """

    def __init__(
        self,
        liquidity_metric: str,
        threshold: float,
        target_instrument_ids: List[str] | None = None,
    ) -> None:
        """Create a PIK toggle specification.

        Args:
            liquidity_metric: Node reference for liquidity metric
            threshold: Threshold value (metric < threshold -> PIK)
            target_instrument_ids: List of instrument IDs to toggle
        """
        ...

    @property
    def liquidity_metric(self) -> str: ...
    @property
    def threshold(self) -> float: ...
    @property
    def target_instrument_ids(self) -> List[str] | None: ...
    def __repr__(self) -> str: ...

class WaterfallSpec:
    """Waterfall specification.

    Defines the priority of payments and sweep mechanics for capital structure.
    """

    def __init__(
        self,
        priority_of_payments: List[PaymentPriority] | None = None,
        ecf_sweep: EcfSweepSpec | None = None,
        pik_toggle: PikToggleSpec | None = None,
    ) -> None:
        """Create a waterfall specification.

        Args:
            priority_of_payments: Ordered list of payment priorities
            ecf_sweep: ECF sweep configuration
            pik_toggle: PIK toggle configuration
        """
        ...

    @property
    def priority_of_payments(self) -> List[PaymentPriority]: ...
    @property
    def ecf_sweep(self) -> EcfSweepSpec | None: ...
    @property
    def pik_toggle(self) -> PikToggleSpec | None: ...
    def __repr__(self) -> str: ...
