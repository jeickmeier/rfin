"""Market quote types exposed by :mod:`finstack.valuations.calibration`."""

from __future__ import annotations

from typing import Any

class RatesQuote:
    @classmethod
    def deposit(
        cls,
        id: str,
        index: Any,
        maturity: Any,
        rate: float,
    ) -> RatesQuote: ...
    @classmethod
    def fra(
        cls,
        id: str,
        index: Any,
        start: Any,
        end: Any,
        rate: float,
    ) -> RatesQuote: ...
    @classmethod
    def future(
        cls,
        id: str,
        expiry: Any,
        price: float,
        *,
        contract: str | None = ...,
        convexity_adjustment: float | None = ...,
        vol_surface_id: str | None = ...,
    ) -> RatesQuote: ...
    @classmethod
    def swap(
        cls,
        id: str,
        index: Any,
        maturity: Any,
        rate: float,
        *,
        spread: float | None = ...,
    ) -> RatesQuote: ...
    @property
    def kind(self) -> str: ...
    def to_market_quote(self) -> MarketQuote: ...
    def __repr__(self) -> str: ...

class CreditQuote:
    @classmethod
    def cds_par_spread(
        cls,
        id: str,
        entity: str,
        pillar: Any,
        spread_bp: float,
        recovery_rate: float,
        currency: Any,
        doc_clause: str,
    ) -> CreditQuote: ...
    @classmethod
    def cds_tranche(
        cls,
        id: str,
        index: str,
        attachment: float,
        detachment: float,
        maturity: Any,
        upfront_pct: float,
        running_spread_bp: float,
        currency: Any,
        doc_clause: str,
    ) -> CreditQuote: ...
    @classmethod
    def cds_upfront(
        cls,
        id: str,
        entity: str,
        pillar: Any,
        upfront_pct: float,
        running_spread_bp: float,
        recovery_rate: float,
        currency: Any,
        doc_clause: str,
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
        convention: str,
    ) -> VolQuote: ...
    @classmethod
    def swaption_vol(
        cls,
        expiry: Any,
        tenor: Any,
        strike: float,
        vol: float,
        quote_type: str,
        convention: str,
    ) -> VolQuote: ...
    @property
    def kind(self) -> str: ...
    def to_market_quote(self) -> MarketQuote: ...
    def __repr__(self) -> str: ...

class InflationQuote:
    @classmethod
    def inflation_swap(cls, maturity: Any, rate: float, index: str, convention: str) -> InflationQuote: ...
    @classmethod
    def yoy_inflation_swap(
        cls,
        maturity: Any,
        rate: float,
        index: str,
        frequency: Any,
        convention: str,
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
