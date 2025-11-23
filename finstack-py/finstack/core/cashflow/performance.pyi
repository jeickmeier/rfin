"""Cashflow performance analytics (NPV, IRR)."""

from datetime import date
from typing import List, Tuple, Optional

def npv(
    cash_flows: List[Tuple[date, float]],
    discount_rate: float,
    base_date: Optional[date] = None,
    day_count: Optional[str] = None,
) -> float: ...

"""Calculate net present value of a cashflow stream.

Computes the present value of a series of cashflows discounted at a
specified rate. Cashflows are discounted to the base_date (or first
cashflow date if not provided).

Parameters
----------
cash_flows : List[Tuple[date, float]]
    List of (date, amount) tuples representing cashflows. Amounts can
    be positive (inflows) or negative (outflows).
discount_rate : float
    Annual discount rate as a decimal (e.g., 0.05 for 5%).
base_date : date, optional
    Base date for discounting. If None, uses the first cashflow date.
day_count : str, optional
    Day-count convention for time calculations (default: ACT/365).

Returns
-------
float
    Net present value (sum of discounted cashflows).

Raises
------
ValueError
    If cash_flows is empty or if discount_rate is invalid.

Examples
--------
        >>> from finstack.core.cashflow import npv
    >>> from datetime import date
    >>> 
    >>> cashflows = [
    ...     (date(2025, 1, 1), -1_000_000),  # Initial investment
    ...     (date(2025, 6, 30), 50_000),     # Coupon
    ...     (date(2025, 12, 31), 1_050_000) # Principal + coupon
    ... ]
    >>> pv = npv(cashflows, discount_rate=0.05, base_date=date(2025, 1, 1))
    >>> print(f"NPV: ${pv:,.2f}")
    NPV: $0.00

Notes
-----
- NPV = 0 when discount rate equals IRR
- Positive NPV indicates profitable investment
- Negative NPV indicates unprofitable investment
"""

def irr_periodic(amounts: List[float], guess: Optional[float] = None) -> float: ...

"""Calculate internal rate of return for periodic cashflows.

Computes the IRR for a series of cashflows with equal time intervals
(e.g., monthly, quarterly, annual). The first cashflow is typically
negative (initial investment), and subsequent cashflows are positive
(returns).

Parameters
----------
amounts : List[float]
    List of cashflow amounts in chronological order. First amount is
    typically negative (investment), subsequent amounts are positive
    (returns). All cashflows are assumed to occur at equal intervals.
guess : float, optional
    Initial guess for IRR (default: 0.1 for 10%). Should be close to
    expected IRR for faster convergence.

Returns
-------
float
    Internal rate of return as a decimal (e.g., 0.05 for 5% per period).

Raises
------
ValueError
    If amounts is empty, if all amounts have the same sign, or if
    convergence fails.

Examples
--------
        >>> from finstack.core.cashflow import irr_periodic
    >>> 
    >>> # Annual cashflows: -$1000 investment, $100, $200, $300, $400 returns
    >>> amounts = [-1_000, 100, 200, 300, 400]
    >>> irr = irr_periodic(amounts)
    >>> print(f"IRR: {irr*100:.2f}%")
    IRR: 8.45%

Notes
-----
- Assumes equal time intervals between cashflows
- Use xirr() for cashflows with irregular dates
- IRR is the discount rate that makes NPV = 0
- Multiple IRRs possible if cashflows change sign multiple times
"""
