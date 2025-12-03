use crate::statements::evaluator::JsResults;
use crate::statements::types::JsFinancialModelSpec;
use finstack_core::dates::{Date, PeriodId};
use finstack_valuations::covenants::engine::{Covenant, CovenantSpec, CovenantType, ThresholdTest};
use finstack_valuations::covenants::forward::{
    CovenantForecast as ValCovForecast, CovenantForecastConfig as ValCovForecastConfig,
    McConfig as ValMcConfig, ModelTimeSeries,
};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct JsCovenantType {
    inner: CovenantType,
}

#[wasm_bindgen]
impl JsCovenantType {
    #[wasm_bindgen(js_name = maxDebtToEBITDA)]
    pub fn max_debt_to_ebitda(threshold: f64) -> JsCovenantType {
        JsCovenantType {
            inner: CovenantType::MaxDebtToEBITDA { threshold },
        }
    }

    #[wasm_bindgen(js_name = minInterestCoverage)]
    pub fn min_interest_coverage(threshold: f64) -> JsCovenantType {
        JsCovenantType {
            inner: CovenantType::MinInterestCoverage { threshold },
        }
    }

    #[wasm_bindgen(js_name = minFixedChargeCoverage)]
    pub fn min_fixed_charge_coverage(threshold: f64) -> JsCovenantType {
        JsCovenantType {
            inner: CovenantType::MinFixedChargeCoverage { threshold },
        }
    }

    #[wasm_bindgen(js_name = maxTotalLeverage)]
    pub fn max_total_leverage(threshold: f64) -> JsCovenantType {
        JsCovenantType {
            inner: CovenantType::MaxTotalLeverage { threshold },
        }
    }

    #[wasm_bindgen(js_name = maxSeniorLeverage)]
    pub fn max_senior_leverage(threshold: f64) -> JsCovenantType {
        JsCovenantType {
            inner: CovenantType::MaxSeniorLeverage { threshold },
        }
    }

    #[wasm_bindgen(js_name = custom)]
    pub fn custom(
        metric: String,
        comparator: String,
        threshold: f64,
    ) -> Result<JsCovenantType, JsValue> {
        let test = match comparator.to_ascii_lowercase().as_str() {
            "maximum" | "le" | "lte" | "<=" => ThresholdTest::Maximum(threshold),
            "minimum" | "ge" | "gte" | ">=" => ThresholdTest::Minimum(threshold),
            other => return Err(JsValue::from_str(&format!("Unknown comparator: {}", other))),
        };
        Ok(JsCovenantType {
            inner: CovenantType::Custom { metric, test },
        })
    }
}

#[wasm_bindgen]
pub struct JsCovenant {
    inner: Covenant,
}

#[wasm_bindgen]
impl JsCovenant {
    #[wasm_bindgen(constructor)]
    pub fn new(ctype: &JsCovenantType) -> JsCovenant {
        JsCovenant {
            inner: Covenant::new(
                ctype.inner.clone(),
                finstack_core::dates::Frequency::quarterly(),
            ),
        }
    }
}

#[wasm_bindgen]
pub struct JsCovenantSpec {
    inner: CovenantSpec,
}

#[wasm_bindgen]
impl JsCovenantSpec {
    #[wasm_bindgen(js_name = withMetric)]
    pub fn with_metric(covenant: &JsCovenant, metric_id: String) -> JsCovenantSpec {
        JsCovenantSpec {
            inner: CovenantSpec::with_metric(
                covenant.inner.clone(),
                finstack_valuations::metrics::MetricId::custom(&metric_id),
            ),
        }
    }
}

#[wasm_bindgen]
pub struct JsCovenantForecastConfig {
    inner: ValCovForecastConfig,
}

#[wasm_bindgen]
impl JsCovenantForecastConfig {
    #[wasm_bindgen(constructor)]
    pub fn new(
        stochastic: bool,
        num_paths: usize,
        volatility: Option<f64>,
        seed: Option<u64>,
        antithetic: bool,
    ) -> JsCovenantForecastConfig {
        let mc = antithetic.then_some(ValMcConfig {
            seed: 0,
            antithetic: true,
        });
        let cfg = ValCovForecastConfig {
            stochastic,
            num_paths,
            volatility,
            random_seed: seed,
            mc,
        };
        JsCovenantForecastConfig { inner: cfg }
    }
}

#[wasm_bindgen]
pub struct JsCovenantForecast {
    inner: ValCovForecast,
}

#[wasm_bindgen]
impl JsCovenantForecast {
    #[wasm_bindgen(getter, js_name = covenantId)]
    pub fn covenant_id(&self) -> String {
        self.inner.covenant_id.clone()
    }

    #[wasm_bindgen(getter, js_name = testDates)]
    pub fn test_dates(&self) -> js_sys::Array {
        let arr = js_sys::Array::new();
        for d in &self.inner.test_dates {
            arr.push(&JsValue::from_str(&d.to_string()));
        }
        arr
    }

    #[wasm_bindgen(getter, js_name = projectedValues)]
    pub fn projected_values(&self) -> js_sys::Float64Array {
        js_sys::Float64Array::from(self.inner.projected_values.as_slice())
    }
    #[wasm_bindgen(getter)]
    pub fn thresholds(&self) -> js_sys::Float64Array {
        js_sys::Float64Array::from(self.inner.thresholds.as_slice())
    }
    #[wasm_bindgen(getter)]
    pub fn headroom(&self) -> js_sys::Float64Array {
        js_sys::Float64Array::from(self.inner.headroom.as_slice())
    }
    #[wasm_bindgen(getter, js_name = breachProbability)]
    pub fn breach_probability(&self) -> js_sys::Float64Array {
        js_sys::Float64Array::from(self.inner.breach_probability.as_slice())
    }

    #[wasm_bindgen(js_name = warningIndices)]
    pub fn warning_indices(&self, warn_threshold: f64) -> js_sys::Uint32Array {
        let indices: Vec<u32> = self
            .inner
            .warning_indices(warn_threshold)
            .into_iter()
            .map(|idx| idx as u32)
            .collect();
        js_sys::Uint32Array::from(indices.as_slice())
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
        Date::from_calendar_date(period.year, time::Month::December, 31).unwrap()
    }
}

#[wasm_bindgen(js_name = forecastCovenant)]
pub fn forecast_covenant(
    spec: &JsCovenantSpec,
    model: &JsFinancialModelSpec,
    base_case: &JsResults,
    periods: js_sys::Array,
    config: Option<JsCovenantForecastConfig>,
) -> Result<JsCovenantForecast, JsValue> {
    let adapter = StatementsAdapter::new(&model.inner, &base_case.inner);
    let mut ps: Vec<PeriodId> = Vec::with_capacity(periods.length() as usize);
    for v in periods.iter() {
        let s = v
            .as_string()
            .ok_or_else(|| JsValue::from_str("Invalid period id; expected string"))?;
        let pid = PeriodId::from_str(&s)
            .map_err(|e| JsValue::from_str(&format!("Invalid period '{}': {}", s, e)))?;
        ps.push(pid);
    }
    let cfg = config.map(|c| c.inner).unwrap_or_default();
    finstack_valuations::covenants::forward::forecast_covenant_generic(
        &spec.inner,
        &adapter,
        &ps,
        cfg,
    )
    .map(|inner| JsCovenantForecast { inner })
    .map_err(|e| JsValue::from_str(&e.to_string()))
}
