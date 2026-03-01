"""JSON Schema helpers for Finstack instrument and result types."""

from typing import Any

def bond_schema() -> dict[str, Any]:
    """Return the JSON Schema for the Bond instrument configuration."""
    ...

def valuation_result_schema() -> dict[str, Any]:
    """Return the JSON Schema for the ValuationResult envelope."""
    ...

__all__ = ["bond_schema", "valuation_result_schema"]
