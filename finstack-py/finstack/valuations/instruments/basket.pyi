"""Basket instrument."""

from typing import Union, Dict, Any
from ..common import InstrumentType

class Basket:
    """Basket instrument wrapper parsed from JSON definitions."""

    @classmethod
    def from_json(cls, data: Union[str, Dict[str, Any]]) -> "Basket":
        """Parse a basket definition from a JSON string or dictionary."""
        ...

    def to_json(self) -> str:
        """Serialize the basket definition to a JSON string."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
