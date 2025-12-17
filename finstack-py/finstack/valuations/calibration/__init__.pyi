"""Typing stubs for :mod:`finstack.valuations.calibration` (Rust extension module)."""

from __future__ import annotations

from .config import (
    CalibrationConfig,
    CalibrationMethod,
    MultiCurveConfig,
    RateBounds,
    SolverKind,
    ValidationMode,
)
from .report import CalibrationReport
from .sabr import SABRCalibrationDerivatives, SABRMarketData, SABRModelParams
from .validation import (
    ValidationConfig,
    ValidationError,
    validate_discount_curve,
    validate_forward_curve,
    validate_hazard_curve,
    validate_inflation_curve,
    validate_vol_surface,
)
from .quote import (
    CreditQuote,
    FutureSpecs,
    InflationQuote,
    MarketQuote,
    RatesQuote,
    VolQuote,
)
from finstack.core.market_data import MarketContext
from typing import Any

CALIBRATION_SCHEMA_V2: str

def execute_calibration_v2(
    plan_id: str,
    quote_sets: dict[str, list[MarketQuote]],
    steps: list[dict[str, Any]],
    settings: CalibrationConfig | None = ...,
    initial_market: MarketContext | None = ...,
    description: str | None = ...,
) -> tuple[MarketContext, CalibrationReport, dict[str, CalibrationReport]]: ...

__all__ = [
    "CALIBRATION_SCHEMA_V2",
    "execute_calibration_v2",
    "SolverKind",
    "CalibrationMethod",
    "ValidationMode",
    "RateBounds",
    "MultiCurveConfig",
    "CalibrationConfig",
    "FutureSpecs",
    "RatesQuote",
    "CreditQuote",
    "VolQuote",
    "InflationQuote",
    "MarketQuote",
    "CalibrationReport",
    "ValidationError",
    "ValidationConfig",
    "validate_discount_curve",
    "validate_forward_curve",
    "validate_hazard_curve",
    "validate_inflation_curve",
    "validate_vol_surface",
    "SABRModelParams",
    "SABRMarketData",
    "SABRCalibrationDerivatives",
]
