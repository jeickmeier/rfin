use super::helpers::{
    fx_pair_map_from_python, fx_pair_map_to_python, money_map_from_python, money_map_to_python,
    money_pair_map_from_python, money_pair_map_to_python,
};
use super::taylor::PyTaylorAttributionConfig;
use crate::core::dates::utils::date_to_py;
use finstack_core::HashMap;
use finstack_valuations::attribution::{
    AttributionFactor, AttributionMeta, AttributionMethod, CarryDetail, CorrelationsAttribution,
    CreditCurvesAttribution, CrossFactorDetail, FxAttribution, InflationCurvesAttribution,
    ModelParamsAttribution, RatesCurvesAttribution, ScalarsAttribution, VolAttribution,
};
use pyo3::prelude::*;

/// Python wrapper for AttributionMethod.
#[pyclass(name = "AttributionMethod", from_py_object)]
#[derive(Clone)]
pub struct PyAttributionMethod {
    pub(crate) inner: AttributionMethod,
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
