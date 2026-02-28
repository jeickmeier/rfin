//! Monte Carlo analysis for statement forecasts.
//!
//! This module provides a lightweight Monte Carlo wrapper around the
//! existing forecasting engine. It is designed to:
//!
//! - Reuse the existing `Evaluator` logic for precedence/where-clause handling
////! - Sample stochastic forecast methods (`Normal`, `LogNormal`) across many
//!   independent paths
//! - Aggregate simulated paths into percentile bands (P5/P50/P95, etc.)
//! - Optionally expose full path data for downstream analytics
//!
//! The core workflow is:
//!
//! 1. Run `Evaluator::evaluate_monte_carlo` with a [`MonteCarloConfig`]
//! 2. The evaluator replays the model `n_paths` times with deterministic,
//!    per-path seeds for stochastic forecast methods
//! 3. Raw path results are aggregated into:
//!    - [`MonteCarloResults::percentile_results`] for quick bands
//!    - Optional `path_data` DataFrame (when `dataframes` feature is enabled)

use crate::error::{Error, Result};
use crate::types::FinancialModelSpec;
use finstack_core::dates::PeriodId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashSet;

/// Configuration for Monte Carlo evaluation of a statement model.
///
/// This configuration controls the number of paths, base RNG seed, and the
/// percentiles to compute from the simulated distribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloConfig {
    /// Number of Monte Carlo paths to simulate.
    pub n_paths: usize,

    /// Base random seed used to derive per-path seeds.
    ///
    /// The same `(model, n_paths, seed, percentiles)` will always produce
    /// identical [`MonteCarloResults`], regardless of serial vs parallel
    /// execution.
    pub seed: u64,

    /// Percentiles to compute in the closed interval \\([0, 1]\\).
    ///
    /// Examples:
    /// - `[0.05, 0.5, 0.95]` → P5 / P50 / P95
    /// - `[0.25, 0.75]` → interquartile range
    ///
    /// When empty, a default of `[0.05, 0.5, 0.95]` is used.
    #[serde(default)]
    pub percentiles: Vec<f64>,
}

impl MonteCarloConfig {
    /// Create a new Monte Carlo configuration with default percentiles
    /// `[0.05, 0.5, 0.95]`.
    pub fn new(n_paths: usize, seed: u64) -> Self {
        Self {
            n_paths,
            seed,
            percentiles: vec![0.05, 0.5, 0.95],
        }
    }

    /// Override the percentiles to compute.
    ///
    /// Values are clamped into \\([0, 1]\\) at aggregation time.
    #[must_use]
    pub fn with_percentiles(mut self, percentiles: Vec<f64>) -> Self {
        self.percentiles = percentiles;
        self
    }
}

/// Per-metric percentile time series.
///
/// For a given metric (node), this structure stores, for each period, the
/// mapping `percentile → value`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PercentileSeries {
    /// Metric / node identifier (e.g. `"ebitda"`).
    pub metric: String,

    /// Period → list of `(percentile, value)` pairs.
    ///
    /// We store percentiles as a small ordered list instead of using them as
    /// map keys to avoid relying on floating-point `Eq`/`Hash` semantics.
    pub values: IndexMap<PeriodId, Vec<(f64, f64)>>,
}

/// Monte Carlo results for a statement model.
///
/// This structure exposes aggregated percentile bands per metric/period and,
/// internally, retains path-level values for advanced analysis such as
/// breach probability calculations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloResults {
    /// Aggregated percentile results: `metric → PercentileSeries`.
    pub percentile_results: IndexMap<String, PercentileSeries>,

    /// Number of Monte Carlo paths simulated.
    pub n_paths: usize,

    /// Percentiles computed for each metric/period.
    pub percentiles: Vec<f64>,

    /// Forecast (non-actual) periods included in the simulation.
    pub forecast_periods: Vec<PeriodId>,

    /// Internal storage of path-level values:
    /// `metric → period → [values across paths]`.
    ///
    /// This is not serialized to avoid bloating wire formats and can be used
    /// for operations like breach probability estimation.
    #[serde(skip)]
    pub(crate) path_values: IndexMap<String, IndexMap<PeriodId, Vec<f64>>>,

    /// Optional full path data in long-format Polars DataFrame.
    ///
    /// Schema:
    /// - `path_id` (UInt32)
    /// - `period` (Utf8)
    /// - `metric` (Utf8)
    /// - `value` (Float64)
    ///
    /// Note: `path_data` is not serialized to avoid a hard dependency on
    /// Polars' serde support and to keep wire formats lightweight.
    #[cfg(feature = "dataframes")]
    #[serde(skip)]
    pub path_data: Option<polars::prelude::DataFrame>,
}

impl MonteCarloResults {
    /// Get a time series of a specific percentile for a metric.
    ///
    /// Returns a map of `PeriodId → value` for the requested percentile,
    /// or `None` if the metric or percentile is not present.
    pub fn get_percentile_series(
        &self,
        metric: &str,
        percentile: f64,
    ) -> Option<IndexMap<PeriodId, f64>> {
        let series = self.percentile_results.get(metric)?;
        let mut out = IndexMap::new();

        for (period, pairs) in &series.values {
            if let Some((_, value)) = pairs.iter().find(|(q, _)| (*q - percentile).abs() < 1e-12) {
                out.insert(*period, *value);
            }
        }

        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    /// Estimate the probability that a metric breaches a threshold in any
    /// forecast period.
    ///
    /// The current implementation treats a "breach" as `value > threshold`
    /// in **any** forecast period for a given path and returns:
    ///
    /// \\[
    ///   \\mathbb{P}(\\exists t \\in \\text{forecast periods} : X_t > \\text{threshold})
    /// \\]
    ///
    /// Returns `None` if no path-level data is available for the metric.
    pub fn breach_probability(&self, metric: &str, threshold: f64) -> Option<f64> {
        let metric_map = self.path_values.get(metric)?;
        if metric_map.is_empty() || self.n_paths == 0 {
            return None;
        }

        // Only consider forecast periods
        let forecast_set: HashSet<PeriodId> = self.forecast_periods.iter().copied().collect();

        // Collect per-period vectors in a deterministic order
        let mut period_vectors: Vec<(&PeriodId, &Vec<f64>)> = metric_map
            .iter()
            .filter(|(p, _)| forecast_set.contains(p))
            .collect();
        period_vectors.sort_by(|(p1, _), (p2, _)| p1.cmp(p2));

        if period_vectors.is_empty() {
            return None;
        }

        let n_paths = self.n_paths;
        let mut breached_paths = 0usize;

        for path_idx in 0..n_paths {
            let mut breached = false;
            for (_, values) in &period_vectors {
                if path_idx < values.len() && values[path_idx] > threshold {
                    breached = true;
                    break;
                }
            }
            if breached {
                breached_paths += 1;
            }
        }

        if n_paths == 0 {
            None
        } else {
            Some(breached_paths as f64 / n_paths as f64)
        }
    }
}

/// Internal helper to normalize and sort percentiles.
fn normalize_percentiles(raw: &[f64]) -> Vec<f64> {
    let mut v: Vec<f64> = if raw.is_empty() {
        vec![0.05, 0.5, 0.95]
    } else {
        raw.iter().map(|q| q.clamp(0.0, 1.0)).collect()
    };

    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    v.dedup();
    v
}

/// Aggregate path-level results into [`MonteCarloResults`].
///
/// The input is a slice of per-path results in the shape:
///
/// ```text
/// path_results[path_idx][metric][period] = value
/// ```
pub(crate) fn aggregate_monte_carlo_paths(
    model: &FinancialModelSpec,
    config: &MonteCarloConfig,
    all_paths: &[IndexMap<String, IndexMap<PeriodId, f64>>],
) -> Result<MonteCarloResults> {
    if all_paths.is_empty() {
        return Err(Error::eval(
            "Monte Carlo aggregation requires at least one path",
        ));
    }

    if all_paths.len() != config.n_paths {
        return Err(Error::eval(format!(
            "Monte Carlo aggregation mismatch: expected {} paths, got {}",
            config.n_paths,
            all_paths.len()
        )));
    }

    let percentiles = normalize_percentiles(&config.percentiles);

    // Identify forecast periods from the model.
    let forecast_periods: Vec<PeriodId> = model
        .periods
        .iter()
        .filter(|p| !p.is_actual)
        .map(|p| p.id)
        .collect();

    if forecast_periods.is_empty() {
        return Err(Error::eval(
            "Monte Carlo evaluation requires at least one forecast period. \
             Use .periods(range, Some(actuals_cutoff)) to define forecast periods.",
        ));
    }

    let forecast_set: HashSet<PeriodId> = forecast_periods.iter().copied().collect();

    // Build metric → period → Vec<path_values>
    let mut path_values: IndexMap<String, IndexMap<PeriodId, Vec<f64>>> = IndexMap::new();

    for path_results in all_paths {
        for (metric, period_map) in path_results {
            let metric_entry = path_values.entry(metric.clone()).or_default();

            for (period_id, value) in period_map {
                if !forecast_set.contains(period_id) {
                    continue;
                }
                metric_entry.entry(*period_id).or_default().push(*value);
            }
        }
    }

    // Aggregate percentiles.
    let mut percentile_results: IndexMap<String, PercentileSeries> = IndexMap::new();

    for (metric, period_map) in &path_values {
        let mut series = PercentileSeries {
            metric: metric.clone(),
            values: IndexMap::new(),
        };

        for (period_id, values) in period_map {
            if values.is_empty() {
                continue;
            }

            let mut sorted = values.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));

            let len = sorted.len();
            if len == 0 {
                continue;
            }

            let mut pairs: Vec<(f64, f64)> = Vec::with_capacity(percentiles.len());
            for &q in &percentiles {
                let index = q * (len.saturating_sub(1) as f64);
                let lower = index.floor() as usize;
                let upper = index.ceil() as usize;

                let value = if lower == upper {
                    sorted[lower]
                } else {
                    let weight = index - lower as f64;
                    sorted[lower] * (1.0 - weight) + sorted[upper] * weight
                };
                pairs.push((q, value));
            }

            series.values.insert(*period_id, pairs);
        }

        percentile_results.insert(metric.clone(), series);
    }

    #[cfg(feature = "dataframes")]
    let path_data = {
        use polars::prelude::*;

        let mut path_ids: Vec<u32> = Vec::new();
        let mut periods: Vec<String> = Vec::new();
        let mut metrics: Vec<String> = Vec::new();
        let mut values: Vec<f64> = Vec::new();

        for (metric, period_map) in &path_values {
            for (period_id, vals) in period_map {
                for (path_idx, v) in vals.iter().enumerate() {
                    path_ids.push(path_idx as u32);
                    periods.push(period_id.to_string());
                    metrics.push(metric.clone());
                    values.push(*v);
                }
            }
        }

        if path_ids.is_empty() {
            None
        } else {
            let df = DataFrame::new_infer_height(vec![
                Series::new("path_id".into(), path_ids).into(),
                Series::new("period".into(), periods).into(),
                Series::new("metric".into(), metrics).into(),
                Series::new("value".into(), values).into(),
            ])
            .map_err(|e| {
                Error::invalid_input(format!("Failed to build Monte Carlo path DataFrame: {e}"))
            })?;
            Some(df)
        }
    };

    Ok(MonteCarloResults {
        percentile_results,
        n_paths: all_paths.len(),
        percentiles,
        forecast_periods,
        path_values,
        #[cfg(feature = "dataframes")]
        path_data,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::normalize_percentiles;

    #[test]
    fn normalize_percentiles_clamps_and_dedupes() {
        let raw = vec![-0.1, 0.05, 0.5, 1.2, 0.5];
        let norm = normalize_percentiles(&raw);
        assert_eq!(norm, vec![0.0, 0.05, 0.5, 1.0]);
    }
}
