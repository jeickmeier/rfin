"""Type stubs for period aggregation functions."""

from __future__ import annotations
from datetime import date
from .performance import PeriodStatsResult

def group_by_period(
    dates: list[date],
    returns: list[float],
    freq: str = "monthly",
) -> list[tuple[str, float]]:
    """Group daily returns by period, compounding within each bucket."""
    ...

def period_stats(
    dates: list[date],
    returns: list[float],
    freq: str = "monthly",
) -> PeriodStatsResult:
    """Compute period-level statistics from a return series."""
    ...

def grouped_returns(
    dates: list[date],
    returns: list[float],
    freq: str = "monthly",
) -> list[float]:
    """Return compounded per-period returns as a flat list."""
    ...
