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
    def from_months(cls, months: int) -> "Frequency": ...
    """Construct a frequency based on a number of calendar months."""

    @classmethod
    def from_days(cls, days: int) -> "Frequency": ...
    """Construct a frequency based on a number of days."""

    @classmethod
    def from_payments_per_year(cls, payments_per_year: int) -> "Frequency": ...
    """Construct a frequency from payments per year (e.g. 1, 2, 4, 12)."""

    @property
    def months(self) -> Optional[int]: ...
    """Month-based interval represented by this frequency, if any."""

    @property
    def days(self) -> Optional[int]: ...
    """Day-based interval represented by this frequency, if any."""

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...


class StubKind:
    """Stub period handling for irregular schedules.

    Available kinds:
    - NONE
    - SHORT_FRONT
    - SHORT_BACK
    - LONG_FRONT
    - LONG_BACK
    """

    @property
    def name(self) -> str: ...
    """Snake-case label describing the stub type."""

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...


class ScheduleBuilder:
    """Builder for payment schedules.

    Provides a fluent interface for constructing payment schedules
    with various frequency and adjustment options.
    """

    def __init__(self, start: Union[str, date], end: Union[str, date]) -> None: ...

    def frequency(self, frequency: Frequency) -> "ScheduleBuilder": ...

    def stub_rule(self, stub: StubKind) -> "ScheduleBuilder": ...

    def adjust_with(
        self, convention: BusinessDayConvention, calendar: Calendar
    ) -> "ScheduleBuilder": ...

    def end_of_month(self, enabled: bool) -> "ScheduleBuilder": ...

    def cds_imm(self) -> "ScheduleBuilder": ...

    def build(self) -> "Schedule": ...

    def __repr__(self) -> str: ...


class Schedule:
    """A payment schedule with dates and metadata."""

    @property
    def dates(self) -> List[date]: ...

    def __len__(self) -> int: ...

    def __repr__(self) -> str: ...


class ScheduleSpec:
    """Serializable specification describing how to build a schedule."""

    def __init__(
        self,
        start: Union[str, date],
        end: Union[str, date],
        frequency: Frequency,
        stub: Optional[StubKind] = ...,
        business_day_convention: Optional[BusinessDayConvention] = ...,
        calendar_id: Optional[str] = ...,
        end_of_month: bool = ...,
        cds_imm_mode: bool = ...,
        graceful: bool = ...,
    ) -> None: ...

    def build(self) -> Schedule: ...
    """Materialize the described schedule."""

    def to_json(self) -> str: ...
    """Serialize the spec to JSON."""

    @classmethod
    def from_json(cls, payload: str) -> "ScheduleSpec": ...
    """Deserialize from JSON."""

    @property
    def calendar_id(self) -> Optional[str]: ...

    @property
    def frequency(self) -> Frequency: ...

    @property
    def stub(self) -> StubKind: ...
