"""Core utility functions.

Provides common utility functions for date conversion
and other core operations.
"""

from typing import Union
from datetime import date

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
