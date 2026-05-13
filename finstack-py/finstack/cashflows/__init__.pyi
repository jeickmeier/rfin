"""Cashflow schedule JSON construction and validation."""

from __future__ import annotations

__all__ = [
    "accrued_interest",
    "bond_from_cashflows",
    "build_cashflow_schedule",
    "dated_flows",
    "validate_cashflow_schedule",
]

def build_cashflow_schedule(spec_json: str, market_json: str | None = None) -> str: ...
def validate_cashflow_schedule(schedule_json: str) -> str: ...
def dated_flows(schedule_json: str) -> str: ...
def accrued_interest(schedule_json: str, as_of: str, config_json: str | None = None) -> float:
    """Return accrued interest as a host-language ``float``.

    The Rust engine computes from the canonical schedule and crosses the
    binding boundary as ``f64``. For large notionals, compare with an absolute
    tolerance scaled to the schedule notional rather than expecting decimal
    string equality.
    """

def bond_from_cashflows(
    instrument_id: str,
    schedule_json: str,
    discount_curve_id: str,
    quoted_clean: float | None = None,
) -> str: ...
