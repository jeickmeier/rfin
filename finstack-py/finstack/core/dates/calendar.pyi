"""Calendar and business day convention bindings.

Provides business day adjustment logic and holiday calendar management
for financial date calculations.
"""

from typing import List, Optional, Union
from datetime import date

class BusinessDayConvention:
    """Business day adjustment convention.

    Available conventions:
    - Following: Move to next business day
    - Preceding: Move to previous business day
    - ModifiedFollowing: Following, but if in next month, use Preceding
    - ModifiedPreceding: Preceding, but if in previous month, use Following
    - Unadjusted: No adjustment
    """

    @classmethod
    def from_name(cls, name: str) -> BusinessDayConvention: ...
    """Create from string name.
    
    Parameters
    ----------
    name : str
        Convention name (case-insensitive).
        
    Returns
    -------
    BusinessDayConvention
        Convention instance.
    """

    @property
    def name(self) -> str: ...
    """Get the convention name.
    
    Returns
    -------
    str
        Human-readable convention name.
    """

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...

# Business day convention constants
Following: BusinessDayConvention
Preceding: BusinessDayConvention
ModifiedFollowing: BusinessDayConvention
ModifiedPreceding: BusinessDayConvention
Unadjusted: BusinessDayConvention

class Calendar:
    """Holiday calendar with business day logic.

    Provides methods to check business days and holidays for various
    financial centers and jurisdictions.
    """

    @property
    def code(self) -> str: ...
    """Short calendar identifier (matching the registry code).
    
    Returns
    -------
    str
        Calendar code (e.g. "US", "GB", "JP").
    """

    @property
    def name(self) -> str: ...
    """Full calendar name.
    
    Returns
    -------
    str
        Human-readable calendar name.
    """

    @property
    def ignore_weekends(self) -> bool: ...
    """Whether weekends are treated as holidays.
    
    Returns
    -------
    bool
        True if weekends are holidays.
    """

    def is_business_day(self, date: Union[str, date]) -> bool: ...
    """Check if a date is a business day.
    
    Parameters
    ----------
    date : str or date
        Date to check.
        
    Returns
    -------
    bool
        True if the date is a business day.
    """

    def is_holiday(self, date: Union[str, date]) -> bool: ...
    """Check if a date is a holiday.
    
    Parameters
    ----------
    date : str or date
        Date to check.
        
    Returns
    -------
    bool
        True if the date is a holiday.
    """

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...

def available_calendars() -> List[Calendar]: ...

"""Get all available holiday calendars.

Returns
-------
List[Calendar]
    All supported holiday calendars.
"""

def available_calendar_codes() -> List[str]: ...

"""Get all available calendar codes.

Returns
-------
List[str]
    All supported calendar codes.
"""

def get_calendar(code: str) -> Calendar: ...

"""Get a calendar by code.

Parameters
----------
code : str
    Calendar code (e.g. "US", "GB").

Returns
-------
Calendar
    Calendar instance.

Raises
------
ValueError
    If calendar code is not found.
"""

def adjust(date: Union[str, date], convention: BusinessDayConvention, calendar: Calendar) -> date: ...

"""Adjust a date to a business day according to a convention and calendar.

This is the primary function for business day adjustments in financial
calculations. It moves a date to the nearest business day based on the
specified convention, respecting holidays defined in the calendar.

Parameters
----------
date : str or date
    Date to adjust. Can be a ``datetime.date`` object or an ISO-8601 date
    string (e.g., "2024-01-15").
convention : BusinessDayConvention
    Adjustment rule to apply:
    
    - ``FOLLOWING``: Move to next business day
    - ``PRECEDING``: Move to previous business day
    - ``MODIFIED_FOLLOWING``: Following, but if result is in next month,
      use Preceding instead
    - ``MODIFIED_PRECEDING``: Preceding, but if result is in previous month,
      use Following instead
    - ``UNADJUSTED``: Return date unchanged
calendar : Calendar
    Holiday calendar defining business days. Use :func:`get_calendar` to
    retrieve a calendar by code (e.g., "USNY", "GBLO", "JPTO").

Returns
-------
date
    Adjusted date that is a business day according to the calendar and
    convention.

Raises
------
ValueError
    If the date string cannot be parsed or if the adjustment fails.

Examples
--------
Adjust a date falling on a weekend:

    >>> from finstack import adjust, get_calendar, BusinessDayConvention
    >>> from datetime import date
    >>> cal = get_calendar("USNY")
    >>> # Saturday, January 6, 2024
    >>> sat = date(2024, 1, 6)
    >>> adjusted = adjust(sat, BusinessDayConvention.FOLLOWING, cal)
    >>> print(adjusted)  # Monday, January 8, 2024
    2024-01-08

Adjust a date falling on a holiday:

    >>> # New Year's Day 2024 (Monday)
    >>> new_year = date(2024, 1, 1)
    >>> adjusted = adjust(new_year, BusinessDayConvention.FOLLOWING, cal)
    >>> print(adjusted)  # Tuesday, January 2, 2024
    2024-01-02

Use Modified Following to avoid month boundaries:

    >>> # Last day of month that's a holiday
    >>> month_end = date(2024, 1, 31)  # Wednesday
    >>> # If this were a holiday, Modified Following would move backward
    >>> adjusted = adjust(month_end, BusinessDayConvention.MODIFIED_FOLLOWING, cal)
    >>> # Result stays in January if possible

Unadjusted date:

    >>> d = date(2024, 1, 15)
    >>> result = adjust(d, BusinessDayConvention.UNADJUSTED, cal)
    >>> result == d
    True

Notes
-----
- This function is re-exported at the ``finstack`` package root for
  convenience: ``from finstack import adjust``
- Business days exclude weekends (unless calendar ignores weekends) and
  holidays defined in the calendar
- Modified conventions prevent crossing month boundaries when possible
- Use :func:`get_calendar` to retrieve standard calendars (USNY, GBLO, etc.)

See Also
--------
:class:`BusinessDayConvention`: Available adjustment conventions
:class:`Calendar`: Holiday calendar definitions
:func:`get_calendar`: Retrieve calendars by code
"""
