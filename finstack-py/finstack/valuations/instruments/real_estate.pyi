"""Real estate asset valuation instrument."""

from __future__ import annotations

from datetime import date
from typing import Any, Union

from ...core.currency import Currency
from ...core.dates.daycount import DayCount
from ...core.money import Money
from ..common import InstrumentType


class RealEstateAsset:
    """Real estate asset valuation instrument.

    Supports DCF (discounted cashflow with explicit NOI schedule) and
    direct capitalization valuation methods.

    Examples
    --------
    Direct cap valuation:

        >>> from finstack.valuations.instruments import RealEstateAsset
        >>> asset = RealEstateAsset.create_direct_cap(
        ...     "OFFICE-NYC-123",
        ...     currency="USD",
        ...     valuation_date=date(2024, 1, 1),
        ...     stabilized_noi=5_000_000.0,
        ...     cap_rate=0.06,
        ...     discount_curve_id="USD-OIS",
        ... )

    DCF valuation:

        >>> noi_schedule = [
        ...     (date(2024, 12, 31), 4_500_000.0),
        ...     (date(2025, 12, 31), 4_800_000.0),
        ... ]
        >>> asset = RealEstateAsset.create_dcf(
        ...     "OFFICE-NYC-123",
        ...     currency="USD",
        ...     valuation_date=date(2024, 1, 1),
        ...     noi_schedule=noi_schedule,
        ...     discount_rate=0.08,
        ...     terminal_cap_rate=0.065,
        ...     discount_curve_id="USD-OIS",
        ... )
    """

    @classmethod
    def create_dcf(
        cls,
        instrument_id: str,
        *,
        currency: str | Currency,
        valuation_date: date,
        noi_schedule: list[tuple[date, float]],
        discount_rate: float,
        discount_curve_id: str,
        terminal_cap_rate: float | None = None,
        terminal_growth_rate: float | None = None,
        capex_schedule: list[tuple[date, float]] | None = None,
        sale_date: date | None = None,
        sale_price: Money | None = None,
        acquisition_cost: float | None = None,
        acquisition_costs: list[Money] | None = None,
        disposition_cost_pct: float | None = None,
        disposition_costs: list[Money] | None = None,
        purchase_price: Money | None = None,
        property_type: str | None = None,
        day_count: Union[DayCount, str, None] = None,
        appraisal_value: Money | None = None,
    ) -> RealEstateAsset:
        """Create a real estate asset with DCF valuation method."""
        ...

    @classmethod
    def create_direct_cap(
        cls,
        instrument_id: str,
        *,
        currency: str | Currency,
        valuation_date: date,
        stabilized_noi: float,
        cap_rate: float,
        discount_curve_id: str,
        noi_schedule: list[tuple[date, float]] | None = None,
        capex_schedule: list[tuple[date, float]] | None = None,
        acquisition_cost: float | None = None,
        disposition_cost_pct: float | None = None,
        purchase_price: Money | None = None,
        property_type: str | None = None,
        day_count: Union[DayCount, str, None] = None,
        appraisal_value: Money | None = None,
    ) -> RealEstateAsset:
        """Create a real estate asset with direct capitalization valuation method."""
        ...

    @property
    def instrument_id(self) -> str:
        """Instrument identifier."""
        ...
    @property
    def currency(self) -> Currency:
        """Currency."""
        ...
    @property
    def valuation_date(self) -> date:
        """Valuation date."""
        ...
    @property
    def valuation_method(self) -> str:
        """Valuation method ('dcf' or 'direct_cap')."""
        ...
    @property
    def noi_schedule(self) -> list[tuple[date, float]]:
        """NOI schedule as list of (date, amount) tuples."""
        ...
    @property
    def discount_rate(self) -> float | None:
        """Optional discount rate (for DCF)."""
        ...
    @property
    def cap_rate(self) -> float | None:
        """Optional capitalization rate (for direct cap)."""
        ...
    @property
    def stabilized_noi(self) -> float | None:
        """Optional stabilized NOI (for direct cap)."""
        ...
    @property
    def terminal_cap_rate(self) -> float | None:
        """Optional terminal capitalization rate (for DCF)."""
        ...
    @property
    def terminal_growth_rate(self) -> float | None:
        """Optional terminal growth rate (for DCF exit valuation)."""
        ...
    @property
    def acquisition_cost(self) -> float | None:
        """Optional acquisition cost (transaction cost)."""
        ...
    @property
    def disposition_cost_pct(self) -> float | None:
        """Optional disposition cost percentage."""
        ...
    @property
    def purchase_price(self) -> Money | None:
        """Optional purchase price."""
        ...
    @property
    def property_type(self) -> str | None:
        """Optional property type classification."""
        ...
    @property
    def appraisal_value(self) -> Money | None:
        """Optional appraisal value override."""
        ...
    @property
    def day_count(self) -> DayCount:
        """Day count convention."""
        ...
    @property
    def discount_curve_id(self) -> str:
        """Discount curve ID."""
        ...
    @property
    def instrument_type(self) -> InstrumentType:
        """Instrument type key."""
        ...
    def __repr__(self) -> str: ...
