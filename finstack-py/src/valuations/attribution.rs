//! Python bindings for P&L attribution.

use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::errors::map_error;
use finstack_core::config::FinstackConfig;
use finstack_valuations::attribution::types::JsonEnvelope;
use finstack_valuations::attribution::{
    attribute_pnl_metrics_based, attribute_pnl_parallel, attribute_pnl_waterfall,
    AttributionFactor, AttributionMeta, AttributionMethod, CreditCurvesAttribution,
    ModelParamsAttribution, ModelParamsSnapshot, PnlAttribution, RatesCurvesAttribution,
};
use finstack_valuations::metrics::MetricId;
use pyo3::prelude::*;
use pyo3::types::PyString;
use pythonize::depythonize;
use serde_json::{self, Value};
use finstack_core::collections::HashMap;
use std::sync::Arc;

/// Python wrapper for AttributionMethod.
#[pyclass(name = "AttributionMethod")]
#[derive(Clone)]
pub struct PyAttributionMethod {
    pub(crate) inner: AttributionMethod,
}

fn parse_model_params_snapshot(
    value: Option<&Bound<'_, PyAny>>,
) -> PyResult<Option<ModelParamsSnapshot>> {
    if let Some(obj) = value {
        if obj.is_none() {
            return Ok(None);
        }

        if let Ok(text) = obj.downcast::<PyString>() {
            let snapshot: ModelParamsSnapshot =
                serde_json::from_str(text.to_str()?).map_err(|err| {
                    pyo3::exceptions::PyValueError::new_err(format!(
                        "Invalid model_params_t0 JSON: {err}"
                    ))
                })?;
            return Ok(Some(snapshot));
        }

        let json_value: Value = depythonize(obj)?;
        let snapshot: ModelParamsSnapshot = serde_json::from_value(json_value).map_err(|err| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "model_params_t0 does not match expected schema: {err}"
            ))
        })?;
        Ok(Some(snapshot))
    } else {
        Ok(None)
    }
}

#[pymethods]
impl PyAttributionMethod {
    #[staticmethod]
    fn parallel() -> Self {
        Self {
            inner: AttributionMethod::Parallel,
        }
    }

    #[staticmethod]
    fn waterfall(factors: Vec<String>) -> PyResult<Self> {
        let factors: Vec<AttributionFactor> = factors
            .into_iter()
            .map(|s| match s.to_lowercase().as_str() {
                "carry" => Ok(AttributionFactor::Carry),
                "rates_curves" => Ok(AttributionFactor::RatesCurves),
                "credit_curves" => Ok(AttributionFactor::CreditCurves),
                "inflation_curves" => Ok(AttributionFactor::InflationCurves),
                "correlations" => Ok(AttributionFactor::Correlations),
                "fx" => Ok(AttributionFactor::Fx),
                "volatility" => Ok(AttributionFactor::Volatility),
                "model_parameters" => Ok(AttributionFactor::ModelParameters),
                "market_scalars" => Ok(AttributionFactor::MarketScalars),
                _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Unknown attribution factor: {}",
                    s
                ))),
            })
            .collect::<PyResult<Vec<_>>>()?;

        Ok(Self {
            inner: AttributionMethod::Waterfall(factors),
        })
    }

    #[staticmethod]
    fn metrics_based() -> Self {
        Self {
            inner: AttributionMethod::MetricsBased,
        }
    }

    fn __repr__(&self) -> String {
        format!("{}", self.inner)
    }
}

/// Python wrapper for AttributionMeta.
#[pyclass(name = "AttributionMeta")]
#[derive(Clone)]
pub struct PyAttributionMeta {
    pub(crate) inner: AttributionMeta,
}

#[pymethods]
impl PyAttributionMeta {
    #[getter]
    fn method(&self) -> PyAttributionMethod {
        PyAttributionMethod {
            inner: self.inner.method.clone(),
        }
    }

    #[getter]
    fn t0(&self, py: Python) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.t0)
    }

    #[getter]
    fn t1(&self, py: Python) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.t1)
    }

    #[getter]
    fn instrument_id(&self) -> String {
        self.inner.instrument_id.clone()
    }

    #[getter]
    fn num_repricings(&self) -> usize {
        self.inner.num_repricings
    }

    #[getter]
    fn residual_pct(&self) -> f64 {
        self.inner.residual_pct
    }

    #[getter]
    fn tolerance_abs(&self) -> f64 {
        self.inner.tolerance_abs
    }

    #[getter]
    fn tolerance_pct(&self) -> f64 {
        self.inner.tolerance_pct
    }

    fn __repr__(&self) -> String {
        format!(
            "AttributionMeta(method={}, instrument={}, repricings={}, residual_pct={:.2}%)",
            self.inner.method,
            self.inner.instrument_id,
            self.inner.num_repricings,
            self.inner.residual_pct
        )
    }
}

/// Python wrapper for RatesCurvesAttribution.
#[pyclass(name = "RatesCurvesAttribution")]
#[derive(Clone)]
pub struct PyRatesCurvesAttribution {
    pub(crate) inner: RatesCurvesAttribution,
}

#[pymethods]
impl PyRatesCurvesAttribution {
    fn by_curve_to_dict(&self) -> HashMap<String, crate::core::money::PyMoney> {
        self.inner
            .by_curve
            .iter()
            .map(|(k, v)| (k.to_string(), crate::core::money::PyMoney { inner: *v }))
            .collect()
    }

    #[getter]
    fn discount_total(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.discount_total,
        }
    }

    #[getter]
    fn forward_total(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.forward_total,
        }
    }
}

/// Python wrapper for CreditCurvesAttribution.
#[pyclass(name = "CreditCurvesAttribution")]
#[derive(Clone)]
pub struct PyCreditCurvesAttribution {
    pub(crate) inner: CreditCurvesAttribution,
}

#[pymethods]
impl PyCreditCurvesAttribution {
    fn by_curve_to_dict(&self) -> HashMap<String, crate::core::money::PyMoney> {
        self.inner
            .by_curve
            .iter()
            .map(|(k, v)| (k.to_string(), crate::core::money::PyMoney { inner: *v }))
            .collect()
    }
}

/// Python wrapper for ModelParamsAttribution.
#[pyclass(name = "ModelParamsAttribution")]
#[derive(Clone)]
pub struct PyModelParamsAttribution {
    pub(crate) inner: ModelParamsAttribution,
}

#[pymethods]
impl PyModelParamsAttribution {
    #[getter]
    fn prepayment(&self) -> Option<crate::core::money::PyMoney> {
        self.inner
            .prepayment
            .map(|m| crate::core::money::PyMoney { inner: m })
    }

    #[getter]
    fn default_rate(&self) -> Option<crate::core::money::PyMoney> {
        self.inner
            .default_rate
            .map(|m| crate::core::money::PyMoney { inner: m })
    }

    #[getter]
    fn recovery_rate(&self) -> Option<crate::core::money::PyMoney> {
        self.inner
            .recovery_rate
            .map(|m| crate::core::money::PyMoney { inner: m })
    }

    #[getter]
    fn conversion_ratio(&self) -> Option<crate::core::money::PyMoney> {
        self.inner
            .conversion_ratio
            .map(|m| crate::core::money::PyMoney { inner: m })
    }
}

/// Python wrapper for PnlAttribution.
#[pyclass(name = "PnlAttribution")]
#[derive(Clone)]
pub struct PyPnlAttribution {
    pub(crate) inner: PnlAttribution,
}

#[pymethods]
impl PyPnlAttribution {
    #[getter]
    fn total_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.total_pnl,
        }
    }

    #[getter]
    fn carry(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.carry,
        }
    }

    #[getter]
    fn rates_curves_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.rates_curves_pnl,
        }
    }

    #[getter]
    fn credit_curves_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.credit_curves_pnl,
        }
    }

    #[getter]
    fn inflation_curves_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.inflation_curves_pnl,
        }
    }

    #[getter]
    fn correlations_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.correlations_pnl,
        }
    }

    #[getter]
    fn fx_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.fx_pnl,
        }
    }

    #[getter]
    fn vol_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.vol_pnl,
        }
    }

    #[getter]
    fn model_params_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.model_params_pnl,
        }
    }

    #[getter]
    fn market_scalars_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.market_scalars_pnl,
        }
    }

    #[getter]
    fn residual(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.residual,
        }
    }

    #[getter]
    fn meta(&self) -> PyAttributionMeta {
        PyAttributionMeta {
            inner: self.inner.meta.clone(),
        }
    }

    // Detail getters
    #[getter]
    fn rates_detail(&self) -> Option<PyRatesCurvesAttribution> {
        self.inner
            .rates_detail
            .as_ref()
            .map(|d| PyRatesCurvesAttribution { inner: d.clone() })
    }

    #[getter]
    fn credit_detail(&self) -> Option<PyCreditCurvesAttribution> {
        self.inner
            .credit_detail
            .as_ref()
            .map(|d| PyCreditCurvesAttribution { inner: d.clone() })
    }

    #[getter]
    fn model_params_detail(&self) -> Option<PyModelParamsAttribution> {
        self.inner
            .model_params_detail
            .as_ref()
            .map(|d| PyModelParamsAttribution { inner: d.clone() })
    }

    // Export methods
    fn to_csv(&self) -> String {
        self.inner.to_csv()
    }

    fn to_json(&self) -> PyResult<String> {
        self.inner.to_json().map_err(map_error)
    }

    fn rates_detail_to_csv(&self) -> Option<String> {
        self.inner.rates_detail_to_csv()
    }

    fn explain(&self) -> String {
        self.inner.explain()
    }

    fn residual_within_tolerance(&self, pct_tolerance: f64, abs_tolerance: f64) -> bool {
        self.inner
            .residual_within_tolerance(pct_tolerance, abs_tolerance)
    }

    fn __repr__(&self) -> String {
        format!(
            "PnlAttribution(total={}, carry={}, rates={}, residual={})",
            self.inner.total_pnl,
            self.inner.carry,
            self.inner.rates_curves_pnl,
            self.inner.residual
        )
    }
}

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
pub fn attribute_pnl(
    instrument: Bound<'_, PyAny>,
    market_t0: &crate::core::market_data::PyMarketContext,
    market_t1: &crate::core::market_data::PyMarketContext,
    as_of_t0: Bound<'_, PyAny>,
    as_of_t1: Bound<'_, PyAny>,
    method: Option<&PyAttributionMethod>,
    model_params_t0: Option<&Bound<'_, PyAny>>,
) -> PyResult<PyPnlAttribution> {
    // Extract dates
    let date_t0 = py_to_date(&as_of_t0)?;
    let date_t1 = py_to_date(&as_of_t1)?;

    // Extract instrument using existing pattern
    let handle = crate::valuations::instruments::extract_instrument(&instrument)?;
    let instrument_arc: Arc<dyn finstack_valuations::instruments::common::traits::Instrument> =
        Arc::from(handle.instrument);

    // Get attribution method (default to Parallel)
    let method_inner = method
        .map(|m| m.inner.clone())
        .unwrap_or(AttributionMethod::Parallel);

    // Get config
    let config = FinstackConfig::default();
    let model_params_snapshot = parse_model_params_snapshot(model_params_t0)?;
    let model_params_ref = model_params_snapshot.as_ref();

    // Call appropriate Rust function based on method
    let attribution = match method_inner {
        AttributionMethod::Parallel => attribute_pnl_parallel(
            &instrument_arc,
            &market_t0.inner,
            &market_t1.inner,
            date_t0,
            date_t1,
            &config,
            model_params_ref,
        )
        .map_err(map_error)?,

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
        .map_err(map_error)?,

        AttributionMethod::MetricsBased => {
            // Metrics-based requires ValuationResults with risk metrics
            // Include standard risk metrics for P&L attribution (first and second-order)
            let metrics = vec![
                // First-order metrics
                MetricId::Theta,       // Time decay (carry)
                MetricId::Dv01,        // Interest rate sensitivity
                MetricId::Cs01,        // Credit spread sensitivity
                MetricId::Vega,        // Volatility sensitivity
                MetricId::Delta,       // Delta for options/equity
                MetricId::Fx01,        // FX sensitivity
                MetricId::Inflation01, // Inflation sensitivity
                // Second-order metrics (for improved accuracy)
                MetricId::Gamma,              // Spot convexity for options
                MetricId::Convexity,          // Rate convexity for bonds
                MetricId::IrConvexity,        // Rate convexity for IRS
                MetricId::Volga,              // Volatility convexity
                MetricId::Vanna,              // Cross-gamma (spot-vol)
                MetricId::CsGamma,            // Credit spread convexity
                MetricId::InflationConvexity, // Inflation convexity
            ];
            let val_t0 = instrument_arc
                .price_with_metrics(&market_t0.inner, date_t0, &metrics)
                .map_err(map_error)?;

            let val_t1 = instrument_arc
                .price_with_metrics(&market_t1.inner, date_t1, &metrics)
                .map_err(map_error)?;

            attribute_pnl_metrics_based(
                &instrument_arc,
                &market_t0.inner,
                &market_t1.inner,
                &val_t0,
                &val_t1,
                date_t0,
                date_t1,
            )
            .map_err(map_error)?
        }
    };

    Ok(PyPnlAttribution { inner: attribution })
}

/// Python wrapper for PortfolioAttribution.
#[pyclass(name = "PortfolioAttribution")]
#[derive(Clone)]
pub struct PyPortfolioAttribution {
    pub(crate) inner: finstack_portfolio::PortfolioAttribution,
}

#[pymethods]
impl PyPortfolioAttribution {
    #[getter]
    fn total_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.total_pnl,
        }
    }

    #[getter]
    fn carry(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.carry,
        }
    }

    #[getter]
    fn rates_curves_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.rates_curves_pnl,
        }
    }

    #[getter]
    fn credit_curves_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.credit_curves_pnl,
        }
    }

    #[getter]
    fn inflation_curves_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.inflation_curves_pnl,
        }
    }

    #[getter]
    fn correlations_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.correlations_pnl,
        }
    }

    #[getter]
    fn fx_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.fx_pnl,
        }
    }

    #[getter]
    fn vol_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.vol_pnl,
        }
    }

    #[getter]
    fn model_params_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.model_params_pnl,
        }
    }

    #[getter]
    fn market_scalars_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.market_scalars_pnl,
        }
    }

    #[getter]
    fn residual(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.residual,
        }
    }

    fn by_position_to_dict(&self) -> HashMap<String, PyPnlAttribution> {
        self.inner
            .by_position
            .iter()
            .map(|(k, v)| (k.to_string(), PyPnlAttribution { inner: v.clone() }))
            .collect()
    }

    fn to_csv(&self) -> String {
        self.inner.to_csv()
    }

    fn position_detail_to_csv(&self) -> String {
        self.inner.position_detail_to_csv()
    }

    fn explain(&self) -> String {
        self.inner.explain()
    }

    fn __repr__(&self) -> String {
        format!(
            "PortfolioAttribution(total={}, positions={})",
            self.inner.total_pnl,
            self.inner.by_position.len()
        )
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
pub fn attribute_portfolio_pnl(
    portfolio: &crate::portfolio::portfolio::PyPortfolio,
    market_t0: &crate::core::market_data::PyMarketContext,
    market_t1: &crate::core::market_data::PyMarketContext,
    as_of_t0: &Bound<'_, pyo3::PyAny>,
    as_of_t1: &Bound<'_, pyo3::PyAny>,
    method: Option<&PyAttributionMethod>,
) -> PyResult<PyPortfolioAttribution> {
    // Convert Python dates to Rust dates
    let date_t0 = py_to_date(as_of_t0)?;
    let date_t1 = py_to_date(as_of_t1)?;

    // Get attribution method (default to Parallel)
    let method_inner = method
        .map(|m| m.inner.clone())
        .unwrap_or(AttributionMethod::Parallel);

    // Get config
    let config = FinstackConfig::default();

    // Call Rust function
    let attribution = finstack_portfolio::attribute_portfolio_pnl(
        &portfolio.inner,
        &market_t0.inner,
        &market_t1.inner,
        date_t0,
        date_t1,
        &config,
        method_inner,
    )
    .map_err(crate::portfolio::error::portfolio_to_py)?;

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
pub fn attribute_pnl_from_json(spec_json: &str) -> PyResult<PyPnlAttribution> {
    use finstack_valuations::attribution::AttributionEnvelope;

    // Parse the JSON envelope
    let envelope = AttributionEnvelope::from_json(spec_json).map_err(map_error)?;

    // Execute the attribution
    let result_envelope = envelope.execute().map_err(map_error)?;

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
pub fn attribution_result_to_json(attribution: &PyPnlAttribution) -> PyResult<String> {
    use finstack_core::config::results_meta;
    use finstack_valuations::attribution::{AttributionResult, AttributionResultEnvelope};

    let result = AttributionResult {
        attribution: attribution.inner.clone(),
        results_meta: results_meta(&finstack_core::config::FinstackConfig::default()),
    };

    let envelope = AttributionResultEnvelope::new(result);
    envelope.to_json().map_err(map_error)
}

/// Register attribution bindings with Python module.
pub fn register(module: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyAttributionMethod>()?;
    module.add_class::<PyAttributionMeta>()?;
    module.add_class::<PyRatesCurvesAttribution>()?;
    module.add_class::<PyCreditCurvesAttribution>()?;
    module.add_class::<PyModelParamsAttribution>()?;
    module.add_class::<PyPnlAttribution>()?;
    module.add_class::<PyPortfolioAttribution>()?;
    module.add_function(wrap_pyfunction!(attribute_pnl, module)?)?;
    module.add_function(wrap_pyfunction!(attribute_portfolio_pnl, module)?)?;
    module.add_function(wrap_pyfunction!(attribute_pnl_from_json, module)?)?;
    module.add_function(wrap_pyfunction!(attribution_result_to_json, module)?)?;

    Ok(vec![
        "AttributionMethod",
        "AttributionMeta",
        "RatesCurvesAttribution",
        "CreditCurvesAttribution",
        "ModelParamsAttribution",
        "PnlAttribution",
        "PortfolioAttribution",
        "attribute_pnl",
        "attribute_portfolio_pnl",
        "attribute_pnl_from_json",
        "attribution_result_to_json",
    ])
}
