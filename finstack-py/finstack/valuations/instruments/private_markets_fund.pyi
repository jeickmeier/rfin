"""Private markets fund instrument."""

from typing import Optional, Dict, Any, List, Tuple, Union
from datetime import date
from ...core.money import Money
from ...core.currency import Currency
from ..common import InstrumentType

class PrivateMarketsFund:
    """Private markets fund instrument wrapper parsed from JSON definitions."""

    @classmethod
    def from_json(cls, data: Union[str, Dict[str, Any]]) -> "PrivateMarketsFund":
        """Create a private markets fund from JSON string or dictionary."""
        ...

    def to_json(self) -> str:
        """Serialize the fund to a JSON string."""
        ...

    def lp_cashflows(self) -> List[Tuple[date, Money]]:
        """Calculate LP cashflows."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def currency(self) -> Currency: ...
    @property
    def discount_curve(self) -> Optional[str]: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
