"""Period and fiscal calendar bindings.

Provides period identification and fiscal calendar management
for financial modeling and reporting.
"""

from typing import List, Optional, Union
from datetime import date

class FiscalConfig:
    """Fiscal calendar configuration.
    
    Defines the start of the fiscal year for various jurisdictions
    and reporting standards.
    """
    
    def __init__(self, start_month: int, start_day: int) -> None: ...
    """Create a fiscal calendar starting on the given month/day.
    
    Parameters
    ----------
    start_month : int
        Month (1-12) when fiscal year starts.
    start_day : int
        Day (1-31) when fiscal year starts.
    """
    
    @classmethod
    def CALENDAR_YEAR(cls) -> FiscalConfig: ...
    """Calendar year fiscal config (Jan 1)."""
    
    @classmethod
    def US_FEDERAL(cls) -> FiscalConfig: ...
    """US Federal fiscal year (Oct 1)."""
    
    @classmethod
    def UK(cls) -> FiscalConfig: ...
    """UK fiscal year (Apr 6)."""
    
    @classmethod
    def JAPAN(cls) -> FiscalConfig: ...
    """Japan fiscal year (Apr 1)."""
    
    @classmethod
    def CANADA(cls) -> FiscalConfig: ...
    """Canada fiscal year (Apr 1)."""
    
    @classmethod
    def AUSTRALIA(cls) -> FiscalConfig: ...
    """Australia fiscal year (Jul 1)."""
    
    @classmethod
    def GERMANY(cls) -> FiscalConfig: ...
    """Germany fiscal year (Jan 1)."""
    
    @classmethod
    def FRANCE(cls) -> FiscalConfig: ...
    """France fiscal year (Jan 1)."""
    
    @property
    def start_month(self) -> int: ...
    """Get the start month.
    
    Returns
    -------
    int
        Month (1-12) when fiscal year starts.
    """
    
    @property
    def start_day(self) -> int: ...
    """Get the start day.
    
    Returns
    -------
    int
        Day (1-31) when fiscal year starts.
    """
    
    def __repr__(self) -> str: ...

class PeriodId:
    """Period identifier for quarters, months, weeks, etc."""
    
    @classmethod
    def quarter(cls, year: int, quarter: int) -> PeriodId: ...
    """Construct a period id for a specific calendar quarter.
    
    Parameters
    ----------
    year : int
        Calendar year.
    quarter : int
        Quarter (1-4).
        
    Returns
    -------
    PeriodId
        Quarter period identifier.
    """
    
    @classmethod
    def month(cls, year: int, month: int) -> PeriodId: ...
    """Construct a period id for a specific month.
    
    Parameters
    ----------
    year : int
        Calendar year.
    month : int
        Month (1-12).
        
    Returns
    -------
    PeriodId
        Month period identifier.
    """
    
    @classmethod
    def week(cls, year: int, week: int) -> PeriodId: ...
    """Construct a period id for a specific week.
    
    Parameters
    ----------
    year : int
        Calendar year.
    week : int
        Week number (1-53).
        
    Returns
    -------
    PeriodId
        Week period identifier.
    """
    
    @classmethod
    def half(cls, year: int, half: int) -> PeriodId: ...
    """Construct a period id for a half year.
    
    Parameters
    ----------
    year : int
        Calendar year.
    half : int
        Half (1-2).
        
    Returns
    -------
    PeriodId
        Half year period identifier.
    """
    
    @classmethod
    def annual(cls, year: int) -> PeriodId: ...
    """Construct a period id for a full year.
    
    Parameters
    ----------
    year : int
        Calendar year.
        
    Returns
    -------
    PeriodId
        Annual period identifier.
    """
    
    @classmethod
    def parse(cls, code: str) -> PeriodId: ...
    """Parse a period id from string code.
    
    Parameters
    ----------
    code : str
        Period code (e.g. "2023Q1", "2023M03").
        
    Returns
    -------
    PeriodId
        Parsed period identifier.
    """
    
    @property
    def code(self) -> str: ...
    """Get the period code.
    
    Returns
    -------
    str
        Period code string.
    """
    
    @property
    def year(self) -> int: ...
    """Get the year.
    
    Returns
    -------
    int
        Calendar year.
    """
    
    @property
    def index(self) -> int: ...
    """Get the period index.
    
    Returns
    -------
    int
        Period index within the year.
    """
    
    @property
    def kind(self) -> str: ...
    """Get the period kind.
    
    Returns
    -------
    str
        Period type (e.g. "quarter", "month").
    """
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class Period:
    """A time period with start/end dates and metadata."""
    
    @property
    def id(self) -> PeriodId: ...
    """Identifier for this period (quarter/month/etc.).
    
    Returns
    -------
    PeriodId
        Period identifier.
    """
    
    def start(self) -> date: ...
    """Get the start date.
    
    Returns
    -------
    date
        Period start date.
    """
    
    def end(self) -> date: ...
    """Get the end date.
    
    Returns
    -------
    date
        Period end date.
    """
    
    @property
    def is_actual(self) -> bool: ...
    """Whether this is an actual (historical) period.
    
    Returns
    -------
    bool
        True if this is an actual period.
    """
    
    def __repr__(self) -> str: ...

class PeriodPlan:
    """A collection of periods for financial modeling."""
    
    @property
    def periods(self) -> List[Period]: ...
    """Period entries contained in the plan.
    
    Returns
    -------
    List[Period]
        All periods in the plan.
    """
    
    def __len__(self) -> int: ...
    """Get the number of periods.
    
    Returns
    -------
    int
        Number of periods.
    """
    
    def __repr__(self) -> str: ...

def build_periods(range: str, actuals_until: Optional[str] = None) -> PeriodPlan: ...
"""Build periods from a range specification.

Parameters
----------
range : str
    Period range (e.g. "2023Q1:2024Q4").
actuals_until : str, optional
    Last actual period (e.g. "2023Q3").

Returns
-------
PeriodPlan
    Collection of periods.
"""

def build_fiscal_periods(range: str, config: FiscalConfig, actuals_until: Optional[str] = None) -> PeriodPlan: ...
"""Build fiscal periods from a range specification.

Parameters
----------
range : str
    Period range (e.g. "2023Q1:2024Q4").
config : FiscalConfig
    Fiscal calendar configuration.
actuals_until : str, optional
    Last actual period.

Returns
-------
PeriodPlan
    Collection of fiscal periods.
"""
