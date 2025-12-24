use crate::core::dates::periods::PyPeriodId;
use crate::core::dates::utils::date_to_py;
use crate::statements::evaluator::PyResults;
use crate::statements::types::model::PyFinancialModelSpec;
use finstack_core::dates::{Date, PeriodId};
use finstack_valuations::covenants::engine::{
    Covenant, CovenantScope, CovenantSpec, CovenantType, SpringingCondition, ThresholdTest,
};
use finstack_valuations::covenants::forward::{
    CovenantForecast as ValCovForecast, CovenantForecastConfig as ValCovForecastConfig,
    McConfig as ValMcConfig, ModelTimeSeries,
};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;
use time::Month;

#[pyclass(
    module = "finstack.valuations.covenants",
    name = "CovenantType",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyCovenantType {
    pub(crate) inner: CovenantType,
}

#[pymethods]
impl PyCovenantType {
    #[classattr]
    const __DOC__: &'static str = "Covenant type variants for common financial covenants.";

    #[pyo3(text_signature = "(threshold)")]
    #[staticmethod]
    pub fn max_debt_to_ebitda(threshold: f64) -> Self {
        Self {
            inner: CovenantType::MaxDebtToEBITDA { threshold },
        }
    }

    #[pyo3(text_signature = "(threshold)")]
    #[staticmethod]
    pub fn min_interest_coverage(threshold: f64) -> Self {
        Self {
            inner: CovenantType::MinInterestCoverage { threshold },
        }
    }

    #[pyo3(text_signature = "(threshold)")]
    #[staticmethod]
    pub fn min_fixed_charge_coverage(threshold: f64) -> Self {
        Self {
            inner: CovenantType::MinFixedChargeCoverage { threshold },
        }
    }

    #[pyo3(text_signature = "(threshold)")]
    #[staticmethod]
    pub fn max_total_leverage(threshold: f64) -> Self {
        Self {
            inner: CovenantType::MaxTotalLeverage { threshold },
        }
    }

    #[pyo3(text_signature = "(threshold)")]
    #[staticmethod]
    pub fn max_senior_leverage(threshold: f64) -> Self {
        Self {
            inner: CovenantType::MaxSeniorLeverage { threshold },
        }
    }

    #[pyo3(text_signature = "(metric, limit)")]
    #[staticmethod]
    pub fn basket(metric: &str, limit: f64) -> Self {
        Self {
            inner: CovenantType::Basket {
                name: metric.to_string(),
                limit,
            },
        }
    }

    #[pyo3(text_signature = "(metric, comparator, threshold)")]
    #[staticmethod]
    pub fn custom(metric: &str, comparator: &str, threshold: f64) -> PyResult<Self> {
        let test = match comparator.to_ascii_lowercase().as_str() {
            "maximum" | "le" | "lte" | "<=" => ThresholdTest::Maximum(threshold),
            "minimum" | "ge" | "gte" | ">=" => ThresholdTest::Minimum(threshold),
            other => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Unknown comparator: {} (expected 'maximum' or 'minimum')",
                    other
                )))
            }
        };
        Ok(Self {
            inner: CovenantType::Custom {
                metric: metric.to_string(),
                test,
            },
        })
    }
}

#[pyclass(module = "finstack.valuations.covenants", name = "Covenant", frozen)]
#[derive(Clone, Debug)]
pub struct PyCovenant {
    pub(crate) inner: Covenant,
}

#[pymethods]
impl PyCovenant {
    #[new]
    #[pyo3(text_signature = "(ctype)")]
    pub fn new(ctype: &PyCovenantType) -> Self {
        Self {
            inner: Covenant::new(
                ctype.inner.clone(),
                finstack_core::dates::Tenor::quarterly(),
            ),
        }
    }

    #[pyo3(text_signature = "(scope)")]
    pub fn with_scope(&self, scope: &PyCovenantScope) -> Self {
        let mut inner = self.inner.clone();
        inner.scope = scope.inner.clone();
        Self { inner }
    }

    #[pyo3(text_signature = "(condition)")]
    pub fn with_springing_condition(&self, condition: &PySpringingCondition) -> Self {
        let mut inner = self.inner.clone();
        inner.springing_condition = Some(condition.inner.clone());
        Self { inner }
    }
}

#[pyclass(
    module = "finstack.valuations.covenants",
    name = "CovenantSpec",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyCovenantSpec {
    pub(crate) inner: CovenantSpec,
}

#[pymethods]
impl PyCovenantSpec {
    #[pyo3(text_signature = "(covenant, metric_id)")]
    #[staticmethod]
    pub fn with_metric(covenant: &PyCovenant, metric_id: &str) -> Self {
        Self {
            inner: CovenantSpec::with_metric(
                covenant.inner.clone(),
                finstack_valuations::metrics::MetricId::custom(metric_id),
            ),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.covenants",
    name = "CovenantScope",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyCovenantScope {
    pub(crate) inner: CovenantScope,
}

#[pymethods]
impl PyCovenantScope {
    #[staticmethod]
    pub fn maintenance() -> Self {
        Self {
            inner: CovenantScope::Maintenance,
        }
    }

    #[staticmethod]
    pub fn incurrence() -> Self {
        Self {
            inner: CovenantScope::Incurrence,
        }
    }

    fn __repr__(&self) -> &'static str {
        match self.inner {
            CovenantScope::Maintenance => "<CovenantScope.Maintenance>",
            CovenantScope::Incurrence => "<CovenantScope.Incurrence>",
        }
    }
}

#[pyclass(
    module = "finstack.valuations.covenants",
    name = "SpringingCondition",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PySpringingCondition {
    pub(crate) inner: SpringingCondition,
}

#[pymethods]
impl PySpringingCondition {
    #[new]
    #[pyo3(text_signature = "(metric_id, comparator, threshold)")]
    pub fn new(metric_id: &str, comparator: &str, threshold: f64) -> PyResult<Self> {
        let test = match comparator.to_ascii_lowercase().as_str() {
            "maximum" | "le" | "lte" | "<=" => ThresholdTest::Maximum(threshold),
            "minimum" | "ge" | "gte" | ">=" => ThresholdTest::Minimum(threshold),
            other => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Unknown comparator: {} (expected 'maximum' or 'minimum')",
                    other
                )))
            }
        };
        Ok(Self {
            inner: SpringingCondition {
                metric_id: finstack_valuations::metrics::MetricId::custom(metric_id),
                test,
            },
        })
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "<SpringingCondition metric='{}'>",
            self.inner.metric_id.as_str()
        ))
    }
}

#[pyclass(
    module = "finstack.valuations.covenants",
    name = "CovenantForecastConfig",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyCovenantForecastConfig {
    pub(crate) inner: ValCovForecastConfig,
}

#[pymethods]
impl PyCovenantForecastConfig {
    #[new]
    #[pyo3(
        text_signature = "(stochastic=False, num_paths=0, volatility=None, seed=None, antithetic=False)",
        signature = (
            stochastic = None,
            num_paths = None,
            volatility = None,
            seed = None,
            antithetic = None
        )
    )]
    pub fn new(
        stochastic: Option<bool>,
        num_paths: Option<usize>,
        volatility: Option<f64>,
        seed: Option<u64>,
        antithetic: Option<bool>,
    ) -> Self {
        let mut cfg = ValCovForecastConfig::default();
        cfg.stochastic = stochastic.unwrap_or(false);
        cfg.num_paths = num_paths.unwrap_or(0);
        cfg.volatility = volatility;
        cfg.random_seed = seed;
        if antithetic.unwrap_or(false) {
            cfg.mc = Some(ValMcConfig { antithetic: true });
        }
        Self { inner: cfg }
    }
}

#[pyclass(
    module = "finstack.valuations.covenants",
    name = "CovenantForecast",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyCovenantForecast {
    pub(crate) inner: ValCovForecast,
}

#[pymethods]
impl PyCovenantForecast {
    #[getter]
    fn covenant_id(&self) -> &str {
        &self.inner.covenant_id
    }

    #[getter]
    fn test_dates(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let list = PyList::empty(py);
        for d in &self.inner.test_dates {
            list.append(date_to_py(py, *d)?)?;
        }
        Ok(list.into())
    }

    #[getter]
    fn projected_values(&self) -> Vec<f64> {
        self.inner.projected_values.clone()
    }
    #[getter]
    fn thresholds(&self) -> Vec<f64> {
        self.inner.thresholds.clone()
    }
    #[getter]
    fn headroom(&self) -> Vec<f64> {
        self.inner.headroom.clone()
    }
    #[getter]
    fn breach_probability(&self) -> Vec<f64> {
        self.inner.breach_probability.clone()
    }

    #[getter]
    fn first_breach_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match self.inner.first_breach_date {
            Some(d) => date_to_py(py, d),
            None => Ok(py.None()),
        }
    }

    #[getter]
    fn min_headroom_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.min_headroom_date)
    }

    #[getter]
    fn min_headroom_value(&self) -> f64 {
        self.inner.min_headroom_value
    }

    /// Human-readable multi-period explanation.
    fn explain(&self) -> String {
        self.inner.explain()
    }

    /// Export to Polars DataFrame (requires polars and pyo3-polars).
    fn to_polars(&self) -> PyResult<pyo3_polars::PyDataFrame> {
        use polars::prelude::*;
        let dates: Vec<String> = self
            .inner
            .test_dates
            .iter()
            .map(|d| d.to_string())
            .collect();
        let df = df![
            "test_date" => dates,
            "projected_value" => self.inner.projected_values.clone(),
            "threshold" => self.inner.thresholds.clone(),
            "headroom" => self.inner.headroom.clone(),
            "breach_prob" => self.inner.breach_probability.clone()
        ]
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(pyo3_polars::PyDataFrame(df))
    }
}

struct StatementsAdapter<'a> {
    model: &'a finstack_statements::types::FinancialModelSpec,
    results: &'a finstack_statements::evaluator::Results,
}

impl<'a> StatementsAdapter<'a> {
    fn new(
        model: &'a finstack_statements::types::FinancialModelSpec,
        results: &'a finstack_statements::evaluator::Results,
    ) -> Self {
        Self { model, results }
    }
}

impl<'a> ModelTimeSeries for StatementsAdapter<'a> {
    fn get_scalar(&self, node_id: &str, period: &PeriodId) -> Option<f64> {
        self.results.get(node_id, period)
    }
    fn period_end_date(&self, period: &PeriodId) -> Date {
        for p in &self.model.periods {
            if p.id == *period {
                return p.end;
            }
        }
        Date::from_calendar_date(period.year, Month::December, 31).unwrap()
    }
}

#[pyfunction(name = "forecast_covenant")]
pub fn py_forecast_covenant(
    covenant_spec: &PyCovenantSpec,
    model: &PyFinancialModelSpec,
    base_case: &PyResults,
    periods: Vec<PyPeriodId>,
    config: Option<&PyCovenantForecastConfig>,
) -> PyResult<PyCovenantForecast> {
    let adapter = StatementsAdapter::new(&model.inner, &base_case.inner);
    let ps: Vec<PeriodId> = periods.into_iter().map(|p| p.inner).collect();
    let cfg = config.map(|c| c.inner.clone()).unwrap_or_default();
    finstack_valuations::covenants::forward::forecast_covenant_generic(
        &covenant_spec.inner,
        &adapter,
        &ps,
        cfg,
    )
    .map(|inner| PyCovenantForecast { inner })
    .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

#[pyclass(
    module = "finstack.valuations.covenants",
    name = "FutureBreach",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyFutureBreach {
    pub(crate) inner: finstack_valuations::covenants::forward::FutureBreach,
}

#[pymethods]
impl PyFutureBreach {
    #[getter]
    fn covenant_id(&self) -> &str {
        &self.inner.covenant_id
    }

    #[getter]
    fn breach_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.breach_date)
    }

    #[getter]
    fn projected_value(&self) -> f64 {
        self.inner.projected_value
    }

    #[getter]
    fn threshold(&self) -> f64 {
        self.inner.threshold
    }

    #[getter]
    fn headroom(&self) -> f64 {
        self.inner.headroom
    }

    #[getter]
    fn breach_probability(&self) -> f64 {
        self.inner.breach_probability
    }

    fn __repr__(&self) -> String {
        format!(
            "<FutureBreach id='{}' date='{}' val={:.2} thr={:.2} prob={:.1}%>",
            self.inner.covenant_id,
            self.inner.breach_date,
            self.inner.projected_value,
            self.inner.threshold,
            self.inner.breach_probability * 100.0
        )
    }
}

#[pyfunction(name = "forecast_breaches")]
pub fn py_forecast_breaches(
    specs: Vec<PyCovenantSpec>,
    model: &PyFinancialModelSpec,
    base_case: &PyResults,
    config: Option<&PyCovenantForecastConfig>,
) -> PyResult<Vec<PyFutureBreach>> {
    let mut engine = finstack_valuations::covenants::engine::CovenantEngine::new();
    for spec in specs {
        engine.add_spec(spec.inner.clone());
    }

    let _adapter = StatementsAdapter::new(&model.inner, &base_case.inner);

    // We need periods. The Rust `forecast_breaches` extracts them from results.
    // Using `finstack_statements::analysis::covenants::forecast_breaches` which takes `&Results` and `&CovenantEngine`.

    let cfg = config.map(|c| c.inner.clone()).unwrap_or_default();

    finstack_statements::analysis::covenants::forecast_breaches(
        &base_case.inner,
        &engine,
        Some(&model.inner),
        cfg,
    )
    .map(|breaches| {
        breaches
            .into_iter()
            .map(|b| PyFutureBreach { inner: b })
            .collect()
    })
    .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "covenants")?;
    module.setattr(
        "__doc__",
        "Covenant forward-projection bindings: spec types, config, and forecast functions.",
    )?;
    module.add_class::<PyCovenantType>()?;
    module.add_class::<PyCovenant>()?;
    module.add_class::<PyCovenantSpec>()?;
    module.add_class::<PyCovenantScope>()?;
    module.add_class::<PySpringingCondition>()?;
    module.add_class::<PyCovenantForecastConfig>()?;
    module.add_class::<PyCovenantForecast>()?;
    module.add_class::<PyFutureBreach>()?;
    module.add_function(pyo3::wrap_pyfunction!(py_forecast_covenant, &module)?)?;
    module.add_function(pyo3::wrap_pyfunction!(py_forecast_breaches, &module)?)?;
    let exports = [
        "CovenantType",
        "Covenant",
        "CovenantSpec",
        "CovenantScope",
        "SpringingCondition",
        "CovenantForecastConfig",
        "CovenantForecast",
        "FutureBreach",
        "forecast_covenant",
        "forecast_breaches",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports.to_vec())
}
