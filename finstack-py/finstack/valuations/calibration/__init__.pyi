"""Typing stubs for :mod:`finstack.valuations.calibration` (Rust extension module)."""

from __future__ import annotations

from typing import Any

from finstack.core.market_data.context import MarketContext

from .config import (
    CalibrationConfig,
    CalibrationMethod,
    RateBounds,
    SolverKind,
    ValidationMode,
)
from .hull_white import HullWhiteParams, SwaptionQuote, calibrate_hull_white
from .quote import (
    CreditQuote,
    InflationQuote,
    MarketQuote,
    RatesQuote,
    VolQuote,
)
from .report import CalibrationReport
from .sabr import SABRCalibrationDerivatives, SABRMarketData, SABRModelParams
from .validation import (
    ValidationConfig,
    validate_discount_curve,
    validate_forward_curve,
    validate_hazard_curve,
    validate_inflation_curve,
    validate_vol_surface,
)

CALIBRATION_SCHEMA: str

def execute_calibration(
    plan_id: str,
    quote_sets: dict[str, list[MarketQuote]],
    steps: list[dict[str, Any]],
    settings: CalibrationConfig | None = ...,
    initial_market: MarketContext | None = ...,
    description: str | None = ...,
) -> tuple[MarketContext, CalibrationReport, dict[str, CalibrationReport]]: ...

__all__ = [
    "CALIBRATION_SCHEMA",
    "execute_calibration",
    "SolverKind",
    "CalibrationMethod",
    "ValidationMode",
    "RateBounds",
    "CalibrationConfig",
    "RatesQuote",
    "CreditQuote",
    "VolQuote",
    "InflationQuote",
    "MarketQuote",
    "CalibrationReport",
    "ValidationConfig",
    "validate_discount_curve",
    "validate_forward_curve",
    "validate_hazard_curve",
    "validate_inflation_curve",
    "validate_vol_surface",
    "HullWhiteParams",
    "SwaptionQuote",
    "calibrate_hull_white",
    "SABRModelParams",
    "SABRMarketData",
    "SABRCalibrationDerivatives",
]
