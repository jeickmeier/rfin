"""Performance helpers (XIRR, IRR, NPV) delegated to finstack-core.

These functions are thin stubs that mirror the compiled Rust bindings and
exist purely for IDE completion and inline help. All calculations are
performed in Rust; the stubs document the expected shapes and behavior.
"""

from typing import Iterable, List, Optional, Tuple, Union
from datetime import date

from ..core.dates.daycount import DayCount

def xirr(cash_flows: Iterable[Tuple[date, float]], guess: Optional[float] = ...) -> float:
    """Calculate XIRR (Extended Internal Rate of Return) for irregular cash flows.

    Parameters
    ----------
    cash_flows:
        Iterable of ``(date, amount)`` pairs. Negative amounts are outflows
        (investments); positive amounts are inflows (returns).
    guess:
        Optional initial guess for the root-finder. Defaults to ``0.1`` (10%).

    Returns
    -------
    float
        Annualized IRR expressed as a decimal (``0.12`` = 12%).

    Raises
    ------
    ValueError
        If fewer than two cash flows are provided or no sign change exists.
    RuntimeError
        If the solver cannot converge.
    """
    ...

def npv(
    cash_flows: Iterable[Tuple[date, float]],
    discount_rate: float,
    base_date: Optional[date] = ...,
    day_count: Optional[Union[str, DayCount]] = ...,
) -> float:
    """Compute Net Present Value for dated cash flows.

    Parameters
    ----------
    cash_flows:
        Iterable of ``(date, amount)`` pairs.
    discount_rate:
        Annual discount rate as a decimal (``0.05`` = 5%).
    base_date:
        Optional base date; defaults to the first cash flow date.
    day_count:
        Optional day-count convention (DayCount object or string identifier like
        ``"act365f"``, ``"act360"``, ``"30/360"``). Defaults to Act/365F if omitted.

    Returns
    -------
    float
        Net present value using the supplied discount rate.
    """
    ...

def irr_periodic(amounts: List[float], guess: Optional[float] = ...) -> float:
    """Compute IRR for evenly spaced (periodic) cash flows.

    Parameters
    ----------
    amounts:
        Cash flow amounts in order of occurrence; spacing between flows is
        assumed constant (monthly/quarterly/etc.).
    guess:
        Optional initial guess for the solver.

    Returns
    -------
    float
        Periodic IRR as a decimal. Convert to annual using compounding
        appropriate to your period length.
    """
    ...
