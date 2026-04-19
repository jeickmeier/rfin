"""Instrument pricing, risk metrics, and P&L attribution.

Bindings for the ``finstack-valuations`` Rust crate.
"""

from __future__ import annotations

from finstack.finstack import valuations as _valuations
from finstack.valuations import correlation as correlation

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
metrics_table_from_dict = _valuations.metrics_table_from_dict
cashflow_ladder = _valuations.cashflow_ladder
scenario_matrix = _valuations.scenario_matrix
waterfall_from_steps = _valuations.waterfall_from_steps
format_bps = _valuations.format_bps
format_pct = _valuations.format_pct
format_currency = _valuations.format_currency
format_ratio = _valuations.format_ratio
format_scientific = _valuations.format_scientific
tarn_coupon_profile = _valuations.tarn_coupon_profile
snowball_coupon_profile = _valuations.snowball_coupon_profile
cms_spread_option_intrinsic = _valuations.cms_spread_option_intrinsic
callable_range_accrual_accrued = _valuations.callable_range_accrual_accrued
bs_cos_price = _valuations.bs_cos_price
bs_lewis_price = _valuations.bs_lewis_price
vg_cos_price = _valuations.vg_cos_price
merton_jump_cos_price = _valuations.merton_jump_cos_price
ValuationCache = _valuations.ValuationCache
execute_recovery_waterfall = _valuations.execute_recovery_waterfall
analyze_exchange_offer = _valuations.analyze_exchange_offer
analyze_lme = _valuations.analyze_lme

__all__: list[str] = [
    "CalibrationResult",
    "FactorPnlProfile",
    "PnlAttribution",
    "RiskDecomposition",
    "SensitivityMatrix",
    "ValuationCache",
    "ValuationResult",
    "analyze_exchange_offer",
    "analyze_lme",
    "attribute_pnl",
    "attribute_pnl_from_spec",
    "bs_cos_price",
    "bs_lewis_price",
    "calibrate",
    "callable_range_accrual_accrued",
    "cashflow_ladder",
    "cms_spread_option_intrinsic",
    "compute_factor_sensitivities",
    "compute_pnl_profiles",
    "correlation",
    "decompose_factor_risk",
    "default_attribution_metrics",
    "default_waterfall_order",
    "execute_recovery_waterfall",
    "format_bps",
    "format_currency",
    "format_pct",
    "format_ratio",
    "format_scientific",
    "list_standard_metrics",
    "list_standard_metrics_grouped",
    "merton_jump_cos_price",
    "metrics_table_from_dict",
    "price_instrument",
    "price_instrument_with_metrics",
    "scenario_matrix",
    "snowball_coupon_profile",
    "tarn_coupon_profile",
    "validate_attribution_json",
    "validate_calibration_json",
    "validate_instrument_json",
    "vg_cos_price",
    "waterfall_from_steps",
]
