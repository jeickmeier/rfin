"""Market builders (quotes + BuildCtx) type stubs."""

from __future__ import annotations

from typing import Any

from finstack.core.currency import Currency
from finstack.core.market_data.context import MarketContext
from finstack.valuations.common import InstrumentType
from finstack.valuations.common.parameters import OptionType
from finstack.valuations.conventions import CdsConventionKey

class QuoteId:
    def __init__(self, id: str) -> None: ...
    @property
    def value(self) -> str: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class Pillar:
    @classmethod
    def tenor(cls, tenor: Any) -> Pillar: ...
    @classmethod
    def date(cls, date: Any) -> Pillar: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class BuildCtx:
    def __init__(self, as_of: Any, notional: float, *, curve_ids: dict[str, str] | None = ...) -> None: ...
    @property
    def as_of(self) -> Any: ...
    @property
    def notional(self) -> float: ...
    def curve_id(self, role: str) -> str | None: ...

class BuiltInstrument:
    @property
    def id(self) -> str: ...
    @property
    def instrument_type(self) -> InstrumentType: ...

class RateQuote:
    @classmethod
    def deposit(cls, id: QuoteId | str, index: Any, pillar: Any, rate: float) -> RateQuote: ...
    @classmethod
    def fra(cls, id: QuoteId | str, index: Any, start: Any, end: Any, rate: float) -> RateQuote: ...
    @classmethod
    def future(
        cls,
        id: QuoteId | str,
        expiry: Any,
        price: float,
        *,
        contract: str | None = ...,
        convexity_adjustment: float | None = ...,
        vol_surface_id: str | None = ...,
    ) -> RateQuote: ...
    @classmethod
    def swap(
        cls,
        id: QuoteId | str,
        index: Any,
        pillar: Any,
        rate: float,
        *,
        spread_decimal: float | None = ...,
    ) -> RateQuote: ...
    @property
    def id(self) -> QuoteId: ...
    def __repr__(self) -> str: ...

class CdsQuote:
    @classmethod
    def par_spread(
        cls,
        id: QuoteId | str,
        entity: str,
        convention: CdsConventionKey,
        pillar: Any,
        spread_bp: float,
        *,
        recovery_rate: float = ...,
    ) -> CdsQuote: ...
    @classmethod
    def upfront(
        cls,
        id: QuoteId | str,
        entity: str,
        convention: CdsConventionKey,
        pillar: Any,
        running_spread_bp: float,
        upfront_pct: float,
        *,
        recovery_rate: float = ...,
    ) -> CdsQuote: ...
    @property
    def id(self) -> QuoteId: ...
    def __repr__(self) -> str: ...

class CdsTrancheQuote:
    @classmethod
    def cds_tranche(
        cls,
        id: QuoteId | str,
        index: str,
        attachment: float,
        detachment: float,
        maturity: Any,
        upfront_pct: float,
        running_spread_bp: float,
        convention: CdsConventionKey,
    ) -> CdsTrancheQuote: ...
    @property
    def id(self) -> QuoteId: ...
    def __repr__(self) -> str: ...

class CDSTrancheBuildOverrides:
    def __init__(
        self,
        series: int,
        *,
        payment_frequency: Any | None = ...,
        day_count: Any | None = ...,
        business_day_convention: Any | None = ...,
        calendar_id: str | None = ...,
        use_imm_dates: bool = ...,
    ) -> None: ...
    @property
    def series(self) -> int: ...
    @property
    def use_imm_dates(self) -> bool: ...
    def __repr__(self) -> str: ...

class BondQuote:
    @classmethod
    def fixed_rate_bullet_clean_price(
        cls,
        id: QuoteId | str,
        currency: Currency | str,
        issue_date: Any,
        maturity: Any,
        coupon_rate: float,
        convention: str,
        clean_price_pct: float,
    ) -> BondQuote: ...
    @classmethod
    def fixed_rate_bullet_z_spread(
        cls,
        id: QuoteId | str,
        currency: Currency | str,
        issue_date: Any,
        maturity: Any,
        coupon_rate: float,
        convention: str,
        z_spread: float,
    ) -> BondQuote: ...
    @classmethod
    def fixed_rate_bullet_oas(
        cls,
        id: QuoteId | str,
        currency: Currency | str,
        issue_date: Any,
        maturity: Any,
        coupon_rate: float,
        convention: str,
        oas: float,
    ) -> BondQuote: ...
    @classmethod
    def fixed_rate_bullet_ytm(
        cls,
        id: QuoteId | str,
        currency: Currency | str,
        issue_date: Any,
        maturity: Any,
        coupon_rate: float,
        convention: str,
        ytm: float,
    ) -> BondQuote: ...
    @property
    def id(self) -> QuoteId: ...
    def __repr__(self) -> str: ...

class InflationQuote:
    @classmethod
    def inflation_swap(
        cls,
        maturity: Any,
        rate: float,
        index: str,
        convention: str,
    ) -> InflationQuote: ...
    @classmethod
    def yoy_inflation_swap(
        cls,
        maturity: Any,
        rate: float,
        index: str,
        frequency: Any,
        convention: str,
    ) -> InflationQuote: ...
    def __repr__(self) -> str: ...

class VolQuote:
    @classmethod
    def option_vol(
        cls,
        underlying: str,
        expiry: Any,
        strike: float,
        vol: float,
        option_type: OptionType | str,
        convention: str,
    ) -> VolQuote: ...
    @classmethod
    def swaption_vol(
        cls,
        expiry: Any,
        maturity: Any,
        strike: float,
        vol: float,
        quote_type: str,
        convention: str,
    ) -> VolQuote: ...
    def __repr__(self) -> str: ...

class FxQuote:
    @classmethod
    def forward_outright(
        cls,
        id: QuoteId | str,
        convention: str,
        pillar: Any,
        forward_rate: float,
    ) -> FxQuote: ...
    @classmethod
    def swap_outright(
        cls,
        id: QuoteId | str,
        convention: str,
        far_pillar: Any,
        near_rate: float,
        far_rate: float,
    ) -> FxQuote: ...
    @classmethod
    def option_vanilla(
        cls,
        id: QuoteId | str,
        convention: str,
        expiry: Any,
        strike: float,
        option_type: OptionType | str,
        vol_surface_id: str,
    ) -> FxQuote: ...
    @property
    def id(self) -> QuoteId: ...
    def __repr__(self) -> str: ...

class XccyQuote:
    @classmethod
    def basis_swap(
        cls,
        id: QuoteId | str,
        convention: str,
        far_pillar: Any,
        basis_spread_bp: float,
        *,
        spot_fx: float | None = ...,
    ) -> XccyQuote: ...
    @property
    def id(self) -> QuoteId: ...
    def __repr__(self) -> str: ...

class MarketQuote:
    @classmethod
    def from_rate(cls, quote: RateQuote) -> MarketQuote: ...
    @classmethod
    def from_cds(cls, quote: CdsQuote) -> MarketQuote: ...
    @classmethod
    def from_cds_tranche(cls, quote: CdsTrancheQuote) -> MarketQuote: ...
    @classmethod
    def from_bond(cls, quote: BondQuote) -> MarketQuote: ...
    @classmethod
    def from_inflation(cls, quote: InflationQuote) -> MarketQuote: ...
    @classmethod
    def from_vol(cls, quote: VolQuote) -> MarketQuote: ...
    @classmethod
    def from_fx(cls, quote: FxQuote) -> MarketQuote: ...
    @classmethod
    def from_xccy(cls, quote: XccyQuote) -> MarketQuote: ...
    def __repr__(self) -> str: ...

def build_rate_instrument(quote: RateQuote, ctx: BuildCtx) -> BuiltInstrument: ...
def build_cds_instrument(quote: CdsQuote, ctx: BuildCtx) -> BuiltInstrument: ...
def build_cds_tranche_instrument(
    quote: CdsTrancheQuote,
    ctx: BuildCtx,
    overrides: CDSTrancheBuildOverrides,
) -> BuiltInstrument: ...
def build_bond_instrument(
    quote: BondQuote,
    ctx: BuildCtx,
    *,
    market: MarketContext | None = ...,
) -> BuiltInstrument: ...
def build_fx_instrument(quote: FxQuote, ctx: BuildCtx) -> BuiltInstrument: ...
def build_xccy_instrument(quote: XccyQuote, ctx: BuildCtx) -> BuiltInstrument: ...

__all__ = [
    "QuoteId",
    "Pillar",
    "BuildCtx",
    "BuiltInstrument",
    "RateQuote",
    "CdsQuote",
    "CdsTrancheQuote",
    "CDSTrancheBuildOverrides",
    "BondQuote",
    "InflationQuote",
    "VolQuote",
    "FxQuote",
    "XccyQuote",
    "MarketQuote",
    "build_rate_instrument",
    "build_cds_instrument",
    "build_cds_tranche_instrument",
    "build_bond_instrument",
    "build_fx_instrument",
    "build_xccy_instrument",
]
