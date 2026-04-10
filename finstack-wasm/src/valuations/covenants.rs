//! WASM bindings for covenant evaluation and breach forecasting.

use crate::core::error::js_error;
use crate::statements::evaluator::JsStatementResult;
use crate::statements::types::JsFinancialModelSpec;
use finstack_core::dates::PeriodId;
use finstack_valuations::covenants::{
    Covenant, CovenantBreach, CovenantEngine, CovenantForecastConfig as ValCovForecastConfig,
    CovenantReport, CovenantScope, CovenantSpec, CovenantType,
    GenericCovenantForecast as ValCovForecast, ThresholdTest,
};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

// =============================================================================
// CovenantType
// =============================================================================

/// Type of financial or operational covenant.
#[wasm_bindgen]
pub struct JsCovenantType {
    inner: CovenantType,
}

#[wasm_bindgen]
impl JsCovenantType {
    /// Maximum debt-to-EBITDA ratio.
    #[wasm_bindgen(js_name = maxDebtToEBITDA)]
    pub fn max_debt_to_ebitda(threshold: f64) -> JsCovenantType {
        JsCovenantType {
            inner: CovenantType::MaxDebtToEBITDA { threshold },
        }
    }

    /// Minimum interest coverage ratio.
    #[wasm_bindgen(js_name = minInterestCoverage)]
    pub fn min_interest_coverage(threshold: f64) -> JsCovenantType {
        JsCovenantType {
            inner: CovenantType::MinInterestCoverage { threshold },
        }
    }

    /// Minimum fixed charge coverage ratio.
    #[wasm_bindgen(js_name = minFixedChargeCoverage)]
    pub fn min_fixed_charge_coverage(threshold: f64) -> JsCovenantType {
        JsCovenantType {
            inner: CovenantType::MinFixedChargeCoverage { threshold },
        }
    }

    /// Maximum total leverage ratio.
    #[wasm_bindgen(js_name = maxTotalLeverage)]
    pub fn max_total_leverage(threshold: f64) -> JsCovenantType {
        JsCovenantType {
            inner: CovenantType::MaxTotalLeverage { threshold },
        }
    }

    /// Maximum senior leverage ratio.
    #[wasm_bindgen(js_name = maxSeniorLeverage)]
    pub fn max_senior_leverage(threshold: f64) -> JsCovenantType {
        JsCovenantType {
            inner: CovenantType::MaxSeniorLeverage { threshold },
        }
    }

    /// Minimum debt service coverage ratio.
    #[wasm_bindgen(js_name = minDSCR)]
    pub fn min_dscr(threshold: f64) -> JsCovenantType {
        JsCovenantType {
            inner: CovenantType::MinDSCR { threshold },
        }
    }

    /// Maximum net debt to EBITDA ratio.
    #[wasm_bindgen(js_name = maxNetDebtToEBITDA)]
    pub fn max_net_debt_to_ebitda(threshold: f64) -> JsCovenantType {
        JsCovenantType {
            inner: CovenantType::MaxNetDebtToEBITDA { threshold },
        }
    }

    /// Maximum capital expenditure.
    #[wasm_bindgen(js_name = maxCapex)]
    pub fn max_capex(threshold: f64) -> JsCovenantType {
        JsCovenantType {
            inner: CovenantType::MaxCapex { threshold },
        }
    }

    /// Minimum liquidity (cash + available revolver).
    #[wasm_bindgen(js_name = minLiquidity)]
    pub fn min_liquidity(threshold: f64) -> JsCovenantType {
        JsCovenantType {
            inner: CovenantType::MinLiquidity { threshold },
        }
    }

    /// Custom covenant with metric and comparator.
    #[wasm_bindgen(js_name = custom)]
    pub fn custom(
        metric: String,
        comparator: String,
        threshold: f64,
    ) -> Result<JsCovenantType, JsValue> {
        let test = match comparator.to_ascii_lowercase().as_str() {
            "maximum" | "le" | "lte" | "<=" => ThresholdTest::Maximum(threshold),
            "minimum" | "ge" | "gte" | ">=" => ThresholdTest::Minimum(threshold),
            other => return Err(js_error(format!("Unknown comparator: {}", other))),
        };
        Ok(JsCovenantType {
            inner: CovenantType::Custom { metric, test },
        })
    }

    /// Get the stable machine-readable covenant identifier.
    #[wasm_bindgen(getter, js_name = covenantId)]
    pub fn covenant_id(&self) -> String {
        self.inner.covenant_id().to_string()
    }

    /// Get a human-readable description.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        self.inner.to_string()
    }
}

// =============================================================================
// Covenant
// =============================================================================

/// Financial covenant specification with test frequency and consequences.
#[wasm_bindgen]
pub struct JsCovenant {
    inner: Covenant,
}

#[wasm_bindgen]
impl JsCovenant {
    /// Create a covenant with quarterly test frequency.
    #[wasm_bindgen(constructor)]
    pub fn new(ctype: &JsCovenantType) -> JsCovenant {
        JsCovenant {
            inner: Covenant::new(
                ctype.inner.clone(),
                finstack_core::dates::Tenor::quarterly(),
            ),
        }
    }

    /// Set cure period in days.
    #[wasm_bindgen(js_name = withCurePeriod)]
    pub fn with_cure_period(mut self, days: Option<i32>) -> JsCovenant {
        self.inner.cure_period_days = days;
        self
    }

    /// Set covenant scope to incurrence (tested only on specific actions).
    #[wasm_bindgen(js_name = asIncurrence)]
    pub fn as_incurrence(mut self) -> JsCovenant {
        self.inner.scope = CovenantScope::Incurrence;
        self
    }

    /// Whether the covenant is active.
    #[wasm_bindgen(getter, js_name = isActive)]
    pub fn is_active(&self) -> bool {
        self.inner.is_active
    }

    /// Get cure period in days (if set).
    #[wasm_bindgen(getter, js_name = curePeriodDays)]
    pub fn cure_period_days(&self) -> Option<i32> {
        self.inner.cure_period_days
    }
}

// =============================================================================
// CovenantSpec
// =============================================================================

/// Covenant wrapped with a metric identifier for evaluation.
#[wasm_bindgen]
pub struct JsCovenantSpec {
    inner: CovenantSpec,
}

#[wasm_bindgen]
impl JsCovenantSpec {
    /// Create a covenant spec bound to a metric.
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

// =============================================================================
// CovenantReport
// =============================================================================

/// Result of a single covenant check (pass/fail with values and headroom).
#[wasm_bindgen]
pub struct JsCovenantReport {
    inner: CovenantReport,
}

#[wasm_bindgen]
impl JsCovenantReport {
    /// Type of covenant being checked.
    #[wasm_bindgen(getter, js_name = covenantType)]
    pub fn covenant_type(&self) -> String {
        self.inner.covenant_type.clone()
    }

    /// Stable covenant identifier (if available).
    #[wasm_bindgen(getter, js_name = covenantId)]
    pub fn covenant_id(&self) -> Option<String> {
        self.inner.covenant_id.clone()
    }

    /// Whether the covenant passed.
    #[wasm_bindgen(getter)]
    pub fn passed(&self) -> bool {
        self.inner.passed
    }

    /// Actual metric value.
    #[wasm_bindgen(getter, js_name = actualValue)]
    pub fn actual_value(&self) -> Option<f64> {
        self.inner.actual_value
    }

    /// Required threshold.
    #[wasm_bindgen(getter)]
    pub fn threshold(&self) -> Option<f64> {
        self.inner.threshold
    }

    /// Details or explanation.
    #[wasm_bindgen(getter)]
    pub fn details(&self) -> Option<String> {
        self.inner.details.clone()
    }

    /// Cushion relative to threshold (positive = passing buffer).
    #[wasm_bindgen(getter)]
    pub fn headroom(&self) -> Option<f64> {
        self.inner.headroom
    }
}

// =============================================================================
// CovenantBreach
// =============================================================================

/// A detected covenant breach.
#[wasm_bindgen]
pub struct JsCovenantBreach {
    inner: CovenantBreach,
}

#[wasm_bindgen]
impl JsCovenantBreach {
    /// Covenant identifier.
    #[wasm_bindgen(getter, js_name = covenantId)]
    pub fn covenant_id(&self) -> String {
        self.inner.covenant_id.clone()
    }

    /// Actual metric value at breach.
    #[wasm_bindgen(getter, js_name = actualValue)]
    pub fn actual_value(&self) -> Option<f64> {
        self.inner.actual_value
    }

    /// Threshold that was breached.
    #[wasm_bindgen(getter)]
    pub fn threshold(&self) -> Option<f64> {
        self.inner.threshold
    }

    /// Whether the breach has been cured.
    #[wasm_bindgen(getter, js_name = isCured)]
    pub fn is_cured(&self) -> bool {
        self.inner.is_cured
    }

    /// Breach date as ISO string.
    #[wasm_bindgen(getter, js_name = breachDate)]
    pub fn breach_date(&self) -> String {
        self.inner.breach_date.to_string()
    }
}

// =============================================================================
// CovenantEngine
// =============================================================================

/// Rule-based covenant evaluation engine.
///
/// Evaluates financial covenants against current metrics, detects breaches,
/// and applies consequences.
#[wasm_bindgen]
pub struct JsCovenantEngine {
    inner: CovenantEngine,
}

#[wasm_bindgen]
impl JsCovenantEngine {
    /// Create a new covenant engine from a list of covenant specs (as JSON).
    #[wasm_bindgen(constructor)]
    pub fn new(specs_json: &str) -> Result<JsCovenantEngine, JsValue> {
        let specs: Vec<CovenantSpec> = serde_json::from_str(specs_json)
            .map_err(|e| js_error(format!("Invalid covenant specs JSON: {}", e)))?;
        let mut engine = CovenantEngine::new();
        for spec in specs {
            engine.add_spec(spec);
        }
        Ok(JsCovenantEngine { inner: engine })
    }

    /// Get the number of registered covenants.
    #[wasm_bindgen(getter)]
    pub fn count(&self) -> usize {
        self.inner.specs.len()
    }

    /// Serialize the engine state to JSON.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        serde_json::to_string_pretty(&self.inner.specs)
            .map_err(|e| js_error(format!("Serialization failed: {}", e)))
    }
}

// =============================================================================
// CovenantForecastConfig + CovenantForecast
// =============================================================================

/// Configuration for covenant breach forecasting.
#[wasm_bindgen]
pub struct JsCovenantForecastConfig {
    inner: ValCovForecastConfig,
}

#[wasm_bindgen]
impl JsCovenantForecastConfig {
    /// Create a forecast config.
    #[wasm_bindgen(constructor)]
    pub fn new(
        stochastic: bool,
        num_paths: usize,
        volatility: Option<f64>,
        seed: Option<u64>,
        antithetic: bool,
    ) -> JsCovenantForecastConfig {
        let cfg = ValCovForecastConfig {
            stochastic,
            num_paths,
            volatility,
            random_seed: seed,
            antithetic,
            reference_date: None,
        };
        JsCovenantForecastConfig { inner: cfg }
    }
}

/// Result of a covenant breach forecast.
#[wasm_bindgen]
pub struct JsCovenantForecast {
    inner: ValCovForecast,
}

#[wasm_bindgen]
impl JsCovenantForecast {
    /// Covenant identifier.
    #[wasm_bindgen(getter, js_name = covenantId)]
    pub fn covenant_id(&self) -> String {
        self.inner.covenant_id.clone()
    }

    /// Test dates as ISO strings.
    #[wasm_bindgen(getter, js_name = testDates)]
    pub fn test_dates(&self) -> js_sys::Array {
        let arr = js_sys::Array::new();
        for d in &self.inner.test_dates {
            arr.push(&JsValue::from_str(&d.to_string()));
        }
        arr
    }

    /// Projected metric values at each test date.
    #[wasm_bindgen(getter, js_name = projectedValues)]
    pub fn projected_values(&self) -> js_sys::Float64Array {
        js_sys::Float64Array::from(self.inner.projected_values.as_slice())
    }

    /// Threshold at each test date.
    #[wasm_bindgen(getter)]
    pub fn thresholds(&self) -> js_sys::Float64Array {
        js_sys::Float64Array::from(self.inner.thresholds.as_slice())
    }

    /// Headroom at each test date (positive = buffer).
    #[wasm_bindgen(getter)]
    pub fn headroom(&self) -> js_sys::Float64Array {
        js_sys::Float64Array::from(self.inner.headroom.as_slice())
    }

    /// Probability of breach at each test date (for stochastic forecasts).
    #[wasm_bindgen(getter, js_name = breachProbability)]
    pub fn breach_probability(&self) -> js_sys::Float64Array {
        js_sys::Float64Array::from(self.inner.breach_probability.as_slice())
    }

    /// Get indices where headroom is below the warning threshold.
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

// =============================================================================
// Covenant templates
// =============================================================================

/// Generate a standard LBO covenant package as JSON.
///
/// Returns a JSON array of `CovenantSpec` objects.
#[wasm_bindgen(js_name = lboStandardCovenants)]
pub fn lbo_standard_covenants(
    initial_leverage: f64,
    interest_coverage: f64,
    fixed_charge_coverage: f64,
    max_capex: f64,
) -> Result<JsValue, JsValue> {
    let specs = finstack_valuations::covenants::templates::lbo_standard(
        initial_leverage,
        interest_coverage,
        fixed_charge_coverage,
        max_capex,
    );
    serde_wasm_bindgen::to_value(&specs)
        .map_err(|e| js_error(format!("Serialization failed: {}", e)))
}

/// Generate a covenant-lite leveraged loan package as JSON.
#[wasm_bindgen(js_name = covLiteCovenants)]
pub fn cov_lite_covenants(max_leverage: f64, max_senior_leverage: f64) -> Result<JsValue, JsValue> {
    let specs =
        finstack_valuations::covenants::templates::cov_lite(max_leverage, max_senior_leverage);
    serde_wasm_bindgen::to_value(&specs)
        .map_err(|e| js_error(format!("Serialization failed: {}", e)))
}

// =============================================================================
// Forecast function
// =============================================================================

/// Forecast covenant compliance over future periods.
#[wasm_bindgen(js_name = forecastCovenant)]
pub fn forecast_covenant(
    spec: &JsCovenantSpec,
    model: &JsFinancialModelSpec,
    base_case: &JsStatementResult,
    periods: js_sys::Array,
    config: Option<JsCovenantForecastConfig>,
) -> Result<JsCovenantForecast, JsValue> {
    let mut ps: Vec<PeriodId> = Vec::with_capacity(periods.length() as usize);
    for v in periods.iter() {
        let s = v
            .as_string()
            .ok_or_else(|| js_error("Invalid period id; expected string"))?;
        let pid = PeriodId::from_str(&s)
            .map_err(|e| js_error(format!("Invalid period '{}': {}", s, e)))?;
        ps.push(pid);
    }
    let cfg = config.map(|c| c.inner).unwrap_or_default();
    finstack_statements_analytics::analysis::covenants::forecast_covenant(
        &spec.inner,
        &model.inner,
        &base_case.inner,
        &ps,
        cfg,
    )
    .map(|inner| JsCovenantForecast { inner })
    .map_err(|e| js_error(e.to_string()))
}
