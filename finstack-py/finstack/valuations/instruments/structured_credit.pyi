"""Structured credit instrument."""

from typing import Optional, Dict, Any, Union
from ..common import InstrumentType

class StructuredCredit:
    """Unified structured credit instrument wrapper (ABS, CLO, CMBS, RMBS)."""

    @classmethod
    def from_json(cls, data: Union[str, Dict[str, Any]]) -> "StructuredCredit":
        """Parse a JSON payload into a structured credit instrument."""
        ...

    def to_json(self) -> str:
        """Serialize the structured credit definition back to JSON."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def deal_type(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    @property
    def tranche_count(self) -> int: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
