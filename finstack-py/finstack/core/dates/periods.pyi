"""Period and fiscal calendar bindings.

Provides period identification and fiscal calendar management
for financial modeling and reporting.
"""

from typing import List, Optional
from datetime import date

class FiscalConfig:
    """Fiscal calendar configuration.

    Defines the start of the fiscal year for various jurisdictions
    and reporting standards.
    """

    def __init__(self, start_month: int, start_day: int) -> None: ...
    @classmethod
    def CALENDAR_YEAR(cls) -> "FiscalConfig": ...
    @classmethod
    def US_FEDERAL(cls) -> "FiscalConfig": ...
    @classmethod
    def UK(cls) -> "FiscalConfig": ...
    @classmethod
    def JAPAN(cls) -> "FiscalConfig": ...
    @classmethod
    def CANADA(cls) -> "FiscalConfig": ...
    @classmethod
    def AUSTRALIA(cls) -> "FiscalConfig": ...
    @classmethod
    def GERMANY(cls) -> "FiscalConfig": ...
    @classmethod
    def FRANCE(cls) -> "FiscalConfig": ...
    @property
    def start_month(self) -> int: ...
    @property
    def start_day(self) -> int: ...
    def __repr__(self) -> str: ...

class PeriodId:
    """Period identifier for quarters, months, weeks, halves, or years."""

    @classmethod
    def quarter(cls, year: int, quarter: int) -> "PeriodId": ...
    @classmethod
    def month(cls, year: int, month: int) -> "PeriodId": ...
    @classmethod
    def week(cls, year: int, week: int) -> "PeriodId": ...
    @classmethod
    def half(cls, year: int, half: int) -> "PeriodId": ...
    @classmethod
    def annual(cls, year: int) -> "PeriodId": ...
    @classmethod
    def parse(cls, code: str) -> "PeriodId": ...
    @property
    def code(self) -> str: ...
    @property
    def year(self) -> int: ...
    @property
    def index(self) -> int: ...
    @property
    def kind(self) -> str: ...
    def periods_per_year(self) -> int: ...
    """Number of periods per year implied by the period kind."""

    def next(self) -> "PeriodId": ...
    """Next period in sequence."""

    def prev(self) -> "PeriodId": ...
    """Previous period in sequence."""

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class Period:
    """A time period with start/end dates and metadata."""

    @property
    def id(self) -> PeriodId: ...
    def start(self) -> date: ...
    def end(self) -> date: ...
    @property
    def is_actual(self) -> bool: ...
    def __repr__(self) -> str: ...

class PeriodPlan:
    """A collection of periods for financial modeling."""

    @property
    def periods(self) -> List[Period]: ...
    def __len__(self) -> int: ...
    def __repr__(self) -> str: ...

def build_periods(range: str, actuals_until: Optional[str] = None) -> PeriodPlan: ...

"""Build a sequence of calendar periods from a date range expression.

Creates a :class:`PeriodPlan` containing monthly, quarterly, or annual
periods based on a date range string. This is the primary function for
generating period structures for financial statement modeling and reporting.

Parameters
----------
range : str
    Date range expression in the format "YYYY-MM-DD to YYYY-MM-DD" or
    "YYYY-MM-DD/YYYY-MM-DD". The range defines the start and end dates
    for period generation.
actuals_until : str, optional
    ISO-8601 date string marking the boundary between actual and forecast
    periods. Periods on or before this date are marked as actuals;
    periods after are forecasts. If ``None``, all periods are forecasts.

Returns
-------
PeriodPlan
    Collection of periods covering the specified date range. Each period
    has start/end dates and metadata indicating whether it's an actual
    or forecast period.

Raises
------
ValueError
    If the range string cannot be parsed or if dates are invalid.

Examples
--------
Build monthly periods for a year:

    >>> from finstack.core.dates.periods import build_periods
    >>> plan = build_periods("2024-01-01 to 2024-12-31")
    >>> len(plan.periods)
    12
    >>> first = plan.periods[0]
    >>> print(first.start())
    2024-01-01
    >>> print(first.end())
    2024-01-31

Build quarterly periods:

    >>> plan = build_periods("2024-01-01 to 2024-12-31")
    >>> # Periods are monthly by default, but can be aggregated to quarters

Mark actuals vs forecasts:

    >>> plan = build_periods(
    ...     "2024-01-01 to 2024-12-31",
    ...     actuals_until="2024-06-30"
    ... )
    >>> # Periods through June are actuals, July-December are forecasts
    >>> plan.periods[0].is_actual  # January
    True
    >>> plan.periods[6].is_actual  # July
    False

Notes
-----
- This function is re-exported at the ``finstack`` package root:
  ``from finstack import build_periods``
- Periods are generated as calendar months by default
- Use :func:`build_fiscal_periods` for fiscal year periods
- The ``actuals_until`` parameter is useful for separating historical
  data from projections in financial models

See Also
--------
:func:`build_fiscal_periods`: Build periods using fiscal year conventions
:class:`PeriodPlan`: Period collection container
:class:`Period`: Individual period with start/end dates
"""

def build_fiscal_periods(range: str, config: FiscalConfig, actuals_until: Optional[str] = None) -> PeriodPlan: ...

"""Build a sequence of fiscal periods from a date range expression.

Creates a :class:`PeriodPlan` containing fiscal periods based on a date
range and fiscal calendar configuration. Fiscal periods align with the
fiscal year start defined in the configuration, which may differ from
calendar months.

Parameters
----------
range : str
    Date range expression in the format "YYYY-MM-DD to YYYY-MM-DD" or
    "YYYY-MM-DD/YYYY-MM-DD". The range defines the start and end dates
    for period generation.
config : FiscalConfig
    Fiscal calendar configuration defining the fiscal year start (month
    and day). Use predefined configs like :attr:`FiscalConfig.US_FEDERAL`
    or create custom configs.
actuals_until : str, optional
    ISO-8601 date string marking the boundary between actual and forecast
    periods. Periods on or before this date are marked as actuals;
    periods after are forecasts. If ``None``, all periods are forecasts.

Returns
-------
PeriodPlan
    Collection of fiscal periods covering the specified date range.
    Periods align with the fiscal year defined in the configuration.

Raises
------
ValueError
    If the range string cannot be parsed, if dates are invalid, or if
    the fiscal configuration is invalid.

Examples
--------
Build US Federal fiscal periods (starts October 1):

    >>> from finstack.core.dates.periods import build_fiscal_periods, FiscalConfig
    >>> from datetime import date
    >>> config = FiscalConfig.US_FEDERAL
    >>> plan = build_fiscal_periods("2023-10-01 to 2024-09-30", config)
    >>> # First period starts October 1, 2023
    >>> print(plan.periods[0].start())
    2023-10-01

Build UK fiscal periods (starts April 6):

    >>> uk_config = FiscalConfig.UK
    >>> plan = build_fiscal_periods("2024-04-06 to 2025-04-05", uk_config)
    >>> print(plan.periods[0].start())
    2024-04-06

Custom fiscal year:

    >>> custom = FiscalConfig(7, 1)  # July 1 start
    >>> plan = build_fiscal_periods("2024-07-01 to 2025-06-30", custom)

Notes
-----
- This function is re-exported at the ``finstack`` package root:
  ``from finstack import build_fiscal_periods``
- Fiscal periods may span calendar month boundaries
- Use predefined :class:`FiscalConfig` constants for common jurisdictions
- Fiscal year periods are useful for government and corporate reporting
  that doesn't align with calendar years

See Also
--------
:func:`build_periods`: Build calendar year periods
:class:`FiscalConfig`: Fiscal calendar configuration
:class:`PeriodPlan`: Period collection container
"""
