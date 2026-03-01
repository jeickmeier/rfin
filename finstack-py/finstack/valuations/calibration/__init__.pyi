"""Typing stubs for :mod:`finstack.valuations.calibration` (Rust extension module)."""

from __future__ import annotations

from typing import Any

from finstack.core.market_data.context import MarketContext

from .config import (
    CalibrationConfig,
    CalibrationMethod,
    DiscountCurveSolveConfig,
    HazardCurveSolveConfig,
    InflationCurveSolveConfig,
    RateBounds,
    RateBoundsPolicy,
    RatesStepConventions,
    ResidualWeightingScheme,
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
    validate_base_correlation_curve,
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
    "CalibrationConfig",
    "CalibrationMethod",
    "CalibrationReport",
    "CreditQuote",
    "DiscountCurveSolveConfig",
    "HazardCurveSolveConfig",
    "HullWhiteParams",
    "InflationCurveSolveConfig",
    "InflationQuote",
    "MarketQuote",
    "RateBounds",
    "RateBoundsPolicy",
    "RatesQuote",
    "RatesStepConventions",
    "ResidualWeightingScheme",
    "SABRCalibrationDerivatives",
    "SABRMarketData",
    "SABRModelParams",
    "SolverKind",
    "SwaptionQuote",
    "ValidationConfig",
    "ValidationMode",
    "VolQuote",
    "calibrate_hull_white",
    "execute_calibration",
    "validate_base_correlation_curve",
    "validate_discount_curve",
    "validate_forward_curve",
    "validate_hazard_curve",
    "validate_inflation_curve",
    "validate_vol_surface",
]
