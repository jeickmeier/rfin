"""FX instruments."""

from typing import Optional
from datetime import date
from ...core.money import Money
from ...core.currency import Currency
from ...core.dates.calendar import BusinessDayConvention
from ..common import InstrumentType

class FxSpot:
    """FX spot instrument exchanging base currency for quote currency."""

    @classmethod
    def create(
        cls,
        instrument_id: str,
        base_currency: Currency,
        quote_currency: Currency,
        *,
        settlement: Optional[date] = None,
        settlement_lag_days: Optional[int] = None,
        spot_rate: Optional[float] = None,
        notional: Optional[Money] = None,
        bdc: Optional[BusinessDayConvention] = None,
        calendar: Optional[str] = None,
    ) -> "FxSpot":
        """Create an FX spot position with optional settlement overrides."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def base_currency(self) -> Currency: ...
    @property
    def quote_currency(self) -> Currency: ...
    @property
    def notional(self) -> Optional[Money]: ...
    @property
    def spot_rate(self) -> Optional[float]: ...
    @property
    def settlement(self) -> Optional[date]: ...
    @property
    def settlement_lag_days(self) -> Optional[int]: ...
    @property
    def business_day_convention(self) -> str: ...
    @property
    def calendar_id(self) -> Optional[str]: ...
    @property
    def pair_name(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class FxOption:
    """Garman–Kohlhagen FX option with European exercise."""

    @classmethod
    def european_call(
        cls,
        instrument_id: str,
        base_currency: Currency,
        quote_currency: Currency,
        strike: float,
        expiry: date,
        notional: Money,
        vol_surface: str,
    ) -> "FxOption":
        """Create a European call option with explicit volatility surface."""
        ...

    @classmethod
    def european_put(
        cls,
        instrument_id: str,
        base_currency: Currency,
        quote_currency: Currency,
        strike: float,
        expiry: date,
        notional: Money,
        vol_surface: str,
    ) -> "FxOption":
        """Create a European put option with explicit volatility surface."""
        ...

    @classmethod
    def builder(
        cls,
        instrument_id: str,
        base_currency: Currency,
        quote_currency: Currency,
        strike: float,
        expiry: date,
        notional: Money,
        domestic_curve: str,
        foreign_curve: str,
        vol_surface: str,
        *,
        settlement: Optional[str] = "cash",
    ) -> "FxOption":
        """Create an FX option with explicit domestic/foreign curves and vol surface."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def base_currency(self) -> Currency: ...
    @property
    def quote_currency(self) -> Currency: ...
    @property
    def notional(self) -> Money: ...
    @property
    def strike(self) -> float: ...
    @property
    def expiry(self) -> date: ...
    @property
    def option_type(self) -> str: ...
    @property
    def exercise_style(self) -> str: ...
    @property
    def settlement(self) -> str: ...
    @property
    def domestic_curve(self) -> str: ...
    @property
    def foreign_curve(self) -> str: ...
    @property
    def vol_surface(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class FxSwap:
    """FX swap exchanging notionals on near and far legs."""

    @classmethod
    def create(
        cls,
        instrument_id: str,
        base_currency: Currency,
        quote_currency: Currency,
        notional: Money,
        near_date: date,
        far_date: date,
        domestic_curve: str,
        foreign_curve: str,
        *,
        near_rate: Optional[float] = None,
        far_rate: Optional[float] = None,
    ) -> "FxSwap":
        """Create an FX swap specifying near/far legs and associated curves."""
        ...

    @property
    def instrument_id(self) -> str: ...
    @property
    def base_currency(self) -> Currency: ...
    @property
    def quote_currency(self) -> Currency: ...
    @property
    def base_notional(self) -> Money: ...
    @property
    def near_date(self) -> date: ...
    @property
    def far_date(self) -> date: ...
    @property
    def near_rate(self) -> Optional[float]: ...
    @property
    def far_rate(self) -> Optional[float]: ...
    @property
    def domestic_curve(self) -> str: ...
    @property
    def foreign_curve(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
