"""Day count convention bindings.

Provides various day count conventions for calculating year fractions
between dates, essential for interest calculations.
"""

from typing import Optional, Union
from datetime import date
from .calendar import Calendar
from .schedule import Frequency

class DayCount:
    """Day count convention for year fraction calculations.

    Available conventions:
    - ACT_360: Actual days over 360
    - ACT_365: Actual days over 365
    - ACT_ACT: Actual days over actual days in year
    - THIRTY_360: 30/360 convention
    - THIRTY_360_EU: European 30/360
    - THIRTY_360_ISDA: ISDA 30/360
    """

    @classmethod
    def from_name(cls, name: str) -> DayCount:
        """Create from string name.

        Parameters
        ----------
        name : str
            Day count name (case-insensitive).

        Returns
        -------
        DayCount
            Day count instance.
        """
        ...

    @property
    def name(self) -> str:
        """Get the day count name.

        Returns
        -------
        str
            Human-readable day count name.
        """
        ...

    def year_fraction(
        self, start: Union[str, date], end: Union[str, date], ctx: Optional["DayCountContext"] = None
    ) -> float:
        """Calculate year fraction between two dates.

        Parameters
        ----------
        start : str or date
            Start date.
        end : str or date
            End date.
        ctx : DayCountContext, optional
            Context with calendar/frequency hints.

        Returns
        -------
        float
            Year fraction between the dates.
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

# Day count constants
ACT_360: DayCount
ACT_365: DayCount
ACT_ACT: DayCount
THIRTY_360: DayCount
THIRTY_360_EU: DayCount
THIRTY_360_ISDA: DayCount

class DayCountContext:
    """Context for day count calculations.

    Provides optional calendar and frequency hints that may affect
    day count calculations for certain conventions.
    """

    def __init__(self, calendar: Optional[Calendar] = None, frequency: Optional["Frequency"] = None) -> None:
        """Create a context with optional calendar/frequency hints.

        Parameters
        ----------
        calendar : Calendar, optional
            Holiday calendar for business day adjustments.
        frequency : Frequency, optional
            Payment frequency hint.
        """
        ...

    @property
    def calendar(self) -> Optional[Calendar]:
        """Get the calendar hint.

        Returns
        -------
        Calendar or None
            Calendar hint if set.
        """
        ...

    def set_calendar(self, calendar: Optional[Calendar]) -> None:
        """Set the calendar hint.

        Parameters
        ----------
        calendar : Calendar or None
            New calendar hint.
        """
        ...

    @property
    def frequency(self) -> Optional["Frequency"]:
        """Get the frequency hint.

        Returns
        -------
        Frequency or None
            Frequency hint if set.
        """
        ...

    def set_frequency(self, frequency: Optional["Frequency"]) -> None:
        """Set the frequency hint.

        Parameters
        ----------
        frequency : Frequency or None
            New frequency hint.
        """
        ...

    def to_state(self) -> "DayCountContextState":
        """Convert the context into a serializable DTO."""
        ...

    def __repr__(self) -> str: ...

class DayCountContextState:
    """Serializable representation of :class:`DayCountContext`."""

    def __init__(
        self,
        calendar_id: Optional[str] = ...,
        frequency: Optional["Frequency"] = ...,
        bus_basis: Optional[int] = ...,
    ) -> None: ...
    @classmethod
    def from_context(cls, ctx: DayCountContext) -> DayCountContextState:
        """Create a DTO from a runtime context."""
        ...

    def to_context(self) -> DayCountContext:
        """Rehydrate the DTO into a runtime context."""
        ...

    def to_json(self) -> str:
        """Serialize the DTO to JSON."""
        ...

    @classmethod
    def from_json(cls, payload: str) -> DayCountContextState:
        """Deserialize a DTO from JSON."""
        ...

    @property
    def calendar_id(self) -> Optional[str]: ...
    @property
    def frequency(self) -> Optional["Frequency"]: ...
    @property
    def bus_basis(self) -> Optional[int]: ...

class Thirty360Convention:
    """30/360 convention variant.

    Available variants:
    - US: US 30/360
    - EU: European 30/360
    - ISDA: ISDA 30/360
    """

    @classmethod
    def from_name(cls, name: str) -> Thirty360Convention:
        """Create from string name.

        Parameters
        ----------
        name : str
            Convention name (case-insensitive).

        Returns
        -------
        Thirty360Convention
            Convention instance.
        """
        ...

    @property
    def name(self) -> str:
        """Get the convention name.

        Returns
        -------
        str
            Human-readable convention name.
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

# 30/360 convention constants
US: Thirty360Convention
EU: Thirty360Convention
ISDA: Thirty360Convention
