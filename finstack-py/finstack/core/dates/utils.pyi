"""Date utility functions for financial calculations.

This module provides helpers that mirror :mod:`finstack_core.dates.utils` and
selected methods from :trait:`finstack_core::dates::DateExt` for Python.
"""

from typing import Union
from datetime import date


def add_months(d: Union[str, date], months: int) -> date: ...
"""Add calendar months to a date (with end-of-month handling).

Parameters
----------
d : str or date
    Base date.
months : int
    Number of months to add (can be negative).

Returns
-------
date
    Date with months added.
"""


def last_day_of_month(d: Union[str, date]) -> date: ...
"""Get the last calendar day in the month for a date.

Parameters
----------
d : str or date
    Base date.

Returns
-------
date
    Last day of the month containing ``d``.
"""


def is_weekend(d: Union[str, date]) -> bool: ...
"""Return True if the date falls on a weekend (Saturday or Sunday)."""


def quarter(d: Union[str, date]) -> int: ...
"""Return the calendar quarter (1-4) for the given date."""


def fiscal_year(d: Union[str, date], config: "FiscalConfig") -> int: ...
"""Return the fiscal year for a date under the given FiscalConfig."""


def add_weekdays(d: Union[str, date], n: int) -> date: ...
"""Add/subtract a number of weekdays (Mon–Fri), ignoring holidays."""


def add_business_days(d: Union[str, date], n: int, calendar: "Calendar") -> date: ...
"""Add/subtract a number of business days using the supplied calendar."""


def is_business_day(d: Union[str, date], calendar: "Calendar") -> bool: ...
"""Return True if the date is a business day under the supplied calendar."""


def days_in_month(year: int, month: int) -> int: ...
"""Get the number of days in a month.

Parameters
----------
year : int
    Calendar year.
month : int
    Month (1-12).

Returns
-------
int
    Number of days in the month.
"""


def is_leap_year(year: int) -> bool: ...
"""Check if a year is a leap year."""


def date_to_days_since_epoch(d: Union[str, date]) -> int: ...
"""Convert a date to days since the Unix epoch (1970-01-01)."""


def days_since_epoch_to_date(days: int) -> date: ...
"""Convert days since the Unix epoch (1970-01-01) back to a date."""


def create_date(year: int, month: int, day: int) -> date: ...
"""Safely construct a date, raising ValueError for invalid calendar dates."""
