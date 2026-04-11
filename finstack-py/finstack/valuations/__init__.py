"""Instrument pricing, risk metrics, and P&L attribution.

Bindings for the ``finstack-valuations`` Rust crate.
"""

from __future__ import annotations

from finstack.finstack import valuations as _valuations

ValuationResult = _valuations.ValuationResult
validate_instrument_json = _valuations.validate_instrument_json
price_instrument = _valuations.price_instrument
price_instrument_with_metrics = _valuations.price_instrument_with_metrics
list_standard_metrics = _valuations.list_standard_metrics
PnlAttribution = _valuations.PnlAttribution
attribute_pnl = _valuations.attribute_pnl
attribute_pnl_from_spec = _valuations.attribute_pnl_from_spec
validate_attribution_json = _valuations.validate_attribution_json
default_waterfall_order = _valuations.default_waterfall_order
default_attribution_metrics = _valuations.default_attribution_metrics
SensitivityMatrix = _valuations.SensitivityMatrix
FactorPnlProfile = _valuations.FactorPnlProfile
compute_factor_sensitivities = _valuations.compute_factor_sensitivities
compute_pnl_profiles = _valuations.compute_pnl_profiles
RiskDecomposition = _valuations.RiskDecomposition
decompose_factor_risk = _valuations.decompose_factor_risk
CalibrationResult = _valuations.CalibrationResult
validate_calibration_json = _valuations.validate_calibration_json
calibrate = _valuations.calibrate
calibrate_to_market = _valuations.calibrate_to_market

__all__: list[str] = [
    "ValuationResult",
    "list_standard_metrics",
    "price_instrument",
    "price_instrument_with_metrics",
    "validate_instrument_json",
    "PnlAttribution",
    "attribute_pnl",
    "attribute_pnl_from_spec",
    "validate_attribution_json",
    "default_waterfall_order",
    "default_attribution_metrics",
    "SensitivityMatrix",
    "FactorPnlProfile",
    "compute_factor_sensitivities",
    "compute_pnl_profiles",
    "RiskDecomposition",
    "decompose_factor_risk",
    "CalibrationResult",
    "validate_calibration_json",
    "calibrate",
    "calibrate_to_market",
]
