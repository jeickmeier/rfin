"""Year-on-year inflation cap or floor instrument."""

from __future__ import annotations
from typing import Self

from datetime import date

from ....core.dates.calendar import BusinessDayConvention
from ....core.dates.daycount import DayCount
from ....core.dates.schedule import Frequency, StubKind
from ....core.money import Money
from ...common import InstrumentType

class InflationCapFloorType:
    """Inflation cap/floor option type."""

    CAP: InflationCapFloorType
    FLOOR: InflationCapFloorType
    CAPLET: InflationCapFloorType
    FLOORLET: InflationCapFloorType
    @classmethod
    def from_name(cls, name: str) -> InflationCapFloorType: ...
    @property
    def name(self) -> str: ...

class InflationCapFloorBuilder:
    """Fluent builder returned by :meth:`InflationCapFloor.builder`."""

    def __init__(self, instrument_id: str) -> None: ...
    def option_type(self, option_type: str | InflationCapFloorType) -> InflationCapFloorBuilder:
        """Set option type ('cap', 'floor', 'caplet', or 'floorlet')."""
        ...
    def notional(self, amount: float, currency: str) -> InflationCapFloorBuilder:
        """Set notional amount and currency."""
        ...
    def strike(self, rate: float) -> InflationCapFloorBuilder:
        """Set strike rate (annualized, decimal)."""
        ...
    def start_date(self, date: date) -> InflationCapFloorBuilder:
        """Set start date of the first inflation period."""
        ...
    def end_date(self, date: date) -> InflationCapFloorBuilder:
        """Set end date of the final inflation period."""
        ...
    def frequency(self, frequency: str | int) -> InflationCapFloorBuilder:
        """Set payment frequency (ignored for caplet/floorlet)."""
        ...
    def day_count(self, day_count: DayCount | str) -> InflationCapFloorBuilder:
        """Set day count convention for accrual and option time."""
        ...
    def bdc(self, bdc: BusinessDayConvention | str) -> InflationCapFloorBuilder:
        """Set business day convention for schedule and payments."""
        ...
    def stub(self, stub: StubKind | str) -> InflationCapFloorBuilder:
        """Set stub handling convention for irregular periods."""
        ...
    def calendar_id(self, calendar_id: str | None) -> InflationCapFloorBuilder:
        """Set optional holiday calendar identifier."""
        ...
    def inflation_index_id(self, curve_id: str) -> InflationCapFloorBuilder:
        """Set inflation index/curve identifier (e.g., 'US-CPI-U')."""
        ...
    def discount_curve(self, curve_id: str) -> InflationCapFloorBuilder:
        """Set discount curve identifier."""
        ...
    def vol_surface_id(self, curve_id: str) -> InflationCapFloorBuilder:
        """Set volatility surface identifier."""
        ...
    def lag_override(self, lag_override: str | None = ...) -> InflationCapFloorBuilder:
        """Set inflation lag override (e.g., '3M')."""
        ...
    def build(self) -> InflationCapFloor:
        """Build the InflationCapFloor instrument."""
        ...

class InflationCapFloor:
    """Year-on-year inflation cap or floor instrument.

    Prices YoY inflation caps/floors using Black-76 (lognormal) or Bachelier (normal)
    volatility models on the forward YoY inflation rate.

    Examples
    --------
    Create an inflation cap:

        >>> from finstack.valuations.instruments import InflationCapFloor
        >>> cap = (
        ...     InflationCapFloor
        ...     .builder("INFLATION_CAP_001")
        ...     .option_type("cap")
        ...     .notional(10_000_000, "USD")
        ...     .strike(0.025)
        ...     .start_date(date(2024, 1, 1))
        ...     .end_date(date(2029, 1, 1))
        ...     .frequency("annual")
        ...     .inflation_index_id("US-CPI-U")
        ...     .discount_curve("USD-OIS")
        ...     .vol_surface_id("USD-INFLATION-VOL")
        ...     .build()
        ... )

    See Also
    --------
    :class:`InflationSwap`: Vanilla inflation swap
    :class:`InterestRateOption`: Interest rate caps/floors
    """

    @classmethod
    def builder(cls, instrument_id: str) -> InflationCapFloorBuilder:
        """Create a builder for constructing InflationCapFloor instruments."""
        ...
    @property
    def id(self) -> str:
        """Instrument identifier."""
        ...
    @property
    def option_type(self) -> str:
        """Option type (cap, floor, caplet, or floorlet)."""
        ...
    @property
    def notional(self) -> Money:
        """Notional amount."""
        ...
    @property
    def strike(self) -> float:
        """Strike rate (annualized, decimal)."""
        ...
    @property
    def start_date(self) -> date:
        """Start date of the first inflation period."""
        ...
    @property
    def end_date(self) -> date:
        """End date of the final inflation period."""
        ...
    @property
    def inflation_index_id(self) -> str:
        """Inflation index identifier."""
        ...
    @property
    def discount_curve(self) -> str:
        """Discount curve identifier."""
        ...
    @property
    def vol_surface_id(self) -> str:
        """Volatility surface identifier."""
        ...
    @property
    def instrument_type(self) -> InstrumentType:
        """Instrument type enum."""
        ...

    def to_json(self) -> str:
        """Serialize to JSON in envelope format.

        Returns:
            str: JSON string with schema version and tagged instrument spec.
        """
        ...

    @classmethod
    def from_json(cls, json_str: str) -> "Self":
        """Deserialize from JSON in envelope format.

        Args:
            json_str: JSON string in envelope format.

        Returns:
            The deserialized instrument.

        Raises:
            ValueError: If JSON is malformed or contains a different instrument type.
        """
        ...

    def __repr__(self) -> str: ...
