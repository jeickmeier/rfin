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

__all__ = [
    "bond_schema",
    "instrument_schema",
    "instrument_types",
    "valuation_result_schema",
]
