"""Real-estate financial statement templates."""

from __future__ import annotations
from typing import Optional

from ..core.dates.periods import PeriodId

class LeaseSpec:
    """Lease specification for simple rent-roll modelling (v1)."""
    def __init__(
        self,
        node_id: str,
        start: PeriodId,
        base_rent: float,
        growth_rate: float = 0.0,
        end: PeriodId | None = None,
        free_rent_periods: int = 0,
        occupancy: float = 1.0,
    ) -> None: ...
    @property
    def node_id(self) -> str: ...
    @property
    def start(self) -> PeriodId: ...
    @property
    def base_rent(self) -> float: ...
    @property
    def growth_rate(self) -> float: ...
    @property
    def occupancy(self) -> float: ...
    def validate(self) -> None: ...
    def __repr__(self) -> str: ...

class RentStepSpec:
    """Discrete rent step specification."""
    def __init__(self, start: PeriodId, rent: float) -> None: ...
    @property
    def start(self) -> PeriodId: ...
    @property
    def rent(self) -> float: ...
    def __repr__(self) -> str: ...

class FreeRentWindowSpec:
    """Free-rent window specification."""
    def __init__(self, start: PeriodId, periods: int) -> None: ...
    @property
    def start(self) -> PeriodId: ...
    @property
    def periods(self) -> int: ...
    def __repr__(self) -> str: ...

class RenewalSpec:
    """Lease renewal assumption."""
    def __init__(
        self,
        downtime_periods: int = 0,
        term_periods: int = 12,
        probability: float = 1.0,
        rent_factor: float = 1.0,
        free_rent_periods: int = 0,
    ) -> None: ...
    @property
    def downtime_periods(self) -> int: ...
    @property
    def term_periods(self) -> int: ...
    @property
    def probability(self) -> float: ...
    @property
    def rent_factor(self) -> float: ...
    def validate(self) -> None: ...
    def __repr__(self) -> str: ...

class LeaseGrowthConvention:
    """Growth convention for lease escalation."""

    PER_PERIOD: LeaseGrowthConvention
    ANNUAL_ESCALATOR: LeaseGrowthConvention
    def __repr__(self) -> str: ...

class ManagementFeeBase:
    """Base metric for management fee calculation."""

    EGI: ManagementFeeBase
    EFFECTIVE_RENT: ManagementFeeBase
    def __repr__(self) -> str: ...

class ManagementFeeSpec:
    """Management fee specification."""
    def __init__(
        self,
        rate: float,
        base: ManagementFeeBase | None = None,
    ) -> None: ...
    @property
    def rate(self) -> float: ...
    def __repr__(self) -> str: ...

class LeaseSpecV2:
    """Enhanced lease specification with rent steps, free-rent windows, and renewal."""
    def __init__(
        self,
        node_id: str,
        start: PeriodId,
        base_rent: float,
        growth_rate: float = 0.0,
        growth_convention: LeaseGrowthConvention | None = None,
        end: PeriodId | None = None,
        rent_steps: list[RentStepSpec] | None = None,
        free_rent_periods: int = 0,
        free_rent_windows: list[FreeRentWindowSpec] | None = None,
        occupancy: float = 1.0,
        renewal: RenewalSpec | None = None,
    ) -> None: ...
    @property
    def node_id(self) -> str: ...
    @property
    def start(self) -> PeriodId: ...
    @property
    def base_rent(self) -> float: ...
    @property
    def growth_rate(self) -> float: ...
    @property
    def occupancy(self) -> float: ...
    def validate(self) -> None: ...
    def __repr__(self) -> str: ...

class RentRollOutputNodes:
    """Node names for rent-roll output allocation."""
    def __init__(
        self,
        rent_pgi_node: str = "rent_pgi",
        free_rent_node: str = "free_rent",
        vacancy_loss_node: str = "vacancy_loss",
        rent_effective_node: str = "rent_effective",
    ) -> None: ...
    def __repr__(self) -> str: ...

class PropertyTemplateNodes:
    """Node names for a full property operating statement template."""
    def __init__(
        self,
        rent_roll: RentRollOutputNodes | None = None,
        other_income_total_node: str = "other_income_total",
        egi_node: str = "egi",
        management_fee_node: str = "management_fee",
        opex_total_node: str = "opex_total",
        noi_node: str = "noi",
        capex_total_node: str = "capex_total",
        ncf_node: str = "ncf",
    ) -> None: ...
    def __repr__(self) -> str: ...

__all__ = [
    "LeaseSpec",
    "RentStepSpec",
    "FreeRentWindowSpec",
    "RenewalSpec",
    "LeaseGrowthConvention",
    "ManagementFeeBase",
    "ManagementFeeSpec",
    "LeaseSpecV2",
    "RentRollOutputNodes",
    "PropertyTemplateNodes",
]
