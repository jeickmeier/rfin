"""Revolving credit facility instrument."""

from typing import Optional, Dict, Any, List, Union
from datetime import date
from ...core.money import Money
from ...core.currency import Currency
from ..common import InstrumentType
from ..cashflow.builder import CashFlowSchedule

class RevolvingCredit:
    """Revolving credit facility instrument with deterministic and stochastic pricing."""

    @classmethod
    def from_json(cls, json_str: str) -> "RevolvingCredit":
        """Create a revolving credit facility from a JSON string specification."""
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
    def hazard_curve(self) -> Optional[str]: ...
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
