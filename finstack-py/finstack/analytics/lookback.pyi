"""Type stubs for lookback period selectors."""

from __future__ import annotations
from datetime import date

def mtd_select(
    dates: list[date], ref_date: date, offset_days: int = 0
) -> tuple[int, int]:
    """Month-to-date index range into ``dates``."""
    ...

def qtd_select(
    dates: list[date], ref_date: date, offset_days: int = 0
) -> tuple[int, int]:
    """Quarter-to-date index range into ``dates``."""
    ...

def ytd_select(
    dates: list[date], ref_date: date, offset_days: int = 0
) -> tuple[int, int]:
    """Year-to-date index range into ``dates``."""
    ...

def fytd_select(
    dates: list[date],
    ref_date: date,
    fiscal_start_month: int = 10,
    fiscal_start_day: int = 1,
    offset_days: int = 0,
) -> tuple[int, int]:
    """Fiscal-year-to-date index range into ``dates``."""
    ...
