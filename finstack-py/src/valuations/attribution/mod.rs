//! Python bindings for P&L attribution.

use crate::core::currency::extract_currency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::market_data::context::PyMarketContext;
use crate::core::market_data::scalars::{PyMarketScalar, PyScalarTimeSeries};
use crate::core::market_data::surfaces::PyVolSurface;
use crate::core::market_data::term_structures::{
    PyBaseCorrelationCurve, PyDiscountCurve, PyForwardCurve, PyHazardCurve, PyInflationCurve,
};
use crate::errors::core_to_py;
use finstack_core::config::FinstackConfig;
use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use finstack_core::HashMap;
use finstack_valuations::attribution::{
    attribute_pnl_metrics_based, attribute_pnl_parallel, attribute_pnl_taylor,
    attribute_pnl_taylor_compat, attribute_pnl_waterfall, compute_pnl, compute_pnl_with_fx,
    convert_currency, default_waterfall_order, reprice_instrument, AttributionFactor,
    AttributionMeta, AttributionMethod, CarryDetail, CorrelationsAttribution,
    CreditCurvesAttribution, CrossFactorDetail, CurveRestoreFlags, FxAttribution,
    InflationCurvesAttribution, JsonEnvelope, MarketSnapshot, ModelParamsAttribution,
    ModelParamsSnapshot, PnlAttribution, RatesCurvesAttribution, ScalarsAttribution,
    ScalarsSnapshot, TaylorAttributionConfig, TaylorAttributionResult, TaylorFactorResult,
    VolAttribution, VolatilitySnapshot,
};
use indexmap::IndexMap;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyString, PyType};
use pythonize::depythonize;
use serde_json::{self, Value};
use std::sync::Arc;

/// Python wrapper for AttributionMethod.
#[pyclass(name = "AttributionMethod", from_py_object)]
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

        if let Ok(text) = obj.cast::<PyString>() {
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

fn factor_to_label(factor: &AttributionFactor) -> &'static str {
    match factor {
        AttributionFactor::Carry => "carry",
        AttributionFactor::RatesCurves => "rates_curves",
        AttributionFactor::CreditCurves => "credit_curves",
        AttributionFactor::InflationCurves => "inflation_curves",
        AttributionFactor::Correlations => "correlations",
        AttributionFactor::Fx => "fx",
        AttributionFactor::Volatility => "volatility",
        AttributionFactor::ModelParameters => "model_parameters",
        AttributionFactor::MarketScalars => "market_scalars",
    }
}

fn money_map_from_python(
    values: HashMap<String, crate::core::money::PyMoney>,
) -> IndexMap<CurveId, Money> {
    values
        .into_iter()
        .map(|(key, value)| (CurveId::new(key), value.inner))
        .collect()
}

fn money_map_to_python(
    values: &IndexMap<CurveId, Money>,
) -> HashMap<String, crate::core::money::PyMoney> {
    values
        .iter()
        .map(|(key, value)| {
            (
                key.to_string(),
                crate::core::money::PyMoney { inner: *value },
            )
        })
        .collect()
}

fn money_pair_map_to_python(
    values: &IndexMap<(CurveId, String), Money>,
) -> HashMap<(String, String), crate::core::money::PyMoney> {
    values
        .iter()
        .map(|((curve_id, tenor), value)| {
            (
                (curve_id.to_string(), tenor.clone()),
                crate::core::money::PyMoney { inner: *value },
            )
        })
        .collect()
}

fn money_pair_map_from_python(
    values: HashMap<(String, String), crate::core::money::PyMoney>,
) -> IndexMap<(CurveId, String), Money> {
    values
        .into_iter()
        .map(|((curve_id, tenor), value)| ((CurveId::new(curve_id), tenor), value.inner))
        .collect()
}

fn fx_pair_map_from_python(
    values: HashMap<(String, String), crate::core::money::PyMoney>,
) -> PyResult<IndexMap<(Currency, Currency), Money>> {
    values
        .into_iter()
        .map(|((from, to), value)| {
            Ok((
                (
                    Currency::try_from(from.as_str()).map_err(|_| {
                        pyo3::exceptions::PyValueError::new_err(format!(
                            "Unknown currency code in FX attribution: {from}"
                        ))
                    })?,
                    Currency::try_from(to.as_str()).map_err(|_| {
                        pyo3::exceptions::PyValueError::new_err(format!(
                            "Unknown currency code in FX attribution: {to}"
                        ))
                    })?,
                ),
                value.inner,
            ))
        })
        .collect()
}

fn fx_pair_map_to_python(
    values: &IndexMap<(Currency, Currency), Money>,
) -> HashMap<(String, String), crate::core::money::PyMoney> {
    values
        .iter()
        .map(|((from, to), value)| {
            (
                (from.to_string(), to.to_string()),
                crate::core::money::PyMoney { inner: *value },
            )
        })
        .collect()
}

fn wrap_discount_curves(
    values: &HashMap<CurveId, Arc<finstack_core::market_data::term_structures::DiscountCurve>>,
) -> HashMap<String, PyDiscountCurve> {
    values
        .iter()
        .map(|(key, curve)| (key.to_string(), PyDiscountCurve::new_arc(curve.clone())))
        .collect()
}

fn wrap_forward_curves(
    values: &HashMap<CurveId, Arc<finstack_core::market_data::term_structures::ForwardCurve>>,
) -> HashMap<String, PyForwardCurve> {
    values
        .iter()
        .map(|(key, curve)| (key.to_string(), PyForwardCurve::new_arc(curve.clone())))
        .collect()
}

fn wrap_hazard_curves(
    values: &HashMap<CurveId, Arc<finstack_core::market_data::term_structures::HazardCurve>>,
) -> HashMap<String, PyHazardCurve> {
    values
        .iter()
        .map(|(key, curve)| (key.to_string(), PyHazardCurve::new_arc(curve.clone())))
        .collect()
}

fn wrap_inflation_curves(
    values: &HashMap<CurveId, Arc<finstack_core::market_data::term_structures::InflationCurve>>,
) -> HashMap<String, PyInflationCurve> {
    values
        .iter()
        .map(|(key, curve)| (key.to_string(), PyInflationCurve::new_arc(curve.clone())))
        .collect()
}

fn wrap_base_correlation_curves(
    values: &HashMap<
        CurveId,
        Arc<finstack_core::market_data::term_structures::BaseCorrelationCurve>,
    >,
) -> HashMap<String, PyBaseCorrelationCurve> {
    values
        .iter()
        .map(|(key, curve)| {
            (
                key.to_string(),
                PyBaseCorrelationCurve::new_arc(curve.clone()),
            )
        })
        .collect()
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

    #[staticmethod]
    #[pyo3(signature = (config=None))]
    fn taylor(config: Option<&PyTaylorAttributionConfig>) -> Self {
        Self {
            inner: AttributionMethod::Taylor(
                config.map(|value| value.inner.clone()).unwrap_or_default(),
            ),
        }
    }

    fn __repr__(&self) -> String {
        format!("{}", self.inner)
    }
}

/// Python wrapper for AttributionMeta.
#[pyclass(name = "AttributionMeta", from_py_object)]
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
#[pyclass(name = "RatesCurvesAttribution", from_py_object)]
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

    fn by_tenor_to_dict(&self) -> HashMap<(String, String), crate::core::money::PyMoney> {
        self.inner
            .by_tenor
            .iter()
            .map(|((curve_id, tenor), pnl)| {
                (
                    (curve_id.to_string(), tenor.clone()),
                    crate::core::money::PyMoney { inner: *pnl },
                )
            })
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
#[pyclass(name = "CreditCurvesAttribution", from_py_object)]
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

    fn by_tenor_to_dict(&self) -> HashMap<(String, String), crate::core::money::PyMoney> {
        self.inner
            .by_tenor
            .iter()
            .map(|((curve_id, tenor), pnl)| {
                (
                    (curve_id.to_string(), tenor.clone()),
                    crate::core::money::PyMoney { inner: *pnl },
                )
            })
            .collect()
    }
}

/// Python wrapper for ModelParamsAttribution.
#[pyclass(name = "ModelParamsAttribution", from_py_object)]
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

/// Python wrapper for CarryDetail.
#[pyclass(name = "CarryDetail", from_py_object)]
#[derive(Clone)]
pub struct PyCarryDetail {
    pub(crate) inner: CarryDetail,
}

#[pymethods]
impl PyCarryDetail {
    #[new]
    #[pyo3(signature = (total, *, coupon_income=None, pull_to_par=None, roll_down=None, funding_cost=None, theta=None))]
    fn new(
        total: crate::core::money::PyMoney,
        coupon_income: Option<crate::core::money::PyMoney>,
        pull_to_par: Option<crate::core::money::PyMoney>,
        roll_down: Option<crate::core::money::PyMoney>,
        funding_cost: Option<crate::core::money::PyMoney>,
        theta: Option<crate::core::money::PyMoney>,
    ) -> Self {
        Self {
            inner: CarryDetail {
                total: total.inner,
                coupon_income: coupon_income.map(|value| value.inner),
                pull_to_par: pull_to_par.map(|value| value.inner),
                roll_down: roll_down.map(|value| value.inner),
                funding_cost: funding_cost.map(|value| value.inner),
                theta: theta.map(|value| value.inner),
            },
        }
    }

    #[getter]
    fn total(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.total,
        }
    }

    #[getter]
    fn theta(&self) -> Option<crate::core::money::PyMoney> {
        self.inner
            .theta
            .map(|value| crate::core::money::PyMoney { inner: value })
    }

    #[getter]
    fn coupon_income(&self) -> Option<crate::core::money::PyMoney> {
        self.inner
            .coupon_income
            .map(|value| crate::core::money::PyMoney { inner: value })
    }

    #[getter]
    fn pull_to_par(&self) -> Option<crate::core::money::PyMoney> {
        self.inner
            .pull_to_par
            .map(|value| crate::core::money::PyMoney { inner: value })
    }

    #[getter]
    fn roll_down(&self) -> Option<crate::core::money::PyMoney> {
        self.inner
            .roll_down
            .map(|value| crate::core::money::PyMoney { inner: value })
    }

    #[getter]
    fn funding_cost(&self) -> Option<crate::core::money::PyMoney> {
        self.inner
            .funding_cost
            .map(|value| crate::core::money::PyMoney { inner: value })
    }
}

/// Python wrapper for InflationCurvesAttribution.
#[pyclass(name = "InflationCurvesAttribution", from_py_object)]
#[derive(Clone)]
pub struct PyInflationCurvesAttribution {
    pub(crate) inner: InflationCurvesAttribution,
}

#[pymethods]
impl PyInflationCurvesAttribution {
    #[new]
    #[pyo3(signature = (by_curve, *, by_tenor=None))]
    fn new(
        by_curve: HashMap<String, crate::core::money::PyMoney>,
        by_tenor: Option<HashMap<(String, String), crate::core::money::PyMoney>>,
    ) -> Self {
        Self {
            inner: InflationCurvesAttribution {
                by_curve: money_map_from_python(by_curve),
                by_tenor: by_tenor.map(money_pair_map_from_python),
            },
        }
    }

    fn by_curve_to_dict(&self) -> HashMap<String, crate::core::money::PyMoney> {
        money_map_to_python(&self.inner.by_curve)
    }

    fn by_tenor_to_dict(&self) -> Option<HashMap<(String, String), crate::core::money::PyMoney>> {
        self.inner.by_tenor.as_ref().map(money_pair_map_to_python)
    }
}

/// Python wrapper for CorrelationsAttribution.
#[pyclass(name = "CorrelationsAttribution", from_py_object)]
#[derive(Clone)]
pub struct PyCorrelationsAttribution {
    pub(crate) inner: CorrelationsAttribution,
}

#[pymethods]
impl PyCorrelationsAttribution {
    #[new]
    fn new(by_curve: HashMap<String, crate::core::money::PyMoney>) -> Self {
        Self {
            inner: CorrelationsAttribution {
                by_curve: money_map_from_python(by_curve),
            },
        }
    }

    fn by_curve_to_dict(&self) -> HashMap<String, crate::core::money::PyMoney> {
        money_map_to_python(&self.inner.by_curve)
    }
}

/// Python wrapper for FxAttribution.
#[pyclass(name = "FxAttribution", from_py_object)]
#[derive(Clone)]
pub struct PyFxAttribution {
    pub(crate) inner: FxAttribution,
}

#[pymethods]
impl PyFxAttribution {
    #[new]
    fn new(by_pair: HashMap<(String, String), crate::core::money::PyMoney>) -> PyResult<Self> {
        Ok(Self {
            inner: FxAttribution {
                by_pair: fx_pair_map_from_python(by_pair)?,
            },
        })
    }

    fn by_pair_to_dict(&self) -> HashMap<(String, String), crate::core::money::PyMoney> {
        fx_pair_map_to_python(&self.inner.by_pair)
    }
}

/// Python wrapper for VolAttribution.
#[pyclass(name = "VolAttribution", from_py_object)]
#[derive(Clone)]
pub struct PyVolAttribution {
    pub(crate) inner: VolAttribution,
}

#[pymethods]
impl PyVolAttribution {
    #[new]
    fn new(by_surface: HashMap<String, crate::core::money::PyMoney>) -> Self {
        Self {
            inner: VolAttribution {
                by_surface: money_map_from_python(by_surface),
            },
        }
    }

    fn by_surface_to_dict(&self) -> HashMap<String, crate::core::money::PyMoney> {
        money_map_to_python(&self.inner.by_surface)
    }
}

/// Python wrapper for ScalarsAttribution.
#[pyclass(name = "ScalarsAttribution", from_py_object)]
#[derive(Clone)]
pub struct PyScalarsAttribution {
    pub(crate) inner: ScalarsAttribution,
}

#[pymethods]
impl PyScalarsAttribution {
    #[new]
    #[pyo3(signature = (*, dividends=None, inflation=None, equity_prices=None, commodity_prices=None))]
    fn new(
        dividends: Option<HashMap<String, crate::core::money::PyMoney>>,
        inflation: Option<HashMap<String, crate::core::money::PyMoney>>,
        equity_prices: Option<HashMap<String, crate::core::money::PyMoney>>,
        commodity_prices: Option<HashMap<String, crate::core::money::PyMoney>>,
    ) -> Self {
        Self {
            inner: ScalarsAttribution {
                dividends: dividends.map(money_map_from_python).unwrap_or_default(),
                inflation: inflation.map(money_map_from_python).unwrap_or_default(),
                equity_prices: equity_prices.map(money_map_from_python).unwrap_or_default(),
                commodity_prices: commodity_prices
                    .map(money_map_from_python)
                    .unwrap_or_default(),
            },
        }
    }

    fn dividends_to_dict(&self) -> HashMap<String, crate::core::money::PyMoney> {
        money_map_to_python(&self.inner.dividends)
    }

    fn inflation_to_dict(&self) -> HashMap<String, crate::core::money::PyMoney> {
        money_map_to_python(&self.inner.inflation)
    }

    fn equity_prices_to_dict(&self) -> HashMap<String, crate::core::money::PyMoney> {
        money_map_to_python(&self.inner.equity_prices)
    }

    fn commodity_prices_to_dict(&self) -> HashMap<String, crate::core::money::PyMoney> {
        money_map_to_python(&self.inner.commodity_prices)
    }
}

/// Python wrapper for CrossFactorDetail.
#[pyclass(name = "CrossFactorDetail", from_py_object)]
#[derive(Clone)]
pub struct PyCrossFactorDetail {
    pub(crate) inner: CrossFactorDetail,
}

#[pymethods]
impl PyCrossFactorDetail {
    #[new]
    fn new(
        total: crate::core::money::PyMoney,
        by_pair: HashMap<String, crate::core::money::PyMoney>,
    ) -> Self {
        Self {
            inner: CrossFactorDetail {
                total: total.inner,
                by_pair: by_pair.into_iter().map(|(k, v)| (k, v.inner)).collect(),
            },
        }
    }

    #[getter]
    fn total(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.total,
        }
    }

    fn by_pair_to_dict(&self) -> HashMap<String, crate::core::money::PyMoney> {
        self.inner
            .by_pair
            .iter()
            .map(|(k, v)| (k.clone(), crate::core::money::PyMoney { inner: *v }))
            .collect()
    }
}

/// Python wrapper for TaylorAttributionConfig.
#[pyclass(name = "TaylorAttributionConfig", from_py_object)]
#[derive(Clone)]
pub struct PyTaylorAttributionConfig {
    pub(crate) inner: TaylorAttributionConfig,
}

#[pymethods]
impl PyTaylorAttributionConfig {
    #[new]
    #[pyo3(signature = (*, include_gamma=false, rate_bump_bp=1.0, credit_bump_bp=1.0, vol_bump=0.01))]
    fn new(include_gamma: bool, rate_bump_bp: f64, credit_bump_bp: f64, vol_bump: f64) -> Self {
        Self {
            inner: TaylorAttributionConfig {
                include_gamma,
                rate_bump_bp,
                credit_bump_bp,
                vol_bump,
            },
        }
    }

    #[getter]
    fn include_gamma(&self) -> bool {
        self.inner.include_gamma
    }

    #[getter]
    fn rate_bump_bp(&self) -> f64 {
        self.inner.rate_bump_bp
    }

    #[getter]
    fn credit_bump_bp(&self) -> f64 {
        self.inner.credit_bump_bp
    }

    #[getter]
    fn vol_bump(&self) -> f64 {
        self.inner.vol_bump
    }
}

/// Python wrapper for TaylorFactorResult.
#[pyclass(name = "TaylorFactorResult", from_py_object)]
#[derive(Clone)]
pub struct PyTaylorFactorResult {
    pub(crate) inner: TaylorFactorResult,
}

#[pymethods]
impl PyTaylorFactorResult {
    #[new]
    #[pyo3(signature = (factor_name, sensitivity, market_move, explained_pnl, *, gamma_pnl=None))]
    fn new(
        factor_name: String,
        sensitivity: f64,
        market_move: f64,
        explained_pnl: f64,
        gamma_pnl: Option<f64>,
    ) -> Self {
        Self {
            inner: TaylorFactorResult {
                factor_name,
                sensitivity,
                market_move,
                explained_pnl,
                gamma_pnl,
            },
        }
    }

    #[getter]
    fn factor_name(&self) -> &str {
        &self.inner.factor_name
    }

    #[getter]
    fn sensitivity(&self) -> f64 {
        self.inner.sensitivity
    }

    #[getter]
    fn market_move(&self) -> f64 {
        self.inner.market_move
    }

    #[getter]
    fn explained_pnl(&self) -> f64 {
        self.inner.explained_pnl
    }

    #[getter]
    fn gamma_pnl(&self) -> Option<f64> {
        self.inner.gamma_pnl
    }
}

/// Python wrapper for TaylorAttributionResult.
#[pyclass(name = "TaylorAttributionResult", from_py_object)]
#[derive(Clone)]
pub struct PyTaylorAttributionResult {
    pub(crate) inner: TaylorAttributionResult,
}

#[pymethods]
impl PyTaylorAttributionResult {
    #[new]
    #[pyo3(signature = (actual_pnl, total_explained, unexplained, unexplained_pct, factors, num_repricings, pv_t0, pv_t1))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        actual_pnl: f64,
        total_explained: f64,
        unexplained: f64,
        unexplained_pct: f64,
        factors: Vec<PyTaylorFactorResult>,
        num_repricings: usize,
        pv_t0: crate::core::money::PyMoney,
        pv_t1: crate::core::money::PyMoney,
    ) -> Self {
        Self {
            inner: TaylorAttributionResult {
                actual_pnl,
                total_explained,
                unexplained,
                unexplained_pct,
                factors: factors.into_iter().map(|value| value.inner).collect(),
                num_repricings,
                pv_t0: pv_t0.inner,
                pv_t1: pv_t1.inner,
            },
        }
    }

    #[getter]
    fn actual_pnl(&self) -> f64 {
        self.inner.actual_pnl
    }

    #[getter]
    fn total_explained(&self) -> f64 {
        self.inner.total_explained
    }

    #[getter]
    fn unexplained(&self) -> f64 {
        self.inner.unexplained
    }

    #[getter]
    fn unexplained_pct(&self) -> f64 {
        self.inner.unexplained_pct
    }

    #[getter]
    fn factors(&self) -> Vec<PyTaylorFactorResult> {
        self.inner
            .factors
            .iter()
            .cloned()
            .map(|inner| PyTaylorFactorResult { inner })
            .collect()
    }

    #[getter]
    fn num_repricings(&self) -> usize {
        self.inner.num_repricings
    }

    #[getter]
    fn pv_t0(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.pv_t0,
        }
    }

    #[getter]
    fn pv_t1(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.pv_t1,
        }
    }
}

/// Python wrapper for CurveRestoreFlags.
#[pyclass(name = "CurveRestoreFlags", frozen, from_py_object)]
#[derive(Clone, Copy)]
pub struct PyCurveRestoreFlags {
    pub(crate) inner: CurveRestoreFlags,
}

#[pymethods]
impl PyCurveRestoreFlags {
    #[classattr]
    const DISCOUNT: Self = Self {
        inner: CurveRestoreFlags::DISCOUNT,
    };

    #[classattr]
    const FORWARD: Self = Self {
        inner: CurveRestoreFlags::FORWARD,
    };

    #[classattr]
    const HAZARD: Self = Self {
        inner: CurveRestoreFlags::HAZARD,
    };

    #[classattr]
    const INFLATION: Self = Self {
        inner: CurveRestoreFlags::INFLATION,
    };

    #[classattr]
    const CORRELATION: Self = Self {
        inner: CurveRestoreFlags::CORRELATION,
    };

    #[classattr]
    const RATES: Self = Self {
        inner: CurveRestoreFlags::RATES,
    };

    #[classattr]
    const CREDIT: Self = Self {
        inner: CurveRestoreFlags::CREDIT,
    };

    #[classmethod]
    fn all(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: CurveRestoreFlags::all(),
        }
    }

    #[classmethod]
    fn empty(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: CurveRestoreFlags::empty(),
        }
    }

    fn contains(&self, other: &PyCurveRestoreFlags) -> bool {
        self.inner.contains(other.inner)
    }

    fn __or__(&self, other: &PyCurveRestoreFlags) -> Self {
        Self {
            inner: self.inner | other.inner,
        }
    }

    fn __and__(&self, other: &PyCurveRestoreFlags) -> Self {
        Self {
            inner: self.inner & other.inner,
        }
    }

    fn __invert__(&self) -> Self {
        Self { inner: !self.inner }
    }
}

/// Python wrapper for MarketSnapshot.
#[pyclass(name = "MarketSnapshot", from_py_object)]
#[derive(Clone, Default)]
pub struct PyMarketSnapshot {
    pub(crate) inner: MarketSnapshot,
}

#[pymethods]
impl PyMarketSnapshot {
    #[classmethod]
    fn extract(
        _cls: &Bound<'_, PyType>,
        market: &PyMarketContext,
        flags: &PyCurveRestoreFlags,
    ) -> Self {
        Self {
            inner: MarketSnapshot::extract(&market.inner, flags.inner),
        }
    }

    #[staticmethod]
    fn restore_market(
        current_market: &PyMarketContext,
        snapshot: &PyMarketSnapshot,
        restore_flags: &PyCurveRestoreFlags,
    ) -> PyMarketContext {
        PyMarketContext {
            inner: MarketSnapshot::restore_market(
                &current_market.inner,
                &snapshot.inner,
                restore_flags.inner,
            ),
        }
    }

    fn discount_curves(&self) -> HashMap<String, PyDiscountCurve> {
        wrap_discount_curves(&self.inner.discount_curves)
    }

    fn forward_curves(&self) -> HashMap<String, PyForwardCurve> {
        wrap_forward_curves(&self.inner.forward_curves)
    }

    fn hazard_curves(&self) -> HashMap<String, PyHazardCurve> {
        wrap_hazard_curves(&self.inner.hazard_curves)
    }

    fn inflation_curves(&self) -> HashMap<String, PyInflationCurve> {
        wrap_inflation_curves(&self.inner.inflation_curves)
    }

    fn base_correlation_curves(&self) -> HashMap<String, PyBaseCorrelationCurve> {
        wrap_base_correlation_curves(&self.inner.base_correlation_curves)
    }
}

/// Python wrapper for VolatilitySnapshot.
#[pyclass(name = "VolatilitySnapshot", from_py_object)]
#[derive(Clone)]
pub struct PyVolatilitySnapshot {
    pub(crate) inner: VolatilitySnapshot,
}

#[pymethods]
impl PyVolatilitySnapshot {
    #[classmethod]
    fn extract(_cls: &Bound<'_, PyType>, market: &PyMarketContext) -> Self {
        Self {
            inner: VolatilitySnapshot::extract(&market.inner),
        }
    }

    fn surfaces(&self) -> HashMap<String, PyVolSurface> {
        self.inner
            .surfaces
            .iter()
            .map(|(key, value)| (key.to_string(), PyVolSurface::new_arc(value.clone())))
            .collect()
    }
}

/// Python wrapper for ScalarsSnapshot.
#[pyclass(name = "ScalarsSnapshot", from_py_object)]
#[derive(Clone)]
pub struct PyScalarsSnapshot {
    pub(crate) inner: ScalarsSnapshot,
}

#[pymethods]
impl PyScalarsSnapshot {
    #[classmethod]
    fn extract(_cls: &Bound<'_, PyType>, market: &PyMarketContext) -> Self {
        Self {
            inner: ScalarsSnapshot::extract(&market.inner),
        }
    }

    fn prices(&self) -> HashMap<String, PyMarketScalar> {
        self.inner
            .prices
            .iter()
            .map(|(key, value)| (key.to_string(), PyMarketScalar::new(value.clone())))
            .collect()
    }

    fn series(&self) -> HashMap<String, PyScalarTimeSeries> {
        self.inner
            .series
            .iter()
            .map(|(key, value)| (key.to_string(), PyScalarTimeSeries::new_arc(value.clone())))
            .collect()
    }
}

/// Python wrapper for PnlAttribution.
#[pyclass(name = "PnlAttribution", from_py_object)]
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

    #[getter]
    fn carry_detail(&self) -> Option<PyCarryDetail> {
        self.inner
            .carry_detail
            .as_ref()
            .map(|d| PyCarryDetail { inner: d.clone() })
    }

    #[getter]
    fn inflation_detail(&self) -> Option<PyInflationCurvesAttribution> {
        self.inner
            .inflation_detail
            .as_ref()
            .map(|d| PyInflationCurvesAttribution { inner: d.clone() })
    }

    #[getter]
    fn correlations_detail(&self) -> Option<PyCorrelationsAttribution> {
        self.inner
            .correlations_detail
            .as_ref()
            .map(|d| PyCorrelationsAttribution { inner: d.clone() })
    }

    #[getter]
    fn fx_detail(&self) -> Option<PyFxAttribution> {
        self.inner
            .fx_detail
            .as_ref()
            .map(|d| PyFxAttribution { inner: d.clone() })
    }

    #[getter]
    fn vol_detail(&self) -> Option<PyVolAttribution> {
        self.inner
            .vol_detail
            .as_ref()
            .map(|d| PyVolAttribution { inner: d.clone() })
    }

    #[getter]
    fn cross_factor_pnl(&self) -> crate::core::money::PyMoney {
        crate::core::money::PyMoney {
            inner: self.inner.cross_factor_pnl,
        }
    }

    #[getter]
    fn cross_factor_detail(&self) -> Option<PyCrossFactorDetail> {
        self.inner
            .cross_factor_detail
            .as_ref()
            .map(|d| PyCrossFactorDetail { inner: d.clone() })
    }

    #[getter]
    fn scalars_detail(&self) -> Option<PyScalarsAttribution> {
        self.inner
            .scalars_detail
            .as_ref()
            .map(|d| PyScalarsAttribution { inner: d.clone() })
    }

    // Export methods
    fn to_csv(&self) -> String {
        self.inner.to_csv()
    }

    fn to_json(&self) -> PyResult<String> {
        self.inner.to_json().map_err(core_to_py)
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
#[allow(clippy::too_many_arguments)]
pub fn attribute_pnl(
    py: Python<'_>,
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
    let instrument_arc: Arc<dyn finstack_valuations::instruments::Instrument> = handle.instrument;

    // Get attribution method (default to Parallel)
    let method_inner = method
        .map(|m| m.inner.clone())
        .unwrap_or(AttributionMethod::Parallel);

    // Get config
    let config = FinstackConfig::default();
    let model_params_snapshot = parse_model_params_snapshot(model_params_t0)?;
    let model_params_ref = model_params_snapshot.as_ref();

    // Call appropriate Rust function based on method; release GIL for heavy computation
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
                .price_with_metrics(&market_t0.inner, date_t0, &metrics)
                .map_err(core_to_py)?;

            let val_t1 = instrument_arc
                .price_with_metrics(&market_t1.inner, date_t1, &metrics)
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
        AttributionMethod::Taylor(config_inner) => attribute_pnl_taylor_compat(
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
pub fn attribute_pnl_taylor_py(
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
    let instrument_arc: Arc<dyn finstack_valuations::instruments::Instrument> = handle.instrument;
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
fn reprice_instrument_py(
    instrument: Bound<'_, PyAny>,
    market: &PyMarketContext,
    as_of: Bound<'_, PyAny>,
) -> PyResult<crate::core::money::PyMoney> {
    let date = py_to_date(&as_of)?;
    let handle = crate::valuations::instruments::extract_instrument(&instrument)?;
    let instrument_arc: Arc<dyn finstack_valuations::instruments::Instrument> = handle.instrument;
    let value = reprice_instrument(&instrument_arc, &market.inner, date).map_err(core_to_py)?;
    Ok(crate::core::money::PyMoney { inner: value })
}

/// Convert money into a target currency using a market FX matrix.
#[pyfunction(name = "convert_currency")]
fn convert_currency_py(
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
fn compute_pnl_py(
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
fn compute_pnl_with_fx_py(
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
fn default_waterfall_order_py() -> Vec<String> {
    default_waterfall_order()
        .into_iter()
        .map(|factor| factor_to_label(&factor).to_string())
        .collect()
}

/// Python wrapper for PortfolioAttribution.
#[pyclass(name = "PortfolioAttribution", from_py_object)]
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
    py: Python<'_>,
    portfolio: &crate::portfolio::positions::PyPortfolio,
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

    // Call Rust function; release GIL for heavy computation
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
pub fn attribute_pnl_from_json(spec_json: &str) -> PyResult<PyPnlAttribution> {
    use finstack_valuations::attribution::AttributionEnvelope;

    // Parse the JSON envelope
    let envelope = AttributionEnvelope::from_json(spec_json).map_err(core_to_py)?;

    // Execute the attribution
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
pub fn attribution_result_to_json(attribution: &PyPnlAttribution) -> PyResult<String> {
    use finstack_core::config::results_meta;
    use finstack_valuations::attribution::{AttributionResult, AttributionResultEnvelope};

    let result = AttributionResult {
        attribution: attribution.inner.clone(),
        results_meta: results_meta(&finstack_core::config::FinstackConfig::default()),
    };

    let envelope = AttributionResultEnvelope::new(result);
    envelope.to_json().map_err(core_to_py)
}

/// Register attribution bindings with Python module.
pub fn register(module: &Bound<'_, PyModule>) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyAttributionMethod>()?;
    module.add_class::<PyAttributionMeta>()?;
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

    let exports = vec![
        "AttributionMethod",
        "AttributionMeta",
        "RatesCurvesAttribution",
        "CreditCurvesAttribution",
        "ModelParamsAttribution",
        "CarryDetail",
        "InflationCurvesAttribution",
        "CorrelationsAttribution",
        "FxAttribution",
        "VolAttribution",
        "ScalarsAttribution",
        "TaylorAttributionConfig",
        "TaylorFactorResult",
        "TaylorAttributionResult",
        "CurveRestoreFlags",
        "MarketSnapshot",
        "VolatilitySnapshot",
        "ScalarsSnapshot",
        "PnlAttribution",
        "PortfolioAttribution",
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
    ];
    let py = module.py();
    module.setattr("__doc__", "P&L attribution for instruments and portfolios.")?;
    module.setattr("__all__", PyList::new(py, &exports)?)?;

    Ok(exports)
}
