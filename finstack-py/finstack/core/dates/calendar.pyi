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
"""Adjust a date according to business day convention.

Parameters
----------
date : str or date
    Date to adjust.
convention : BusinessDayConvention
    Adjustment convention.
calendar : Calendar
    Holiday calendar to use.

Returns
-------
date
    Adjusted date.
"""
