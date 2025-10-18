"""Cashflow bindings for financial instruments.

Provides cashflow modeling primitives and amortization
specifications for building payment schedules.
"""

from .primitives import CashFlow, CFKind, AmortizationSpec

__all__ = [
    "CashFlow",
    "CFKind", 
    "AmortizationSpec",
]
