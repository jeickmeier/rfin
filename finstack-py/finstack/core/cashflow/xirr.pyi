"""Extended IRR (XIRR) for cashflows with irregular dates."""

from datetime import date
from typing import List, Tuple, Optional

def xirr(cash_flows: List[Tuple[date, float]], guess: Optional[float] = None) -> float:
    """Calculate extended internal rate of return for cashflows with irregular dates.

    XIRR computes the annualized IRR for a series of cashflows with arbitrary
    dates. Unlike irr_periodic(), XIRR handles cashflows at irregular intervals,
    making it suitable for real-world investments with non-periodic cashflows.

    Parameters
    ----------
    cash_flows : List[Tuple[date, float]]
        List of (date, amount) tuples representing cashflows. First cashflow
        is typically negative (initial investment), subsequent cashflows are
        positive (returns). Dates can be irregular.
    guess : float, optional
        Initial guess for XIRR (default: 0.1 for 10% annual). Should be close
        to expected return for faster convergence.

    Returns
    -------
    float
        Extended internal rate of return as an annual decimal (e.g., 0.08 for 8% annual).

    Raises
    ------
    ValueError
        If cash_flows is empty, if all amounts have the same sign, or if
        convergence fails.

    Examples
    --------
        >>> from finstack.core.cashflow.xirr import xirr
        >>> from datetime import date
        >>> # Irregular cashflows
        >>> cashflows = [
        ...     (date(2024, 1, 1), -1_000_000),  # Initial investment
        ...     (date(2024, 6, 15), 50_000),  # First return (irregular)
        ...     (date(2024, 9, 30), 75_000),  # Second return
        ...     (date(2025, 1, 1), 1_100_000),  # Final return
        ... ]
        >>> annual_irr = xirr(cashflows)
        >>> print(f"XIRR: {annual_irr * 100:.2f}%")
        XIRR: 12.34%

    Notes
    -----
    - Handles cashflows with irregular dates (unlike irr_periodic)
    - Returns annualized IRR regardless of cashflow timing
    - Uses actual day counts between cashflows
    - XIRR is the annual discount rate that makes NPV = 0

    See Also
    --------
    :func:`irr_periodic`: IRR for periodic cashflows
    :func:`npv`: Net present value calculation
    """
    ...
