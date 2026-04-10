use super::helpers::{factor_to_label, parse_model_params_snapshot};
use super::results::{PyPnlAttribution, PyPortfolioAttribution};
use super::taylor::{
    PyModelParamsSnapshot, PyScalarsSnapshot, PyTaylorAttributionConfig, PyTaylorAttributionResult,
};
use super::types::PyAttributionMethod;
use crate::core::currency::extract_currency;
use crate::core::dates::utils::py_to_date;
use crate::core::market_data::context::PyMarketContext;
use crate::errors::core_to_py;
use finstack_core::config::FinstackConfig;
use finstack_valuations::attribution::{
    attribute_pnl_metrics_based, attribute_pnl_parallel, attribute_pnl_taylor,
    attribute_pnl_taylor_standard, attribute_pnl_waterfall, compute_pnl, compute_pnl_with_fx,
    convert_currency, default_attribution_metrics, default_waterfall_order,
    extract_model_params as rust_extract_model_params, measure_conversion_shift,
    measure_default_shift, measure_prepayment_shift, measure_recovery_shift, reprice_instrument,
    restore_scalars as rust_restore_scalars, AttributionMethod, JsonEnvelope,
};
use finstack_valuations::instruments::internal::InstrumentExt as Instrument;
use pyo3::prelude::*;
use std::sync::Arc;

/// Python function to perform P&L attribution.
///
/// # Arguments
///
/// * `instrument` - Any finstack instrument (Bond, IRS, Equity, etc.)
/// * `market_t0` - Market context at T₀
/// * `market_t1` - Market context at T₁
/// * `as_of_t0` - Valuation date at T₀ (date or datetime)
/// * `as_of_t1` - Valuation date at T₁ (date or datetime)
/// * `method` - Optional attribution method (defaults to Parallel)
/// * `model_params_t0` - Optional dict/JSON describing T₀ model parameters
///
/// # Returns
///
/// PnlAttribution with complete factor breakdown
///
/// # Examples
///
/// ```python
/// model_params_t0 = {
///     "StructuredCredit": {
///         "prepayment_spec": {"psa": {"speed_multiplier": 1.0}},
///         "default_spec": {"type": "ConstantCdr", "cdr": 0.02},
///         "recovery_spec": {"type": "Constant", "rate": 0.60}
///     }
/// }
///
/// attr = finstack.attribute_pnl(
///     bond,
///     market_yesterday,
///     market_today,
///     date(2025, 1, 15),
///     date(2025, 1, 16),
///     method=finstack.AttributionMethod.parallel(),
///     model_params_t0=model_params_t0,
/// )
/// ```
#[pyfunction]
#[pyo3(signature = (instrument, market_t0, market_t1, as_of_t0, as_of_t1, method=None, model_params_t0=None))]
#[allow(clippy::too_many_arguments)]
pub(crate) fn attribute_pnl(
    py: Python<'_>,
    instrument: Bound<'_, PyAny>,
    market_t0: &crate::core::market_data::PyMarketContext,
    market_t1: &crate::core::market_data::PyMarketContext,
    as_of_t0: Bound<'_, PyAny>,
    as_of_t1: Bound<'_, PyAny>,
    method: Option<&PyAttributionMethod>,
    model_params_t0: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyPnlAttribution> {
    let date_t0 = py_to_date(&as_of_t0)?;
    let date_t1 = py_to_date(&as_of_t1)?;

    let handle = crate::valuations::instruments::extract_instrument(&instrument)?;
    let instrument_arc: Arc<dyn Instrument> = handle.instrument;

    let method_inner = method
        .map(|m| m.inner.clone())
        .unwrap_or(AttributionMethod::Parallel);

    let config = FinstackConfig::default();
    let model_params_snapshot = parse_model_params_snapshot(model_params_t0)?;
    let model_params_ref = model_params_snapshot.as_ref();

    let attribution = py.detach(|| match method_inner {
        AttributionMethod::Parallel => attribute_pnl_parallel(
            &instrument_arc,
            &market_t0.inner,
            &market_t1.inner,
            date_t0,
            date_t1,
            &config,
            model_params_ref,
        )
        .map_err(core_to_py),

        AttributionMethod::Waterfall(order) => attribute_pnl_waterfall(
            &instrument_arc,
            &market_t0.inner,
            &market_t1.inner,
            date_t0,
            date_t1,
            &config,
            order,
            false,
            model_params_ref,
        )
        .map_err(core_to_py),

        AttributionMethod::MetricsBased => {
            let metrics = method_inner.required_metrics();
            let val_t0 = instrument_arc
                .price_with_metrics(
                    &market_t0.inner,
                    date_t0,
                    &metrics,
                    finstack_valuations::instruments::PricingOptions::default(),
                )
                .map_err(core_to_py)?;

            let val_t1 = instrument_arc
                .price_with_metrics(
                    &market_t1.inner,
                    date_t1,
                    &metrics,
                    finstack_valuations::instruments::PricingOptions::default(),
                )
                .map_err(core_to_py)?;

            attribute_pnl_metrics_based(
                &instrument_arc,
                &market_t0.inner,
                &market_t1.inner,
                &val_t0,
                &val_t1,
                date_t0,
                date_t1,
            )
            .map_err(core_to_py)
        }
        AttributionMethod::Taylor(config_inner) => attribute_pnl_taylor_standard(
            &instrument_arc,
            &market_t0.inner,
            &market_t1.inner,
            date_t0,
            date_t1,
            &config_inner,
        )
        .map_err(core_to_py),
    })?;

    Ok(PyPnlAttribution { inner: attribution })
}

/// Python function to perform Taylor-based P&L attribution.
#[pyfunction(name = "attribute_pnl_taylor")]
#[pyo3(signature = (instrument, market_t0, market_t1, as_of_t0, as_of_t1, config=None))]
pub(crate) fn attribute_pnl_taylor_py(
    py: Python<'_>,
    instrument: Bound<'_, PyAny>,
    market_t0: &PyMarketContext,
    market_t1: &PyMarketContext,
    as_of_t0: Bound<'_, PyAny>,
    as_of_t1: Bound<'_, PyAny>,
    config: Option<&PyTaylorAttributionConfig>,
) -> PyResult<PyTaylorAttributionResult> {
    let date_t0 = py_to_date(&as_of_t0)?;
    let date_t1 = py_to_date(&as_of_t1)?;
    let handle = crate::valuations::instruments::extract_instrument(&instrument)?;
    let instrument_arc: Arc<dyn Instrument> = handle.instrument;
    let taylor_config = config.map(|value| value.inner.clone()).unwrap_or_default();
    let result = py.detach(|| {
        attribute_pnl_taylor(
            &instrument_arc,
            &market_t0.inner,
            &market_t1.inner,
            date_t0,
            date_t1,
            &taylor_config,
        )
        .map_err(core_to_py)
    })?;
    Ok(PyTaylorAttributionResult { inner: result })
}

/// Reprice an instrument directly from Python.
#[pyfunction(name = "reprice_instrument")]
pub(crate) fn reprice_instrument_py(
    instrument: Bound<'_, PyAny>,
    market: &PyMarketContext,
    as_of: Bound<'_, PyAny>,
) -> PyResult<crate::core::money::PyMoney> {
    let date = py_to_date(&as_of)?;
    let handle = crate::valuations::instruments::extract_instrument(&instrument)?;
    let instrument_arc: Arc<dyn Instrument> = handle.instrument;
    let value = reprice_instrument(&instrument_arc, &market.inner, date).map_err(core_to_py)?;
    Ok(crate::core::money::PyMoney { inner: value })
}

/// Convert money into a target currency using a market FX matrix.
#[pyfunction(name = "convert_currency")]
pub(crate) fn convert_currency_py(
    money: &crate::core::money::PyMoney,
    target_ccy: Bound<'_, PyAny>,
    market: &PyMarketContext,
    as_of: Bound<'_, PyAny>,
) -> PyResult<crate::core::money::PyMoney> {
    let target_ccy = extract_currency(&target_ccy)?;
    let date = py_to_date(&as_of)?;
    let value =
        convert_currency(money.inner, target_ccy, &market.inner, date).map_err(core_to_py)?;
    Ok(crate::core::money::PyMoney { inner: value })
}

/// Compute P&L in a target currency using T1 FX conversion.
#[pyfunction(name = "compute_pnl")]
pub(crate) fn compute_pnl_py(
    val_t0: &crate::core::money::PyMoney,
    val_t1: &crate::core::money::PyMoney,
    target_ccy: Bound<'_, PyAny>,
    market_t1: &PyMarketContext,
    as_of_t1: Bound<'_, PyAny>,
) -> PyResult<crate::core::money::PyMoney> {
    let target_ccy = extract_currency(&target_ccy)?;
    let date_t1 = py_to_date(&as_of_t1)?;
    let pnl = compute_pnl(
        val_t0.inner,
        val_t1.inner,
        target_ccy,
        &market_t1.inner,
        date_t1,
    )
    .map_err(core_to_py)?;
    Ok(crate::core::money::PyMoney { inner: pnl })
}

/// Compute P&L with date-appropriate FX conversion at T0 and T1.
#[pyfunction(name = "compute_pnl_with_fx")]
pub(crate) fn compute_pnl_with_fx_py(
    val_t0: &crate::core::money::PyMoney,
    val_t1: &crate::core::money::PyMoney,
    target_ccy: Bound<'_, PyAny>,
    market_fx_t0: &PyMarketContext,
    market_fx_t1: &PyMarketContext,
    as_of_t0: Bound<'_, PyAny>,
    as_of_t1: Bound<'_, PyAny>,
) -> PyResult<crate::core::money::PyMoney> {
    let target_ccy = extract_currency(&target_ccy)?;
    let date_t0 = py_to_date(&as_of_t0)?;
    let date_t1 = py_to_date(&as_of_t1)?;
    let pnl = compute_pnl_with_fx(
        val_t0.inner,
        val_t1.inner,
        target_ccy,
        &market_fx_t0.inner,
        &market_fx_t1.inner,
        date_t0,
        date_t1,
    )
    .map_err(core_to_py)?;
    Ok(crate::core::money::PyMoney { inner: pnl })
}

/// Default waterfall factor order expressed using Python factor labels.
#[pyfunction(name = "default_waterfall_order")]
pub(crate) fn default_waterfall_order_py() -> Vec<String> {
    default_waterfall_order()
        .into_iter()
        .map(|factor| factor_to_label(&factor).to_string())
        .collect()
}

/// Default set of metrics for metrics-based attribution.
#[pyfunction(name = "default_attribution_metrics")]
pub(crate) fn default_attribution_metrics_py() -> Vec<String> {
    default_attribution_metrics()
        .into_iter()
        .map(|m| m.to_string())
        .collect()
}

/// Extract model parameters from an instrument.
#[pyfunction(name = "extract_model_params")]
pub(crate) fn extract_model_params_py(
    instrument: Bound<'_, PyAny>,
) -> PyResult<PyModelParamsSnapshot> {
    let handle = crate::valuations::instruments::extract_instrument(&instrument)?;
    let snapshot = rust_extract_model_params(&handle.instrument);
    Ok(PyModelParamsSnapshot { inner: snapshot })
}

/// Measure prepayment parameter shift between two snapshots (in basis points).
#[pyfunction(name = "measure_prepayment_shift")]
pub(crate) fn measure_prepayment_shift_py(
    snapshot_t0: &PyModelParamsSnapshot,
    snapshot_t1: &PyModelParamsSnapshot,
) -> f64 {
    measure_prepayment_shift(&snapshot_t0.inner, &snapshot_t1.inner)
}

/// Measure default rate parameter shift between two snapshots (in basis points).
#[pyfunction(name = "measure_default_shift")]
pub(crate) fn measure_default_shift_py(
    snapshot_t0: &PyModelParamsSnapshot,
    snapshot_t1: &PyModelParamsSnapshot,
) -> f64 {
    measure_default_shift(&snapshot_t0.inner, &snapshot_t1.inner)
}

/// Measure recovery rate parameter shift between two snapshots (in percentage points).
#[pyfunction(name = "measure_recovery_shift")]
pub(crate) fn measure_recovery_shift_py(
    snapshot_t0: &PyModelParamsSnapshot,
    snapshot_t1: &PyModelParamsSnapshot,
) -> f64 {
    measure_recovery_shift(&snapshot_t0.inner, &snapshot_t1.inner)
}

/// Measure conversion ratio shift between two snapshots (in percentage points).
#[pyfunction(name = "measure_conversion_shift")]
pub(crate) fn measure_conversion_shift_py(
    snapshot_t0: &PyModelParamsSnapshot,
    snapshot_t1: &PyModelParamsSnapshot,
) -> f64 {
    measure_conversion_shift(&snapshot_t0.inner, &snapshot_t1.inner)
}

/// Replace market scalars with snapshot values, preserving curves/FX/surfaces.
#[pyfunction(name = "restore_scalars")]
pub(crate) fn restore_scalars_py(
    market: &PyMarketContext,
    snapshot: &PyScalarsSnapshot,
) -> PyMarketContext {
    PyMarketContext {
        inner: rust_restore_scalars(&market.inner, &snapshot.inner),
    }
}

/// Python function to perform portfolio P&L attribution.
///
/// # Arguments
///
/// * `portfolio` - Portfolio to attribute
/// * `market_t0` - Market context at T₀
/// * `market_t1` - Market context at T₁
/// * `as_of_t0` - Valuation date at T₀ (e.g., yesterday for day-over-day)
/// * `as_of_t1` - Valuation date at T₁ (e.g., today for day-over-day)
/// * `method` - Optional attribution method (defaults to Parallel)
///
/// # Returns
///
/// PortfolioAttribution with position-by-position breakdown
///
/// # Examples
///
/// ```python
/// import datetime
///
/// as_of_t0 = datetime.date(2025, 11, 20)  # Yesterday
/// as_of_t1 = datetime.date(2025, 11, 21)  # Today
///
/// attr = finstack.attribute_portfolio_pnl(
///     portfolio,
///     market_yesterday,
///     market_today,
///     as_of_t0,
///     as_of_t1,
///     method=finstack.AttributionMethod.parallel()
/// )
/// ```
#[pyfunction]
#[pyo3(signature = (portfolio, market_t0, market_t1, as_of_t0, as_of_t1, method=None))]
pub(crate) fn attribute_portfolio_pnl(
    py: Python<'_>,
    portfolio: &crate::portfolio::positions::PyPortfolio,
    market_t0: &crate::core::market_data::PyMarketContext,
    market_t1: &crate::core::market_data::PyMarketContext,
    as_of_t0: &Bound<'_, pyo3::PyAny>,
    as_of_t1: &Bound<'_, pyo3::PyAny>,
    method: Option<&PyAttributionMethod>,
) -> PyResult<PyPortfolioAttribution> {
    let date_t0 = py_to_date(as_of_t0)?;
    let date_t1 = py_to_date(as_of_t1)?;

    let method_inner = method
        .map(|m| m.inner.clone())
        .unwrap_or(AttributionMethod::Parallel);

    let config = FinstackConfig::default();

    let attribution = py.detach(|| {
        finstack_portfolio::attribute_portfolio_pnl(
            &portfolio.inner,
            &market_t0.inner,
            &market_t1.inner,
            date_t0,
            date_t1,
            &config,
            method_inner,
        )
        .map_err(crate::portfolio::error::portfolio_to_py)
    })?;

    Ok(PyPortfolioAttribution { inner: attribution })
}

/// Python function to perform P&L attribution from a JSON specification.
///
/// # Arguments
///
/// * `spec_json` - JSON string containing AttributionEnvelope
///
/// # Returns
///
/// PnlAttribution with complete factor breakdown
///
/// # Examples
///
/// ```python
/// json_spec = '''
/// {
///   "schema": "finstack.attribution/1",
///   "attribution": {
///     "instrument": { ... },
///     "market_t0": { ... },
///     "market_t1": { ... },
///     "as_of_t0": "2025-01-15",
///     "as_of_t1": "2025-01-16",
///     "method": "Parallel"
///   }
/// }
/// '''
/// attr = finstack.attribute_pnl_from_json(json_spec)
/// ```
#[pyfunction]
pub(crate) fn attribute_pnl_from_json(spec_json: &str) -> PyResult<PyPnlAttribution> {
    use finstack_valuations::attribution::AttributionEnvelope;

    let envelope = AttributionEnvelope::from_json(spec_json).map_err(core_to_py)?;
    let result_envelope = envelope.execute().map_err(core_to_py)?;

    Ok(PyPnlAttribution {
        inner: result_envelope.result.attribution,
    })
}

/// Python function to serialize an attribution result to JSON.
///
/// # Arguments
///
/// * `attribution` - PnlAttribution result
///
/// # Returns
///
/// JSON string with AttributionResultEnvelope
#[pyfunction]
pub(crate) fn attribution_result_to_json(attribution: &PyPnlAttribution) -> PyResult<String> {
    use finstack_core::config::results_meta;
    use finstack_valuations::attribution::{AttributionResult, AttributionResultEnvelope};

    let result = AttributionResult {
        attribution: attribution.inner.clone(),
        results_meta: results_meta(&finstack_core::config::FinstackConfig::default()),
    };

    let envelope = AttributionResultEnvelope::new(result);
    envelope.to_json().map_err(core_to_py)
}
