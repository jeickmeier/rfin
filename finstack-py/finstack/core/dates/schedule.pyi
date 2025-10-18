"""Schedule and frequency bindings.

Provides payment schedule generation and frequency handling
for financial instruments.
"""

from typing import List, Optional, Union
from datetime import date
from .calendar import Calendar, BusinessDayConvention

class Frequency:
    """Payment frequency specification.
    
    Represents how often payments occur within a year.
    """
    
    @classmethod
    def from_months(cls, months: int) -> Frequency: ...
    """Construct a frequency based on a number of calendar months.
    
    Parameters
    ----------
    months : int
        Number of months between payments (1-12).
        
    Returns
    -------
    Frequency
        Frequency instance.
    """
    
    @classmethod
    def from_days(cls, days: int) -> Frequency: ...
    """Construct a frequency based on a number of days.
    
    Parameters
    ----------
    days : int
        Number of days between payments.
        
    Returns
    -------
    Frequency
        Frequency instance.
    """
    
    @property
    def months(self) -> Optional[int]: ...
    """Get the months component.
    
    Returns
    -------
    int or None
        Months if this is a monthly frequency.
    """
    
    @property
    def days(self) -> Optional[int]: ...
    """Get the days component.
    
    Returns
    -------
    int or None
        Days if this is a daily frequency.
    """
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

# Common frequency constants
DAILY: Frequency
WEEKLY: Frequency
MONTHLY: Frequency
QUARTERLY: Frequency
SEMIANNUAL: Frequency
ANNUAL: Frequency

class StubKind:
    """Stub period handling for irregular schedules.
    
    Available kinds:
    - ShortFirst: Short first period
    - ShortLast: Short last period
    - LongFirst: Long first period
    - LongLast: Long last period
    - NoStub: No stub handling
    """
    
    @classmethod
    def from_name(cls, name: str) -> StubKind: ...
    """Create from string name.
    
    Parameters
    ----------
    name : str
        Stub kind name (case-insensitive).
        
    Returns
    -------
    StubKind
        Stub kind instance.
    """
    
    @property
    def name(self) -> str: ...
    """Get the stub kind name.
    
    Returns
    -------
    str
        Human-readable stub kind name.
    """
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

# Stub kind constants
ShortFirst: StubKind
ShortLast: StubKind
LongFirst: StubKind
LongLast: StubKind
NoStub: StubKind

class ScheduleBuilder:
    """Builder for payment schedules.
    
    Provides a fluent interface for constructing payment schedules
    with various frequency and adjustment options.
    """
    
    def __init__(self, start: Union[str, date], end: Union[str, date]) -> None: ...
    """Create a schedule builder.
    
    Parameters
    ----------
    start : str or date
        Schedule start date.
    end : str or date
        Schedule end date.
    """
    
    def frequency(self, frequency: Frequency) -> ScheduleBuilder: ...
    """Set the payment frequency.
    
    Parameters
    ----------
    frequency : Frequency
        Payment frequency.
        
    Returns
    -------
    ScheduleBuilder
        Self for chaining.
    """
    
    def stub_rule(self, stub: StubKind) -> ScheduleBuilder: ...
    """Set the stub period handling.
    
    Parameters
    ----------
    stub : StubKind
        Stub period handling.
        
    Returns
    -------
    ScheduleBuilder
        Self for chaining.
    """
    
    def adjust_with(self, convention: BusinessDayConvention, calendar: Calendar) -> ScheduleBuilder: ...
    """Set business day adjustment.
    
    Parameters
    ----------
    convention : BusinessDayConvention
        Business day convention.
    calendar : Calendar
        Holiday calendar.
        
    Returns
    -------
    ScheduleBuilder
        Self for chaining.
    """
    
    def end_of_month(self, enabled: bool) -> ScheduleBuilder: ...
    """Enable/disable end-of-month adjustment.
    
    Parameters
    ----------
    enabled : bool
        Whether to adjust to end of month.
        
    Returns
    -------
    ScheduleBuilder
        Self for chaining.
    """
    
    def cds_imm(self) -> ScheduleBuilder: ...
    """Use CDS IMM dates.
    
    Returns
    -------
    ScheduleBuilder
        Self for chaining.
    """
    
    def build(self) -> Schedule: ...
    """Build the final schedule.
    
    Returns
    -------
    Schedule
        Constructed payment schedule.
    """
    
    def __repr__(self) -> str: ...

class Schedule:
    """A payment schedule with dates and metadata."""
    
    @property
    def dates(self) -> List[date]: ...
    """Dates contained in the schedule as datetime.date objects.
    
    Returns
    -------
    List[date]
        All payment dates.
    """
    
    def __len__(self) -> int: ...
    """Get the number of dates.
    
    Returns
    -------
    int
        Number of dates in the schedule.
    """
    
    def __repr__(self) -> str: ...
