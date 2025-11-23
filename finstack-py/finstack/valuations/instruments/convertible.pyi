"""Convertible bond instrument."""

from typing import Optional, List, Tuple, Union
from datetime import date
from ...core.money import Money
from ..common import InstrumentType
from ..cashflow.builder import FixedCouponSpec, FloatingCouponSpec

class ConversionEvent:
    """Convertible conversion event wrapper."""

    QUALIFIED_IPO: "ConversionEvent"
    CHANGE_OF_CONTROL: "ConversionEvent"
    @classmethod
    def price_trigger(cls, threshold: float, lookback_days: int) -> "ConversionEvent": ...

class ConversionPolicy:
    """Convertible conversion policy wrapper."""
    @classmethod
    def voluntary(cls) -> "ConversionPolicy": ...
    @classmethod
    def mandatory_on(cls, conversion_date: date) -> "ConversionPolicy": ...
    @classmethod
    def window(cls, start: date, end: date) -> "ConversionPolicy": ...
    @classmethod
    def upon_event(cls, event: ConversionEvent) -> "ConversionPolicy": ...

class AntiDilutionPolicy:
    """Anti-dilution policy wrapper."""

    NONE: "AntiDilutionPolicy"
    FULL_RATCHET: "AntiDilutionPolicy"
    WEIGHTED_AVERAGE: "AntiDilutionPolicy"

class DividendAdjustment:
    """Dividend adjustment policy wrapper."""

    NONE: "DividendAdjustment"
    ADJUST_PRICE: "DividendAdjustment"
    ADJUST_RATIO: "DividendAdjustment"

class ConversionSpec:
    """Convertible conversion specification."""
    @classmethod
    def create(
        cls,
        policy: ConversionPolicy,
        *,
        ratio: Optional[float] = None,
        price: Optional[float] = None,
        anti_dilution: Optional[AntiDilutionPolicy] = None,
        dividend_adjustment: Optional[DividendAdjustment] = None,
    ) -> "ConversionSpec": ...
    @property
    def ratio(self) -> Optional[float]: ...
    @property
    def price(self) -> Optional[float]: ...
    @property
    def policy(self) -> str: ...

class ConvertibleBond:
    """Convertible bond wrapper."""

    @classmethod
    def create(
        cls,
        instrument_id: str,
        notional: Money,
        issue: date,
        maturity: date,
        discount_curve: str,
        conversion: ConversionSpec,
        *,
        underlying_equity_id: Optional[str] = None,
        call_schedule: Optional[List[Tuple[date, float]]] = None,
        put_schedule: Optional[List[Tuple[date, float]]] = None,
        fixed_coupon: Optional[FixedCouponSpec] = None,
        floating_coupon: Optional[FloatingCouponSpec] = None,
    ) -> "ConvertibleBond":
        """Create a convertible bond."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def notional(self) -> Money: ...
    @property
    def discount_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    @property
    def conversion_ratio(self) -> Optional[float]: ...
    @property
    def conversion_price(self) -> Optional[float]: ...
    @property
    def conversion_policy(self) -> str: ...
    @property
    def issue(self) -> date: ...
    @property
    def maturity(self) -> date: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
