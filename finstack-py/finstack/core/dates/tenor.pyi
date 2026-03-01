"""Tenor parsing and calendar-aware conversions.

This module mirrors ``finstack_core::dates::tenor`` and provides:
- TenorUnit: Day/Week/Month/Year units
- Tenor: Parsed tenor with helpers for year fractions and calendar adjustment
"""

from __future__ import annotations
from datetime import date

from .calendar import BusinessDayConvention, Calendar
from .daycount import DayCount

class TenorUnit:
    """Unit of tenor length.

    Use class attributes (DAYS, WEEKS, MONTHS, YEARS) or construct from a
    single-character symbol via :meth:`from_symbol`.
    """

    DAYS: "TenorUnit"
    WEEKS: "TenorUnit"
    MONTHS: "TenorUnit"
    YEARS: "TenorUnit"

    @classmethod
    def from_symbol(cls, symbol: str) -> "TenorUnit":
        """Parse a single-letter unit code (D/W/M/Y)."""
        ...

    @property
    def name(self) -> str:
        """Lower-case unit label (``days``, ``weeks``, ``months``, ``years``)."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class Tenor:
    """Market-standard tenor with parsing and calendar-aware helpers.

    Tenors encode durations such as ``"3M"`` or ``"5Y"`` and can be added to
    dates with optional holiday calendar adjustment. They also expose
    year-fraction conversions using day-count conventions.

    Parameters
    ----------
    count : int
        Number of units.
    unit : TenorUnit
        Unit of the tenor (days/weeks/months/years).
    """

    @classmethod
    def parse(cls, text: str) -> "Tenor":
        """Parse tenor strings like ``"1D"``, ``"3M"``, ``"1Y"``.

        Raises
        ------
        ValueError
            If the tenor string is empty or uses an unknown unit.
        """
        ...

    @classmethod
    def from_years(cls, years: float, day_count: DayCount) -> "Tenor":
        """Construct a tenor that approximates a given year fraction."""
        ...

    @staticmethod
    def daily() -> "Tenor":
        """Create a 1-day tenor."""
        ...
    @staticmethod
    def weekly() -> "Tenor":
        """Create a 1-week tenor."""
        ...
    @staticmethod
    def biweekly() -> "Tenor":
        """Create a 2-week tenor."""
        ...
    @staticmethod
    def monthly() -> "Tenor":
        """Create a 1-month tenor."""
        ...
    @staticmethod
    def bimonthly() -> "Tenor":
        """Create a 2-month tenor."""
        ...
    @staticmethod
    def quarterly() -> "Tenor":
        """Create a 3-month tenor."""
        ...
    @staticmethod
    def semi_annual() -> "Tenor":
        """Create a 6-month tenor."""
        ...
    @staticmethod
    def annual() -> "Tenor":
        """Create a 1-year tenor."""
        ...
    @property
    def count(self) -> int:
        """Unit count (e.g., 3 for ``3M``)."""
        ...

    @property
    def unit(self) -> TenorUnit:
        """Tenor unit."""
        ...

    def to_years_simple(self) -> float:
        """Convert tenor to a simple year fraction (approximate)."""
        ...

    def add_to_date(
        self,
        date: date,
        *,
        calendar: Calendar | None = None,
        convention: BusinessDayConvention | None = None,
    ) -> date:
        """Add the tenor to a date, optionally applying a business-day convention."""
        ...

    def to_years_with_context(
        self,
        as_of: date,
        *,
        calendar: Calendar | None = None,
        convention: BusinessDayConvention | None = None,
        day_count: DayCount,
    ) -> float:
        """Compute year fraction using calendar adjustment and day-count."""
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...
    def __eq__(self, other: object) -> bool: ...
    def __ne__(self, other: object) -> bool: ...
