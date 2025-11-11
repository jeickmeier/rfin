"""Cashflow bindings for financial instruments.

Provides cashflow modeling primitives, amortization
specifications, and analytics (XIRR) for building payment schedules.
"""

from datetime import date
from .primitives import CashFlow, CFKind

def xirr(cash_flows: list[tuple[date, float]], guess: float | None = None) -> float:
    """Calculate XIRR (Extended Internal Rate of Return) for irregular cash flows.

    XIRR finds the discount rate that makes the net present value of all cash flows
    equal to zero. It's particularly useful for investments with irregular timing.

    Parameters
    ----------
    cash_flows : list[tuple[date, float]]
        List of (date, amount) pairs. Negative amounts represent outflows (investments),
        positive amounts represent inflows (returns).
    guess : float, optional
        Initial guess for the IRR (default: 0.1 = 10%). Providing a good guess can
        help convergence for difficult cases.

    Returns
    -------
    float
        The XIRR as a decimal (e.g., 0.15 for 15% annual return).

    Raises
    ------
    ValueError
        If less than 2 cash flows provided, or no sign change in cash flows.
    RuntimeError
        If the solver cannot converge to a solution.

    Examples
    --------
    >>> from datetime import date
    >>> from finstack.core.cashflow import xirr
    >>> # Investment with irregular cash flows
    >>> cash_flows = [
    ...     (date(2024, 1, 1), -100000.0),  # Initial investment
    ...     (date(2024, 6, 15), 5000.0),  # Mid-year dividend
    ...     (date(2025, 1, 1), 110000.0),  # Final value
    ... ]
    >>> irr = xirr(cash_flows)
    >>> print(f"IRR: {irr * 100:.2f}%")
    IRR: 15.23%
    """
    ...

__all__ = [
    "CashFlow",
    "CFKind",
    "xirr",
]
