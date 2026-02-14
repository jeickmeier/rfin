"""Market builders (quotes + BuildCtx) type stubs."""

from __future__ import annotations

from typing import Any, Optional

from finstack.valuations.common import InstrumentType
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
    def __init__(self, as_of: Any, notional: float, *, curve_ids: Optional[dict[str, str]] = ...) -> None: ...
    @property
    def as_of(self) -> Any: ...
    @property
    def notional(self) -> float: ...
    def curve_id(self, role: str) -> Optional[str]: ...

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
        convexity_adjustment: Optional[float] = ...,
        vol_surface_id: Optional[str] = ...,
    ) -> RateQuote: ...
    @classmethod
    def swap(
        cls,
        id: QuoteId | str,
        index: Any,
        pillar: Any,
        rate: float,
        *,
        spread_decimal: Optional[float] = ...,
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

class CdsTrancheBuildOverrides:
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

def build_rate_instrument(quote: RateQuote, ctx: BuildCtx) -> BuiltInstrument: ...
def build_cds_instrument(quote: CdsQuote, ctx: BuildCtx) -> BuiltInstrument: ...
def build_cds_tranche_instrument(
    quote: CdsTrancheQuote,
    ctx: BuildCtx,
    overrides: CdsTrancheBuildOverrides,
) -> BuiltInstrument: ...

__all__ = [
    "QuoteId",
    "Pillar",
    "BuildCtx",
    "BuiltInstrument",
    "RateQuote",
    "CdsQuote",
    "CdsTrancheQuote",
    "CdsTrancheBuildOverrides",
    "build_rate_instrument",
    "build_cds_instrument",
    "build_cds_tranche_instrument",
]
