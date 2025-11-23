"""Cashflow bindings for financial instruments.

Provides cashflow modeling primitives, amortization
specifications, and analytics (XIRR) for building payment schedules.
"""

from datetime import date
from .primitives import CashFlow, CFKind
from .performance import npv, irr_periodic
from .xirr import xirr

__all__ = [
    "CashFlow",
    "CFKind",
    "xirr",
    "npv",
    "irr_periodic",
]
