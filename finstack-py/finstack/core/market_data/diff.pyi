'"""Market data comparison helpers."""'

from __future__ import annotations

from typing import Iterable, Sequence

from ..currency import Currency
from ..dates import Date
from .context import MarketContext

class TenorSamplingMethod:
    """Curve sampling strategy for measuring shifts."""

    STANDARD: TenorSamplingMethod
    DYNAMIC: TenorSamplingMethod

    @classmethod
    def default(cls) -> TenorSamplingMethod: ...
    @staticmethod
    def custom(tenors: Sequence[float]) -> TenorSamplingMethod: ...

def standard_tenors() -> list[float]: ...
def measure_discount_curve_shift(
    curve_id: str,
    market_t0: MarketContext,
    market_t1: MarketContext,
    method: TenorSamplingMethod | None = ...,
) -> float: ...
def measure_bucketed_discount_shift(
    curve_id: str,
    market_t0: MarketContext,
    market_t1: MarketContext,
    tenors: Sequence[float],
) -> list[tuple[float, float]]: ...
def measure_hazard_curve_shift(
    curve_id: str,
    market_t0: MarketContext,
    market_t1: MarketContext,
    method: TenorSamplingMethod | None = ...,
) -> float: ...
def measure_inflation_curve_shift(
    curve_id: str,
    market_t0: MarketContext,
    market_t1: MarketContext,
) -> float: ...
def measure_correlation_shift(
    curve_id: str,
    market_t0: MarketContext,
    market_t1: MarketContext,
) -> float: ...
def measure_vol_surface_shift(
    surface_id: str,
    market_t0: MarketContext,
    market_t1: MarketContext,
    reference_expiry: float | None = ...,
    reference_strike: float | None = ...,
) -> float: ...
def measure_fx_shift(
    base_currency: Currency,
    quote_currency: Currency,
    market_t0: MarketContext,
    market_t1: MarketContext,
    as_of_t0: Date,
    as_of_t1: Date,
) -> float: ...
def measure_scalar_shift(
    scalar_id: str,
    market_t0: MarketContext,
    market_t1: MarketContext,
) -> float: ...

ATM_MONEYNESS: float
DEFAULT_VOL_EXPIRY: float
STANDARD_TENORS: Sequence[float]

__all__ = [
    "TenorSamplingMethod",
    "standard_tenors",
    "measure_discount_curve_shift",
    "measure_bucketed_discount_shift",
    "measure_hazard_curve_shift",
    "measure_inflation_curve_shift",
    "measure_correlation_shift",
    "measure_vol_surface_shift",
    "measure_fx_shift",
    "measure_scalar_shift",
    "ATM_MONEYNESS",
    "DEFAULT_VOL_EXPIRY",
    "STANDARD_TENORS",
]
