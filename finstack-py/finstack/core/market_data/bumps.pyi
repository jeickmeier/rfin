'"""Market bump specifications for scenario generation."""'

from __future__ import annotations

from datetime import date
from typing import Sequence

from ..currency import Currency
from ..types import CurveId

class BumpMode:
    """Mode for applying a bump."""

    ADDITIVE: BumpMode
    MULTIPLICATIVE: BumpMode

class BumpUnits:
    """Units used to interpret bump magnitudes."""

    RATE_BP: BumpUnits
    PERCENT: BumpUnits
    FRACTION: BumpUnits
    FACTOR: BumpUnits

class BumpType:
    """Parallel or key-rate bump description."""

    PARALLEL: BumpType

    @staticmethod
    def key_rate(time_years: float) -> BumpType: ...
    @property
    def is_key_rate(self) -> bool: ...
    @property
    def time_years(self) -> float | None: ...

class BumpSpec:
    """Unified bump specification combining mode, units, magnitude, and type."""

    def __init__(
        self,
        mode: BumpMode,
        units: BumpUnits,
        value: float,
        bump_type: BumpType | None = ...,
    ) -> None: ...
    @staticmethod
    def parallel_bp(bump_bp: float) -> BumpSpec: ...
    @staticmethod
    def key_rate_bp(time_years: float, bump_bp: float) -> BumpSpec: ...
    @staticmethod
    def multiplier(factor: float) -> BumpSpec: ...
    @staticmethod
    def inflation_shift_pct(bump_pct: float) -> BumpSpec: ...
    @staticmethod
    def correlation_shift_pct(bump_pct: float) -> BumpSpec: ...
    @property
    def mode(self) -> BumpMode: ...
    @property
    def units(self) -> BumpUnits: ...
    @property
    def value(self) -> float: ...
    @property
    def bump_type(self) -> BumpType: ...

class MarketBump:
    """Concrete bump to apply to market data."""

    @classmethod
    def curve(cls, curve_id: CurveId, spec: BumpSpec) -> MarketBump: ...
    @classmethod
    def fx_pct(
        cls,
        base_currency: Currency,
        quote_currency: Currency,
        pct: float,
        as_of: date,
    ) -> MarketBump: ...
    @classmethod
    def vol_bucket_pct(
        cls,
        surface_id: CurveId,
        pct: float,
        expiries: Sequence[float] | None = ...,
        strikes: Sequence[float] | None = ...,
    ) -> MarketBump: ...
    @classmethod
    def base_corr_bucket_pts(
        cls,
        surface_id: CurveId,
        points: float,
        detachments: Sequence[float] | None = ...,
    ) -> MarketBump: ...
    @property
    def kind(self) -> str: ...

__all__ = [
    "BumpMode",
    "BumpUnits",
    "BumpType",
    "BumpSpec",
    "MarketBump",
]
