"""Cashflow schedule JSON construction and validation."""

from __future__ import annotations

from finstack.finstack import cashflows as _cashflows

build_cashflow_schedule = _cashflows.build_cashflow_schedule
validate_cashflow_schedule = _cashflows.validate_cashflow_schedule
dated_flows = _cashflows.dated_flows
accrued_interest = _cashflows.accrued_interest
bond_from_cashflows = _cashflows.bond_from_cashflows

__all__: list[str] = [
    "accrued_interest",
    "bond_from_cashflows",
    "build_cashflow_schedule",
    "dated_flows",
    "validate_cashflow_schedule",
]
