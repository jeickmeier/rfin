"""Commodity Asian option instrument."""

from __future__ import annotations

from datetime import date
from typing import List, Self, Tuple

from ....core.currency import Currency
from ....core.dates.daycount import DayCount
from ...common import InstrumentType

class CommodityAsianOptionBuilder:
    """Fluent builder returned by :meth:`CommodityAsianOption.builder`."""

    def commodity_type(self, commodity_type: str) -> CommodityAsianOptionBuilder: ...
    def ticker(self, ticker: str) -> CommodityAsianOptionBuilder: ...
    def unit(self, unit: str) -> CommodityAsianOptionBuilder: ...
    def currency(self, currency: str | Currency) -> CommodityAsianOptionBuilder: ...
    def strike(self, strike: float) -> CommodityAsianOptionBuilder: ...
    def option_type(self, option_type: str) -> CommodityAsianOptionBuilder: ...
    def averaging_method(self, method: str) -> CommodityAsianOptionBuilder: ...
    def fixing_dates(self, dates: List[date]) -> CommodityAsianOptionBuilder: ...
    def realized_fixings(self, fixings: List[Tuple[date, float]]) -> CommodityAsianOptionBuilder: ...
    def quantity(self, quantity: float) -> CommodityAsianOptionBuilder: ...
    def expiry(self, expiry: date) -> CommodityAsianOptionBuilder: ...
    def forward_curve_id(self, curve_id: str) -> CommodityAsianOptionBuilder: ...
    def discount_curve_id(self, curve_id: str) -> CommodityAsianOptionBuilder: ...
    def vol_surface_id(self, surface_id: str) -> CommodityAsianOptionBuilder: ...
    def day_count(self, day_count: DayCount | str) -> CommodityAsianOptionBuilder: ...
    def build(self) -> CommodityAsianOption: ...
    def __repr__(self) -> str: ...

class CommodityAsianOption:
    """Commodity Asian option: option on the average of commodity prices.

    This is the dominant option type in commodity markets. The average is
    typically computed over commodity forward/futures prices for specific
    delivery periods.

    Key differences from equity Asian options:
    - Uses forward prices from a price curve for each fixing date, not spot
    - No dividend yield parameter (cost of carry is in the forward curve)
    - Seasoned options combine realized fixings with projected forwards

    Examples
    --------
    Create a WTI arithmetic average call option:

        >>> from finstack.valuations.instruments import CommodityAsianOption
        >>> from datetime import date
        >>> option = (
        ...     CommodityAsianOption
        ...     .builder("WTI-ASIAN-6M")
        ...     .commodity_type("Energy")
        ...     .ticker("CL")
        ...     .unit("BBL")
        ...     .currency("USD")
        ...     .strike(75.0)
        ...     .option_type("call")
        ...     .averaging_method("arithmetic")
        ...     .fixing_dates([date(2025, 1, 31), date(2025, 2, 28)])
        ...     .quantity(1000.0)
        ...     .expiry(date(2025, 7, 2))
        ...     .forward_curve_id("CL-FORWARD")
        ...     .discount_curve_id("USD-OIS")
        ...     .vol_surface_id("CL-VOL")
        ...     .build()
        ... )

    See Also
    --------
    :class:`CommodityOption`: Standard commodity options
    :class:`CommodityForward`: Commodity forward contracts
    """

    @classmethod
    def builder(cls, instrument_id: str) -> CommodityAsianOptionBuilder:
        """Start a fluent builder for a commodity Asian option."""
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

    @property
    def instrument_id(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    @property
    def commodity_type(self) -> str: ...
    @property
    def ticker(self) -> str: ...
    @property
    def unit(self) -> str: ...
    @property
    def currency(self) -> Currency: ...
    @property
    def strike(self) -> float: ...
    @property
    def option_type(self) -> str: ...
    @property
    def averaging_method(self) -> str: ...
    @property
    def fixing_dates(self) -> List[date]: ...
    @property
    def realized_fixings(self) -> List[Tuple[date, float]]: ...
    @property
    def quantity(self) -> float: ...
    @property
    def expiry(self) -> date: ...
    @property
    def forward_curve_id(self) -> str: ...
    @property
    def discount_curve_id(self) -> str: ...
    @property
    def vol_surface_id(self) -> str: ...
    @property
    def day_count(self) -> DayCount: ...
    def accumulated_state(self, as_of: date) -> Tuple[float, float, int]: ...
    def __repr__(self) -> str: ...
