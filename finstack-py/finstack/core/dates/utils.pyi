"""Date utility functions.

Provides common date manipulation and arithmetic functions
for financial calculations.
"""

from typing import Union
from datetime import date

def add_months(date: Union[str, date], months: int) -> date: ...

"""Add months to a date.

Parameters
----------
date : str or date
    Base date.
months : int
    Number of months to add (can be negative).

Returns
-------
date
    Date with months added.
"""

def last_day_of_month(date: Union[str, date]) -> date: ...

"""Get the last day of the month for a date.

Parameters
----------
date : str or date
    Base date.

Returns
-------
date
    Last day of the month.
"""

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

"""Check if a year is a leap year.

Parameters
----------
year : int
    Calendar year.

Returns
-------
bool
    True if the year is a leap year.
"""

def date_to_days_since_epoch(date: Union[str, date]) -> int: ...

"""Convert date to days since epoch.

Parameters
----------
date : str or date
    Date to convert.

Returns
-------
int
    Days since epoch (1970-01-01).
"""

def days_since_epoch_to_date(days: int) -> date: ...

"""Convert days since epoch to date.

Parameters
----------
days : int
    Days since epoch (1970-01-01).

Returns
-------
date
    Corresponding date.
"""
