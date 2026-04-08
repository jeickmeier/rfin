"""JSON Schema helpers for Finstack instrument and result types."""

from __future__ import annotations

from typing import Any

def bond_schema() -> dict[str, Any]:
    """Return the JSON Schema for the Bond instrument configuration."""
    ...

def instrument_schema(instrument_type: str | None = None) -> dict[str, Any]:
    """Return the envelope schema or a schema for a specific instrument type."""
    ...

def instrument_types() -> list[str]:
    """Return canonical instrument type discriminators."""
    ...

def valuation_result_schema() -> dict[str, Any]:
    """Return the JSON Schema for the ValuationResult envelope."""
    ...

def validate_instrument_json(instrument_json: dict[str, Any]) -> None:
    """Validate an instrument JSON dict against the envelope schema.

    Args:
        instrument_json: Dict representing an instrument envelope, e.g.::

            {"schema": "finstack.instrument/1", "instrument": {"type": "bond", "spec": {...}}}

    Raises:
        ValidationError: If the JSON does not conform to the schema.
    """
    ...

def validate_instrument_type_json(instrument_type: str, instrument_json: dict[str, Any]) -> None:
    """Validate an instrument JSON dict against a specific instrument type's schema.

    Args:
        instrument_type: Canonical type (e.g., "bond", "interest_rate_swap").
        instrument_json: Dict representing the instrument envelope.

    Raises:
        ValidationError: If the JSON does not conform to the schema.
    """
    ...

__all__ = [
    "bond_schema",
    "instrument_schema",
    "instrument_types",
    "valuation_result_schema",
    "validate_instrument_json",
    "validate_instrument_type_json",
]
