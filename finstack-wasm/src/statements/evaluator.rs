//! Evaluator for financial models.

use crate::core::dates::FsDate;
use crate::core::error::js_error;
use crate::core::market_data::context::JsMarketContext;
use crate::statements::types::JsFinancialModelSpec;
use finstack_core::dates::PeriodId;
use finstack_statements::evaluator::{
    node_to_dated_schedule as core_node_to_dated_schedule, Evaluator, MonteCarloConfig,
    MonteCarloResults, PercentileSeries, PeriodDateConvention, ResultsMeta, StatementResult,
};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

/// Metadata about evaluation results.
///
/// Contains information about the evaluation process including
/// timing, node count, and period count.
#[wasm_bindgen]
pub struct JsStatementResultMeta {
    inner: ResultsMeta,
}

#[wasm_bindgen]
impl JsStatementResultMeta {
    /// Evaluation time in milliseconds.
    #[wasm_bindgen(getter, js_name = evalTimeMs)]
    pub fn eval_time_ms(&self) -> Option<u64> {
        self.inner.eval_time_ms
    }

    /// Number of nodes evaluated.
    #[wasm_bindgen(getter, js_name = numNodes)]
    pub fn num_nodes(&self) -> usize {
        self.inner.num_nodes
    }

    /// Number of periods evaluated.
    #[wasm_bindgen(getter, js_name = numPeriods)]
    pub fn num_periods(&self) -> usize {
        self.inner.num_periods
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "ResultsMeta(nodes={}, periods={}, eval_time_ms={:?})",
            self.inner.num_nodes, self.inner.num_periods, self.inner.eval_time_ms
        )
    }
}

impl JsStatementResultMeta {
    fn new(inner: ResultsMeta) -> Self {
        Self { inner }
    }
}

/// Results from evaluating a financial model.
///
/// Contains node values for each period and evaluation metadata.
#[wasm_bindgen]
pub struct JsStatementResult {
    pub(crate) inner: StatementResult,
}

#[wasm_bindgen]
impl JsStatementResult {
    /// Get the value for a node at a specific period.
    ///
    /// # Arguments
    /// * `node_id` - Node identifier
    /// * `period_id` - Period identifier (string like "2025Q1")
    ///
    /// # Returns
    /// Value if found, null otherwise
    #[wasm_bindgen]
    pub fn get(&self, node_id: &str, period_id: &str) -> Result<Option<f64>, JsValue> {
        let pid = PeriodId::from_str(period_id)
            .map_err(|e| JsValue::from_str(&format!("Invalid period ID '{}': {}", period_id, e)))?;
        Ok(self.inner.get(node_id, &pid))
    }

    /// Get all period values for a specific node.
    ///
    /// # Arguments
    /// * `node_id` - Node identifier
    ///
    /// # Returns
    /// JavaScript object mapping period IDs to values, or null if node not found
    #[wasm_bindgen(js_name = getNode)]
    pub fn get_node(&self, node_id: &str) -> Result<JsValue, JsValue> {
        if let Some(period_map) = self.inner.get_node(node_id) {
            // Convert to JavaScript object
            let obj = js_sys::Object::new();
            for (period_id, value) in period_map {
                let period_str = period_id.to_string();
                js_sys::Reflect::set(
                    &obj,
                    &JsValue::from_str(&period_str),
                    &JsValue::from_f64(*value),
                )?;
            }
            Ok(JsValue::from(obj))
        } else {
            Ok(JsValue::NULL)
        }
    }

    /// Get value or default.
    ///
    /// # Arguments
    /// * `node_id` - Node identifier
    /// * `period_id` - Period identifier
    /// * `default` - Default value if not found
    ///
    /// # Returns
    /// Value or default
    #[wasm_bindgen(js_name = getOr)]
    pub fn get_or(&self, node_id: &str, period_id: &str, default: f64) -> Result<f64, JsValue> {
        let pid = PeriodId::from_str(period_id)
            .map_err(|e| JsValue::from_str(&format!("Invalid period ID '{}': {}", period_id, e)))?;
        Ok(self.inner.get_or(node_id, &pid, default))
    }

    /// Get all period values for a node as an array.
    ///
    /// # Arguments
    /// * `node_id` - Node identifier
    ///
    /// # Returns
    /// Array of [periodId, value] pairs
    #[wasm_bindgen(js_name = allPeriods)]
    pub fn all_periods(&self, node_id: &str) -> JsValue {
        let array = js_sys::Array::new();
        for (period_id, value) in self.inner.all_periods(node_id) {
            let pair = js_sys::Array::new();
            pair.push(&JsValue::from_str(&period_id.to_string()));
            pair.push(&JsValue::from_f64(value));
            array.push(&pair);
        }
        JsValue::from(array)
    }

    /// Get all node results.
    ///
    /// # Returns
    /// JavaScript object mapping node IDs to period value objects
    #[wasm_bindgen(getter)]
    pub fn nodes(&self) -> Result<JsValue, JsValue> {
        let obj = js_sys::Object::new();
        for (node_id, period_map) in &self.inner.nodes {
            let inner_obj = js_sys::Object::new();
            for (period_id, value) in period_map {
                js_sys::Reflect::set(
                    &inner_obj,
                    &JsValue::from_str(&period_id.to_string()),
                    &JsValue::from_f64(*value),
                )?;
            }
            js_sys::Reflect::set(&obj, &JsValue::from_str(node_id), &JsValue::from(inner_obj))?;
        }
        Ok(JsValue::from(obj))
    }

    /// Get evaluation metadata.
    #[wasm_bindgen(getter)]
    pub fn meta(&self) -> JsStatementResultMeta {
        JsStatementResultMeta::new(self.inner.meta.clone())
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize Results: {}", e)))
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsStatementResult, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsStatementResult { inner })
            .map_err(|e| JsValue::from_str(&format!("Failed to deserialize Results: {}", e)))
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "StatementResult(nodes={}, periods={})",
            self.inner.nodes.len(),
            self.inner.meta.num_periods
        )
    }
}

impl JsStatementResult {
    pub(crate) fn new(inner: StatementResult) -> Self {
        Self { inner }
    }
}

/// Evaluator for financial models.
///
/// The evaluator compiles formulas, resolves dependencies, and evaluates
/// nodes period-by-period according to precedence rules.
///
/// # Example
/// ```javascript
/// const evaluator = new JsEvaluator();
/// const results = evaluator.evaluate(model);
/// console.log(results.get("revenue", "2025Q1"));
/// ```
#[wasm_bindgen]
pub struct JsEvaluator {
    inner: Evaluator,
}

impl Default for JsEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl JsEvaluator {
    /// Create a new evaluator.
    #[wasm_bindgen(constructor)]
    pub fn new() -> JsEvaluator {
        JsEvaluator {
            inner: Evaluator::new(),
        }
    }

    /// Evaluate a financial model over all periods.
    ///
    /// This is a convenience method that evaluates without market context.
    /// If your model uses capital structure with cs.* references, use
    /// `evaluateWithMarketContext` instead.
    ///
    /// # Arguments
    /// * `model` - Financial model specification
    ///
    /// # Returns
    /// Evaluation results
    #[wasm_bindgen]
    pub fn evaluate(&mut self, model: &JsFinancialModelSpec) -> Result<JsStatementResult, JsValue> {
        let results = self
            .inner
            .evaluate(&model.inner)
            .map_err(|e| JsValue::from_str(&format!("Evaluation failed: {}", e)))?;
        Ok(JsStatementResult::new(results))
    }

    /// Evaluate a financial model with market context for pricing.
    ///
    /// This method allows you to provide market context for pricing capital
    /// structure instruments. If capital structure is defined but market context
    /// is not provided, capital structure cashflows will not be computed.
    ///
    /// # Arguments
    /// * `model` - Financial model specification
    /// * `market_ctx` - Market context for pricing instruments
    /// * `as_of` - Valuation date for pricing
    ///
    /// # Returns
    /// Evaluation results
    #[wasm_bindgen(js_name = evaluateWithMarketContext)]
    pub fn evaluate_with_market_context(
        &mut self,
        model: &JsFinancialModelSpec,
        market_ctx: &JsMarketContext,
        as_of: &FsDate,
    ) -> Result<JsStatementResult, JsValue> {
        let results = self
            .inner
            .evaluate_with_market_context(
                &model.inner,
                Some(market_ctx.inner()),
                Some(as_of.inner()),
            )
            .map_err(|e| {
                JsValue::from_str(&format!("Evaluation with market context failed: {}", e))
            })?;
        Ok(JsStatementResult::new(results))
    }

    /// Evaluate a financial model in Monte Carlo mode.
    ///
    /// Replays the model `nPaths` times with independent, deterministic seeds
    /// for stochastic forecast methods and aggregates into percentile bands.
    ///
    /// # Arguments
    /// * `model` - Financial model specification
    /// * `config` - Monte Carlo configuration
    ///
    /// # Returns
    /// Monte Carlo results with percentile distributions
    #[wasm_bindgen(js_name = evaluateMonteCarlo)]
    pub fn evaluate_monte_carlo(
        &mut self,
        model: &JsFinancialModelSpec,
        config: &JsMonteCarloConfig,
    ) -> Result<JsMonteCarloResults, JsValue> {
        let results = self
            .inner
            .evaluate_monte_carlo(&model.inner, &config.inner)
            .map_err(|e| js_error(format!("Monte Carlo evaluation failed: {e}")))?;
        Ok(JsMonteCarloResults { inner: results })
    }
}

/// Configuration for Monte Carlo evaluation of a statement model.
#[wasm_bindgen(js_name = MonteCarloConfig)]
pub struct JsMonteCarloConfig {
    inner: MonteCarloConfig,
}

#[wasm_bindgen(js_class = MonteCarloConfig)]
impl JsMonteCarloConfig {
    /// Create a new Monte Carlo configuration.
    ///
    /// # Arguments
    /// * `n_paths` - Number of simulation paths
    /// * `seed` - Base random seed for deterministic results
    #[wasm_bindgen(constructor)]
    pub fn new(n_paths: usize, seed: u64) -> JsMonteCarloConfig {
        JsMonteCarloConfig {
            inner: MonteCarloConfig::new(n_paths, seed),
        }
    }

    /// Override the percentiles to compute (values in [0, 1]).
    ///
    /// Default percentiles are [0.05, 0.5, 0.95].
    #[wasm_bindgen(js_name = withPercentiles)]
    pub fn with_percentiles(mut self, percentiles: Vec<f64>) -> JsMonteCarloConfig {
        self.inner = self.inner.with_percentiles(percentiles);
        self
    }

    /// Number of Monte Carlo paths.
    #[wasm_bindgen(getter, js_name = nPaths)]
    pub fn n_paths(&self) -> usize {
        self.inner.n_paths
    }

    /// Base random seed.
    #[wasm_bindgen(getter)]
    pub fn seed(&self) -> u64 {
        self.inner.seed
    }

    /// Percentiles to compute.
    #[wasm_bindgen(getter)]
    pub fn percentiles(&self) -> Vec<f64> {
        self.inner.percentiles.clone()
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsMonteCarloConfig, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsMonteCarloConfig { inner })
            .map_err(|e| js_error(format!("Failed to deserialize MonteCarloConfig: {e}")))
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| js_error(format!("Failed to serialize MonteCarloConfig: {e}")))
    }
}

/// Per-metric percentile time series from Monte Carlo evaluation.
#[wasm_bindgen(js_name = PercentileSeries)]
pub struct JsPercentileSeries {
    inner: PercentileSeries,
}

#[wasm_bindgen(js_class = PercentileSeries)]
impl JsPercentileSeries {
    /// Metric / node identifier.
    #[wasm_bindgen(getter)]
    pub fn metric(&self) -> String {
        self.inner.metric.clone()
    }

    /// Get values as a JavaScript object: `{ periodId: [[percentile, value], ...] }`.
    #[wasm_bindgen(getter)]
    pub fn values(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.values)
            .map_err(|e| js_error(format!("Failed to serialize PercentileSeries values: {e}")))
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "PercentileSeries(metric='{}', periods={})",
            self.inner.metric,
            self.inner.values.len()
        )
    }
}

/// Monte Carlo results for a statement model.
///
/// Contains percentile distributions for each metric across forecast periods.
#[wasm_bindgen(js_name = MonteCarloResults)]
pub struct JsMonteCarloResults {
    inner: MonteCarloResults,
}

#[wasm_bindgen(js_class = MonteCarloResults)]
impl JsMonteCarloResults {
    /// Number of Monte Carlo paths simulated.
    #[wasm_bindgen(getter, js_name = nPaths)]
    pub fn n_paths(&self) -> usize {
        self.inner.n_paths
    }

    /// Percentiles computed for each metric/period.
    #[wasm_bindgen(getter)]
    pub fn percentiles(&self) -> Vec<f64> {
        self.inner.percentiles.clone()
    }

    /// Forecast periods included in the simulation.
    #[wasm_bindgen(getter, js_name = forecastPeriods)]
    pub fn forecast_periods(&self) -> Vec<String> {
        self.inner
            .forecast_periods
            .iter()
            .map(|p| p.to_string())
            .collect()
    }

    /// Get the percentile series for a specific metric.
    #[wasm_bindgen(js_name = getPercentileSeries)]
    pub fn get_percentile_series(&self, metric: &str) -> Option<JsPercentileSeries> {
        self.inner
            .percentile_results
            .get(metric)
            .map(|series| JsPercentileSeries {
                inner: series.clone(),
            })
    }

    /// Get a time series of a specific percentile for a metric.
    ///
    /// Returns a JS object mapping period IDs to values, or null if not found.
    #[wasm_bindgen(js_name = getPercentileTimeSeries)]
    pub fn get_percentile_time_series(
        &self,
        metric: &str,
        percentile: f64,
    ) -> Result<JsValue, JsValue> {
        match self.inner.get_percentile_series(metric, percentile) {
            Some(series) => {
                let obj = js_sys::Object::new();
                for (period_id, value) in &series {
                    js_sys::Reflect::set(
                        &obj,
                        &JsValue::from_str(&period_id.to_string()),
                        &JsValue::from_f64(*value),
                    )?;
                }
                Ok(JsValue::from(obj))
            }
            None => Ok(JsValue::NULL),
        }
    }

    /// Estimate the probability that a metric exceeds a threshold in any forecast period.
    #[wasm_bindgen(js_name = breachProbability)]
    pub fn breach_probability(&self, metric: &str, threshold: f64) -> Option<f64> {
        self.inner.breach_probability(metric, threshold)
    }

    /// List all metric names in the results.
    #[wasm_bindgen(getter, js_name = metricNames)]
    pub fn metric_names(&self) -> Vec<String> {
        self.inner.percentile_results.keys().cloned().collect()
    }

    /// Convert to JSON representation.
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner)
            .map_err(|e| js_error(format!("Failed to serialize MonteCarloResults: {e}")))
    }

    /// Create from JSON representation.
    #[wasm_bindgen(js_name = fromJSON)]
    pub fn from_json(value: JsValue) -> Result<JsMonteCarloResults, JsValue> {
        serde_wasm_bindgen::from_value(value)
            .map(|inner| JsMonteCarloResults { inner })
            .map_err(|e| js_error(format!("Failed to deserialize MonteCarloResults: {e}")))
    }

    /// Convert to string representation.
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        format!(
            "MonteCarloResults(paths={}, metrics={}, periods={})",
            self.inner.n_paths,
            self.inner.percentile_results.len(),
            self.inner.forecast_periods.len()
        )
    }
}

/// Convention for mapping a statement period into a point-in-time cashflow date.
#[wasm_bindgen(js_name = PeriodDateConvention)]
pub enum JsPeriodDateConvention {
    /// Use the period start date.
    Start = "start",
    /// Use the last inclusive day of the period.
    End = "end",
}

/// Export a statement node as a dated cashflow schedule.
///
/// Iterates periods in model order and extracts values from results for the node.
/// Each period is mapped to a date using the date convention.
///
/// # Returns
/// Array of `[dateString, value]` pairs.
#[wasm_bindgen(js_name = nodeToDatedSchedule)]
pub fn node_to_dated_schedule(
    model: &JsFinancialModelSpec,
    results: &JsStatementResult,
    node_id: &str,
    date_convention: JsPeriodDateConvention,
) -> Result<JsValue, JsValue> {
    let convention = match date_convention {
        JsPeriodDateConvention::Start => PeriodDateConvention::Start,
        JsPeriodDateConvention::End => PeriodDateConvention::End,
        _ => PeriodDateConvention::End,
    };

    let schedule = core_node_to_dated_schedule(model.inner(), &results.inner, node_id, convention)
        .map_err(|e| js_error(format!("Failed to export dated schedule: {e}")))?;

    let array = js_sys::Array::new();
    for (date, value) in schedule {
        let pair = js_sys::Array::new();
        pair.push(&JsValue::from_str(&date.to_string()));
        pair.push(&JsValue::from_f64(value));
        array.push(&pair);
    }
    Ok(JsValue::from(array))
}
