"""Cashflow bindings for financial instruments.

Provides cashflow modeling primitives, amortization
specifications, and analytics (XIRR) for building payment schedules.
"""

from __future__ import annotations
from datetime import date
from .primitives import CashFlow, CFKind
from .performance import npv, irr_periodic
from .xirr import xirr
from ..money import Money
from ..dates.daycount import DayCount
from ..market_data.term_structures import DiscountCurve

__all__ = [
    "CashFlow",
    "CFKind",
    "xirr",
    "npv",
    "irr_periodic",
    "npv_static",
    "npv_using_curve_dc",
]

def npv_static(
    curve: DiscountCurve,
    base_date: date | str,
    day_count: DayCount | str,
    cash_flows: list[tuple[date, Money]],
) -> Money:
    """Compute NPV of cashflows using a discount curve with explicit day-count.

    Calculates the present value of a series of dated cashflows by
    interpolating discount factors from the provided curve and applying
    the specified day-count convention to compute time fractions.

    Parameters
    ----------
    curve : DiscountCurve
        The discount curve providing discount factors for each tenor.
    base_date : date or str
        The valuation date from which time fractions are measured.
    day_count : DayCount or str
        Day-count convention for computing year fractions (e.g.,
        ``"act365f"``, ``"act360"``, ``"30/360"``).
    cash_flows : list[tuple[date, Money]]
        List of (payment_date, amount) pairs. All amounts must share
        the same currency.

    Returns
    -------
    Money
        The net present value in the same currency as the input cashflows.

    Raises
    ------
    ValueError
        If cashflows have mismatched currencies or dates are invalid.
    RuntimeError
        If curve interpolation fails.

    See Also
    --------
    npv_using_curve_dc : Use curve's internal day-count convention.
    """
    ...

def npv_using_curve_dc(
    curve: DiscountCurve,
    base_date: date | str,
    cash_flows: list[tuple[date, Money]],
) -> Money:
    """Compute NPV of cashflows using the curve's internal day-count convention.

    A convenience function that uses the day-count convention stored in the
    discount curve itself, ensuring consistency between curve construction
    and NPV calculation.

    Parameters
    ----------
    curve : DiscountCurve
        The discount curve providing discount factors. The curve's internal
        day-count convention will be used for time fraction calculations.
    base_date : date or str
        The valuation date from which time fractions are measured.
    cash_flows : list[tuple[date, Money]]
        List of (payment_date, amount) pairs. All amounts must share
        the same currency.

    Returns
    -------
    Money
        The net present value in the same currency as the input cashflows.

    Raises
    ------
    ValueError
        If cashflows have mismatched currencies or dates are invalid.
    RuntimeError
        If curve interpolation fails.

    See Also
    --------
    npv_static : Explicit day-count convention.
    """
    ...
