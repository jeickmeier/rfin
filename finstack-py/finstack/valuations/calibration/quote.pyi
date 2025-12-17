"""Market quote types exposed by :mod:`finstack.valuations.calibration`."""

from __future__ import annotations

from typing import Any, Optional

class FutureSpecs:
    def __init__(
        self,
        *,
        multiplier: float = 1_000_000.0,
        face_value: float = 1_000_000.0,
        delivery_months: int = 3,
        day_count: Any = "ACT/360",
        convexity_adjustment: Optional[float] = None,
    ) -> None: ...
    @property
    def multiplier(self) -> float: ...
    @property
    def face_value(self) -> float: ...
    @property
    def delivery_months(self) -> int: ...
    @property
    def day_count(self) -> Any: ...
    @property
    def convexity_adjustment(self) -> Optional[float]: ...
    def __repr__(self) -> str: ...

class InstrumentConventions:
    def __init__(
        self,
        settlement_days: int | None = ...,
        payment_delay_days: int | None = ...,
        reset_lag: int | None = ...,
        calendar_id: str | None = ...,
        fixing_calendar_id: str | None = ...,
        payment_calendar_id: str | None = ...,
        reset_frequency: Any | None = ...,
        payment_frequency: Any | None = ...,
        business_day_convention: Any | None = ...,
        day_count: Any | None = ...,
        currency: Any | None = ...,
        index: Any | None = ...,
        recovery_rate: float | None = ...,
    ) -> None: ...
    @property
    def settlement_days(self) -> int | None: ...
    @property
    def payment_delay_days(self) -> int | None: ...
    @property
    def reset_lag(self) -> int | None: ...
    @property
    def calendar_id(self) -> str | None: ...
    @property
    def fixing_calendar_id(self) -> str | None: ...
    @property
    def payment_calendar_id(self) -> str | None: ...
    @property
    def reset_frequency(self) -> Any | None: ...
    @property
    def payment_frequency(self) -> Any | None: ...
    @property
    def business_day_convention(self) -> Any | None: ...
    @property
    def day_count(self) -> Any | None: ...
    @property
    def currency(self) -> Any | None: ...
    @property
    def index(self) -> str | None: ...
    @property
    def recovery_rate(self) -> float | None: ...
    def __repr__(self) -> str: ...

class RatesQuote:
    @classmethod
    def deposit(
        cls,
        maturity: Any,
        rate: float,
        *,
        conventions: InstrumentConventions,
    ) -> RatesQuote: ...
    @classmethod
    def fra(
        cls,
        start: Any,
        end: Any,
        rate: float,
        *,
        conventions: InstrumentConventions,
    ) -> RatesQuote: ...
    @classmethod
    def future(
        cls,
        expiry: Any,
        price: float,
        specs: FutureSpecs,
        *,
        fixing_date: Any | None = ...,
        conventions: InstrumentConventions | None = ...,
    ) -> RatesQuote: ...
    @classmethod
    def swap(
        cls,
        maturity: Any,
        rate: float,
        *,
        fixed_leg_conventions: InstrumentConventions,
        float_leg_conventions: InstrumentConventions,
        is_ois: bool = ...,
        conventions: InstrumentConventions | None = ...,
    ) -> RatesQuote: ...
    @classmethod
    def basis_swap(
        cls,
        maturity: Any,
        spread_bp: float,
        *,
        primary_leg_conventions: InstrumentConventions,
        reference_leg_conventions: InstrumentConventions,
        conventions: InstrumentConventions,
    ) -> RatesQuote: ...
    @property
    def kind(self) -> str: ...
    @property
    def conventions(self) -> InstrumentConventions: ...
    @property
    def is_ois(self) -> bool: ...
    @property
    def fixed_leg_conventions(self) -> InstrumentConventions | None: ...
    @property
    def float_leg_conventions(self) -> InstrumentConventions | None: ...
    @property
    def primary_leg_conventions(self) -> InstrumentConventions | None: ...
    @property
    def reference_leg_conventions(self) -> InstrumentConventions | None: ...
    def to_market_quote(self) -> MarketQuote: ...
    def __repr__(self) -> str: ...

class CreditQuote:
    @classmethod
    def cds(
        cls,
        entity: str,
        maturity: Any,
        spread_bp: float,
        recovery_rate: float,
        currency: Any,
    ) -> CreditQuote: ...
    @classmethod
    def cds_upfront(
        cls,
        entity: str,
        maturity: Any,
        upfront_pct: float,
        running_spread_bp: float,
        recovery_rate: float,
        currency: Any,
    ) -> CreditQuote: ...
    @classmethod
    def cds_tranche(
        cls,
        index: str,
        attachment: float,
        detachment: float,
        maturity: Any,
        upfront_pct: float,
        running_spread_bp: float,
    ) -> CreditQuote: ...
    @property
    def kind(self) -> str: ...
    def to_market_quote(self) -> MarketQuote: ...
    def __repr__(self) -> str: ...

class VolQuote:
    @classmethod
    def option_vol(
        cls,
        underlying: str,
        expiry: Any,
        strike: float,
        vol: float,
        option_type: str,
    ) -> VolQuote: ...
    @classmethod
    def swaption_vol(
        cls,
        expiry: Any,
        tenor: Any,
        strike: float,
        vol: float,
        quote_type: str,
    ) -> VolQuote: ...
    @property
    def kind(self) -> str: ...
    def to_market_quote(self) -> MarketQuote: ...
    def __repr__(self) -> str: ...

class InflationQuote:
    @classmethod
    def inflation_swap(cls, maturity: Any, rate: float, index: str) -> InflationQuote: ...
    @classmethod
    def yoy_inflation_swap(cls, maturity: Any, rate: float, index: str, frequency: Any) -> InflationQuote: ...
    @property
    def kind(self) -> str: ...
    def to_market_quote(self) -> MarketQuote: ...
    def __repr__(self) -> str: ...

class MarketQuote:
    @classmethod
    def from_rates(cls, quote: RatesQuote) -> MarketQuote: ...
    @classmethod
    def from_credit(cls, quote: CreditQuote) -> MarketQuote: ...
    @classmethod
    def from_vol(cls, quote: VolQuote) -> MarketQuote: ...
    @classmethod
    def from_inflation(cls, quote: InflationQuote) -> MarketQuote: ...
    @property
    def kind(self) -> str: ...
    def __repr__(self) -> str: ...
