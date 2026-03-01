"""Period and fiscal calendar bindings.

Provides period identification and fiscal calendar management
for financial modeling and reporting.
"""

from __future__ import annotations
from typing import List
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
    """Period identifier for days, quarters, months, weeks, halves, or years."""

    @classmethod
    def day(cls, year: int, ordinal: int) -> "PeriodId": ...
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
    def periods_per_year(self) -> int:
        """Number of periods per year implied by the period kind."""
        ...

    def next(self) -> "PeriodId":
        """Next period in sequence."""
        ...

    def prev(self) -> "PeriodId":
        """Previous period in sequence."""
        ...

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

def build_periods(range: str, actuals_until: str | None = None) -> PeriodPlan:
    """Build a sequence of calendar periods from a period-range expression.

    Creates a :class:`PeriodPlan` containing monthly, quarterly, or annual
    periods based on a date range string. This is the primary function for
    generating period structures for financial statement modeling and reporting.

    Parameters
    ----------
    range : str
        Period range expression, e.g. ``"2024Q1..Q4"`` or ``"2023M1..2023M12"``.
    actuals_until : str, optional
        Optional cutoff code; periods up to and including this code are marked
        actual (e.g. ``"2024Q2"``).

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
    Build quarterly periods and mark actuals vs forecasts:

        >>> from finstack.core.dates.periods import build_periods
        >>> plan = build_periods("2024Q1..Q4", actuals_until="2024Q2")
        >>> [p.id.code for p in plan.periods]
        ['2024Q1', '2024Q2', '2024Q3', '2024Q4']
        >>> [p.is_actual for p in plan.periods]
        [True, True, False, False]

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
    ...

def build_fiscal_periods(range: str, config: FiscalConfig, actuals_until: str | None = None) -> PeriodPlan:
    """Build a sequence of fiscal periods from a period-range expression.

    Creates a :class:`PeriodPlan` containing fiscal periods based on a date
    range and fiscal calendar configuration. Fiscal periods align with the
    fiscal year start defined in the configuration, which may differ from
    calendar months.

    Parameters
    ----------
    range : str
        Period range expression, e.g. ``"2025Q1..Q4"``. Fiscal interpretation is
        controlled by ``config``.
    config : FiscalConfig
        Fiscal calendar configuration defining the fiscal year start (month
        and day). Use predefined configs like :attr:`FiscalConfig.US_FEDERAL`
        or create custom configs.
    actuals_until : str, optional
        Optional cutoff code; periods up to and including this code are marked actual.

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
        >>> from finstack.core.dates.periods import FiscalConfig, build_fiscal_periods
        >>> plan = build_fiscal_periods("2025Q1..Q4", FiscalConfig.US_FEDERAL, actuals_until="2025Q2")
        >>> [p.id.code for p in plan.periods]
        ['2025Q1', '2025Q2', '2025Q3', '2025Q4']

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
    ...
