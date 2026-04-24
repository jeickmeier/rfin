"""Instrument pricing, risk metrics, and P&L attribution.

Bindings for the ``finstack-valuations`` Rust crate.
"""

from __future__ import annotations

from finstack.finstack import valuations as _valuations
from finstack.valuations import correlation as correlation, instruments as instruments

ValuationResult = _valuations.ValuationResult
validate_instrument_json = _valuations.validate_instrument_json
price_instrument = _valuations.price_instrument
price_instrument_with_metrics = _valuations.price_instrument_with_metrics
list_standard_metrics = _valuations.list_standard_metrics
list_standard_metrics_grouped = _valuations.list_standard_metrics_grouped
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
tarn_coupon_profile = _valuations.tarn_coupon_profile
snowball_coupon_profile = _valuations.snowball_coupon_profile
cms_spread_option_intrinsic = _valuations.cms_spread_option_intrinsic
callable_range_accrual_accrued = _valuations.callable_range_accrual_accrued
bs_cos_price = _valuations.bs_cos_price
vg_cos_price = _valuations.vg_cos_price
merton_jump_cos_price = _valuations.merton_jump_cos_price

__all__: list[str] = [
    "CalibrationResult",
    "FactorPnlProfile",
    "PnlAttribution",
    "RiskDecomposition",
    "SensitivityMatrix",
    "ValuationResult",
    "attribute_pnl",
    "attribute_pnl_from_spec",
    "bs_cos_price",
    "calibrate",
    "callable_range_accrual_accrued",
    "cms_spread_option_intrinsic",
    "compute_factor_sensitivities",
    "compute_pnl_profiles",
    "correlation",
    "decompose_factor_risk",
    "default_attribution_metrics",
    "default_waterfall_order",
    "instruments",
    "list_standard_metrics",
    "list_standard_metrics_grouped",
    "merton_jump_cos_price",
    "price_instrument",
    "price_instrument_with_metrics",
    "snowball_coupon_profile",
    "tarn_coupon_profile",
    "validate_attribution_json",
    "validate_calibration_json",
    "validate_instrument_json",
    "vg_cos_price",
]
