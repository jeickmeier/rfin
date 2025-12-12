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
from .methods import (
    BaseCorrelationCalibrator,
    DiscountCurveCalibrator,
    ForwardCurveCalibrator,
    HazardCurveCalibrator,
    InflationCurveCalibrator,
    VolSurfaceCalibrator,
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

__all__ = [
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
    "DiscountCurveCalibrator",
    "ForwardCurveCalibrator",
    "HazardCurveCalibrator",
    "InflationCurveCalibrator",
    "VolSurfaceCalibrator",
    "BaseCorrelationCalibrator",
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
