//! Python bindings for P&L attribution.

mod functions;
mod helpers;
mod results;
mod taylor;
mod types;

pub(crate) use functions::{
    attribute_pnl, attribute_pnl_from_json, attribute_pnl_taylor_py, attribute_portfolio_pnl,
    attribution_result_to_json, compute_pnl_py, compute_pnl_with_fx_py, convert_currency_py,
    default_attribution_metrics_py, default_waterfall_order_py, extract_model_params_py,
    measure_conversion_shift_py, measure_default_shift_py, measure_prepayment_shift_py,
    measure_recovery_shift_py, reprice_instrument_py, restore_scalars_py,
};
pub(crate) use results::{PyPnlAttribution, PyPortfolioAttribution};
pub(crate) use taylor::{
    PyAttributionConfig, PyCurveRestoreFlags, PyMarketSnapshot, PyModelParamsSnapshot,
    PyScalarsSnapshot, PyTaylorAttributionConfig, PyTaylorAttributionResult, PyTaylorFactorResult,
    PyVolatilitySnapshot,
};
pub(crate) use types::{
    PyAttributionMeta, PyAttributionMethod, PyCarryDetail, PyCorrelationsAttribution,
    PyCreditCurvesAttribution, PyCrossFactorDetail, PyFxAttribution, PyInflationCurvesAttribution,
    PyModelParamsAttribution, PyRatesCurvesAttribution, PyScalarsAttribution, PyVolAttribution,
};

use finstack_valuations::attribution::ATTRIBUTION_SCHEMA_V1;
use pyo3::prelude::*;
use pyo3::types::PyList;

/// Register attribution bindings with Python module.
pub fn register(module: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyAttributionMethod>()?;
    module.add_class::<PyAttributionMeta>()?;
    module.add_class::<PyAttributionConfig>()?;
    module.add_class::<PyModelParamsSnapshot>()?;
    module.add_class::<PyRatesCurvesAttribution>()?;
    module.add_class::<PyCreditCurvesAttribution>()?;
    module.add_class::<PyModelParamsAttribution>()?;
    module.add_class::<PyCarryDetail>()?;
    module.add_class::<PyInflationCurvesAttribution>()?;
    module.add_class::<PyCorrelationsAttribution>()?;
    module.add_class::<PyFxAttribution>()?;
    module.add_class::<PyVolAttribution>()?;
    module.add_class::<PyScalarsAttribution>()?;
    module.add_class::<PyCrossFactorDetail>()?;
    module.add_class::<PyTaylorAttributionConfig>()?;
    module.add_class::<PyTaylorFactorResult>()?;
    module.add_class::<PyTaylorAttributionResult>()?;
    module.add_class::<PyCurveRestoreFlags>()?;
    module.add_class::<PyMarketSnapshot>()?;
    module.add_class::<PyVolatilitySnapshot>()?;
    module.add_class::<PyScalarsSnapshot>()?;
    module.add_class::<PyPnlAttribution>()?;
    module.add_class::<PyPortfolioAttribution>()?;
    module.add("ATTRIBUTION_SCHEMA_V1", ATTRIBUTION_SCHEMA_V1)?;
    module.add_function(wrap_pyfunction!(attribute_pnl, module)?)?;
    module.add_function(wrap_pyfunction!(attribute_pnl_taylor_py, module)?)?;
    module.add_function(wrap_pyfunction!(attribute_portfolio_pnl, module)?)?;
    module.add_function(wrap_pyfunction!(attribute_pnl_from_json, module)?)?;
    module.add_function(wrap_pyfunction!(attribution_result_to_json, module)?)?;
    module.add_function(wrap_pyfunction!(reprice_instrument_py, module)?)?;
    module.add_function(wrap_pyfunction!(convert_currency_py, module)?)?;
    module.add_function(wrap_pyfunction!(compute_pnl_py, module)?)?;
    module.add_function(wrap_pyfunction!(compute_pnl_with_fx_py, module)?)?;
    module.add_function(wrap_pyfunction!(default_waterfall_order_py, module)?)?;
    module.add_function(wrap_pyfunction!(default_attribution_metrics_py, module)?)?;
    module.add_function(wrap_pyfunction!(extract_model_params_py, module)?)?;
    module.add_function(wrap_pyfunction!(measure_prepayment_shift_py, module)?)?;
    module.add_function(wrap_pyfunction!(measure_default_shift_py, module)?)?;
    module.add_function(wrap_pyfunction!(measure_recovery_shift_py, module)?)?;
    module.add_function(wrap_pyfunction!(measure_conversion_shift_py, module)?)?;
    module.add_function(wrap_pyfunction!(restore_scalars_py, module)?)?;

    let exports = vec![
        "AttributionMethod",
        "AttributionMeta",
        "AttributionConfig",
        "ModelParamsSnapshot",
        "RatesCurvesAttribution",
        "CreditCurvesAttribution",
        "ModelParamsAttribution",
        "CarryDetail",
        "InflationCurvesAttribution",
        "CorrelationsAttribution",
        "FxAttribution",
        "VolAttribution",
        "ScalarsAttribution",
        "CrossFactorDetail",
        "TaylorAttributionConfig",
        "TaylorFactorResult",
        "TaylorAttributionResult",
        "CurveRestoreFlags",
        "MarketSnapshot",
        "VolatilitySnapshot",
        "ScalarsSnapshot",
        "PnlAttribution",
        "PortfolioAttribution",
        "ATTRIBUTION_SCHEMA_V1",
        "attribute_pnl",
        "attribute_pnl_taylor",
        "attribute_portfolio_pnl",
        "attribute_pnl_from_json",
        "attribution_result_to_json",
        "reprice_instrument",
        "convert_currency",
        "compute_pnl",
        "compute_pnl_with_fx",
        "default_waterfall_order",
        "default_attribution_metrics",
        "extract_model_params",
        "measure_prepayment_shift",
        "measure_default_shift",
        "measure_recovery_shift",
        "measure_conversion_shift",
        "restore_scalars",
    ];
    let py = module.py();
    module.setattr("__doc__", "P&L attribution for instruments and portfolios.")?;
    module.setattr("__all__", PyList::new(py, &exports)?)?;

    Ok(exports)
}
