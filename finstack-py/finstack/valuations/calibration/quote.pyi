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


class RatesQuote:
    @classmethod
    def deposit(cls, maturity: Any, rate: float, day_count: Any) -> RatesQuote: ...

    @classmethod
    def fra(cls, start: Any, end: Any, rate: float, day_count: Any) -> RatesQuote: ...

    @classmethod
    def future(cls, expiry: Any, price: float, specs: FutureSpecs) -> RatesQuote: ...

    @classmethod
    def swap(
        cls,
        maturity: Any,
        rate: float,
        fixed_freq: Any,
        float_freq: Any,
        fixed_day_count: Any,
        float_day_count: Any,
        index: str,
    ) -> RatesQuote: ...

    @classmethod
    def basis_swap(
        cls,
        maturity: Any,
        primary_index: str,
        reference_index: str,
        spread_bp: float,
        primary_frequency: Any,
        reference_frequency: Any,
        primary_day_count: Any,
        reference_day_count: Any,
        currency: Any,
    ) -> RatesQuote: ...

    @property
    def kind(self) -> str: ...

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
    def yoy_inflation_swap(
        cls, maturity: Any, rate: float, index: str, frequency: Any
    ) -> InflationQuote: ...

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
