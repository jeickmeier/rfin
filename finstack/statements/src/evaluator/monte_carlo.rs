//! Monte Carlo evaluation types and aggregation utilities.

use crate::error::{Error, Result};
use crate::evaluator::results::EvalWarning;
use crate::types::FinancialModelSpec;
use finstack_core::dates::PeriodId;
use indexmap::IndexMap;

/// Results for a single Monte Carlo path: per-node per-period values and any warnings emitted.
pub(crate) type PathResult = (IndexMap<String, IndexMap<PeriodId, f64>>, Vec<EvalWarning>);
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Configuration for Monte Carlo evaluation of a statement model.
///
/// # Choosing the number of paths
///
/// The required path count depends on the quantities of interest:
///
/// | Use case | Recommended `n_paths` |
/// |---|---|
/// | Mean / median estimates | 1 000 – 2 000 |
/// | 5th / 95th percentiles | 5 000 – 10 000 |
/// | 1st / 99th percentiles or CVaR | 10 000 – 50 000 |
/// | Breach-probability estimates | 10 000+ |
///
/// Standard-error of a percentile estimate scales as
/// $O\bigl(1/\sqrt{n}\bigr)$, so tails require proportionally more
/// paths to converge. When in doubt, run two simulations with
/// different seeds and compare results; if the metric of interest
/// moves by more than the desired precision, increase `n_paths`.
///
/// The default constructor ([`MonteCarloConfig::new`]) does **not** impose a
/// minimum—callers choose the path count explicitly so the trade-off between
/// accuracy and runtime is visible.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloConfig {
    /// Number of Monte Carlo paths to simulate.
    pub n_paths: usize,
    /// Base random seed used to derive per-path seeds.
    pub seed: u64,
    /// Percentiles to compute in the closed interval [0, 1].
    #[serde(default)]
    pub percentiles: Vec<f64>,
}

impl MonteCarloConfig {
    /// Create a new Monte Carlo configuration with default percentiles.
    pub fn new(n_paths: usize, seed: u64) -> Self {
        Self {
            n_paths,
            seed,
            percentiles: vec![0.05, 0.5, 0.95],
        }
    }

    /// Override the percentiles to compute.
    #[must_use]
    pub fn with_percentiles(mut self, percentiles: Vec<f64>) -> Self {
        self.percentiles = percentiles;
        self
    }
}

/// Per-metric percentile time series.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PercentileSeries {
    /// Metric / node identifier.
    pub metric: String,
    /// Period → ordered list of `(percentile, value)` pairs.
    pub values: IndexMap<PeriodId, Vec<(f64, f64)>>,
}

/// Monte Carlo results for a statement model.
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
    /// Warnings encountered while evaluating Monte Carlo paths.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<EvalWarning>,
    /// Internal storage of path-level values.
    #[serde(skip)]
    pub(crate) path_values: IndexMap<String, IndexMap<PeriodId, Vec<f64>>>,
    /// Optional full path data in long-format Polars DataFrame.
    #[cfg(feature = "dataframes")]
    #[serde(skip)]
    pub path_data: Option<polars::prelude::DataFrame>,
}

impl MonteCarloResults {
    /// Get a time series of a specific percentile for a metric.
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

    /// Estimate the probability that a metric exceeds a threshold in any forecast period.
    ///
    /// This checks **upside** breaches only (`value > threshold`). For downside
    /// breaches (e.g. DSCR falling below a floor), negate both the metric values
    /// and the threshold, or use a derived metric that flips the sign.
    ///
    /// Returns `None` if the metric has no data, no forecast periods, or if any
    /// period's path vector is shorter than `n_paths` (incomplete simulation).
    pub fn breach_probability(&self, metric: &str, threshold: f64) -> Option<f64> {
        let metric_map = self.path_values.get(metric)?;
        if metric_map.is_empty() || self.n_paths == 0 {
            return None;
        }

        let forecast_set: HashSet<PeriodId> = self.forecast_periods.iter().copied().collect();
        let mut period_vectors: Vec<(&PeriodId, &Vec<f64>)> = metric_map
            .iter()
            .filter(|(p, _)| forecast_set.contains(p))
            .collect();
        period_vectors.sort_by(|(p1, _), (p2, _)| p1.cmp(p2));

        if period_vectors.is_empty() {
            return None;
        }

        if period_vectors
            .iter()
            .any(|(_, values)| values.len() < self.n_paths)
        {
            return None;
        }

        let mut breached_paths = 0usize;
        for path_idx in 0..self.n_paths {
            let mut breached = false;
            for (_, values) in &period_vectors {
                if values[path_idx] > threshold {
                    breached = true;
                    break;
                }
            }
            if breached {
                breached_paths += 1;
            }
        }

        Some(breached_paths as f64 / self.n_paths as f64)
    }
}

fn normalize_percentiles(raw: &[f64]) -> Vec<f64> {
    let mut v: Vec<f64> = if raw.is_empty() {
        vec![0.05, 0.5, 0.95]
    } else {
        raw.iter().map(|q| q.clamp(0.0, 1.0)).collect()
    };

    v.sort_by(|a, b| a.total_cmp(b));
    v.dedup();
    v
}

pub(crate) struct MonteCarloAccumulator {
    expected_paths: usize,
    observed_paths: usize,
    percentiles: Vec<f64>,
    forecast_periods: Vec<PeriodId>,
    forecast_set: HashSet<PeriodId>,
    path_values: IndexMap<String, IndexMap<PeriodId, Vec<f64>>>,
    warnings: Vec<EvalWarning>,
    #[cfg(feature = "dataframes")]
    path_ids: Vec<u32>,
    #[cfg(feature = "dataframes")]
    periods: Vec<String>,
    #[cfg(feature = "dataframes")]
    metrics: Vec<String>,
    #[cfg(feature = "dataframes")]
    values: Vec<f64>,
}

impl MonteCarloAccumulator {
    pub(crate) fn new(model: &FinancialModelSpec, config: &MonteCarloConfig) -> Result<Self> {
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

        Ok(Self {
            expected_paths: config.n_paths,
            observed_paths: 0,
            percentiles: normalize_percentiles(&config.percentiles),
            forecast_set: forecast_periods.iter().copied().collect(),
            forecast_periods,
            path_values: IndexMap::new(),
            warnings: Vec::new(),
            #[cfg(feature = "dataframes")]
            path_ids: Vec::new(),
            #[cfg(feature = "dataframes")]
            periods: Vec::new(),
            #[cfg(feature = "dataframes")]
            metrics: Vec::new(),
            #[cfg(feature = "dataframes")]
            values: Vec::new(),
        })
    }

    pub(crate) fn push_path(
        &mut self,
        #[cfg_attr(not(feature = "dataframes"), allow(unused_variables))] path_idx: usize,
        path_results: IndexMap<String, IndexMap<PeriodId, f64>>,
        warnings: Vec<EvalWarning>,
    ) -> Result<()> {
        self.observed_paths += 1;
        self.warnings.extend(warnings);

        for (metric, period_map) in path_results {
            let metric_entry = self.path_values.entry(metric.clone()).or_default();
            for (period_id, value) in period_map {
                if !self.forecast_set.contains(&period_id) {
                    continue;
                }
                if !value.is_finite() {
                    return Err(Error::eval(format!(
                        "Monte Carlo aggregation encountered non-finite value for metric '{}' in period {}",
                        metric, period_id
                    )));
                }
                metric_entry.entry(period_id).or_default().push(value);
                #[cfg(feature = "dataframes")]
                {
                    self.path_ids.push(path_idx as u32);
                    self.periods.push(period_id.to_string());
                    self.metrics.push(metric.clone());
                    self.values.push(value);
                }
            }
        }

        Ok(())
    }

    pub(crate) fn finish(self) -> Result<MonteCarloResults> {
        if self.observed_paths == 0 {
            return Err(Error::eval(
                "Monte Carlo aggregation requires at least one path",
            ));
        }
        if self.observed_paths != self.expected_paths {
            return Err(Error::eval(format!(
                "Monte Carlo aggregation mismatch: expected {} paths, got {}",
                self.expected_paths, self.observed_paths
            )));
        }

        let mut percentile_results: IndexMap<String, PercentileSeries> = IndexMap::new();
        for (metric, period_map) in &self.path_values {
            let mut series = PercentileSeries {
                metric: metric.clone(),
                values: IndexMap::new(),
            };

            for (period_id, values) in period_map {
                if values.is_empty() {
                    continue;
                }

                let mut sorted = values.clone();
                sorted.sort_by(|a, b| a.total_cmp(b));
                let len = sorted.len();
                let mut pairs: Vec<(f64, f64)> = Vec::with_capacity(self.percentiles.len());
                for &q in &self.percentiles {
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

            if self.path_ids.is_empty() {
                None
            } else {
                let df = DataFrame::new_infer_height(vec![
                    Series::new("path_id".into(), self.path_ids).into(),
                    Series::new("period".into(), self.periods).into(),
                    Series::new("metric".into(), self.metrics).into(),
                    Series::new("value".into(), self.values).into(),
                ])
                .map_err(|e| {
                    Error::invalid_input(format!("Failed to build Monte Carlo path DataFrame: {e}"))
                })?;
                Some(df)
            }
        };

        Ok(MonteCarloResults {
            percentile_results,
            n_paths: self.observed_paths,
            percentiles: self.percentiles,
            forecast_periods: self.forecast_periods,
            warnings: self.warnings,
            path_values: self.path_values,
            #[cfg(feature = "dataframes")]
            path_data,
        })
    }

    pub(crate) fn merge(mut self, other: Self) -> Result<Self> {
        if self.expected_paths != other.expected_paths
            || self.percentiles != other.percentiles
            || self.forecast_periods != other.forecast_periods
        {
            return Err(Error::eval(
                "Monte Carlo accumulator merge mismatch across parallel partitions",
            ));
        }

        self.observed_paths += other.observed_paths;
        self.warnings.extend(other.warnings);

        for (metric, period_map) in other.path_values {
            let target_metric = self.path_values.entry(metric).or_default();
            for (period_id, values) in period_map {
                target_metric.entry(period_id).or_default().extend(values);
            }
        }

        #[cfg(feature = "dataframes")]
        {
            self.path_ids.extend(other.path_ids);
            self.periods.extend(other.periods);
            self.metrics.extend(other.metrics);
            self.values.extend(other.values);
        }

        Ok(self)
    }

    pub(crate) fn empty_like(&self) -> Self {
        Self {
            expected_paths: self.expected_paths,
            observed_paths: 0,
            percentiles: self.percentiles.clone(),
            forecast_periods: self.forecast_periods.clone(),
            forecast_set: self.forecast_set.clone(),
            path_values: IndexMap::new(),
            warnings: Vec::new(),
            #[cfg(feature = "dataframes")]
            path_ids: Vec::new(),
            #[cfg(feature = "dataframes")]
            periods: Vec::new(),
            #[cfg(feature = "dataframes")]
            metrics: Vec::new(),
            #[cfg(feature = "dataframes")]
            values: Vec::new(),
        }
    }
}

/// Aggregate path-level results into [`MonteCarloResults`].
#[cfg(test)]
pub(crate) fn aggregate_monte_carlo_paths(
    model: &FinancialModelSpec,
    config: &MonteCarloConfig,
    all_paths: &[PathResult],
) -> Result<MonteCarloResults> {
    let mut accumulator = MonteCarloAccumulator::new(model, config)?;
    for (path_idx, (path_results, warnings)) in all_paths.iter().cloned().enumerate() {
        accumulator.push_path(path_idx, path_results, warnings)?;
    }
    accumulator.finish()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::{
        aggregate_monte_carlo_paths, normalize_percentiles, MonteCarloAccumulator, MonteCarloConfig,
    };
    use crate::builder::ModelBuilder;
    use crate::evaluator::EvalWarning;
    use crate::types::AmountOrScalar;
    use finstack_core::dates::PeriodId;
    use indexmap::IndexMap;

    #[test]
    fn normalize_percentiles_clamps_and_dedupes() {
        let raw = vec![-0.1, 0.05, 0.5, 1.2, 0.5];
        let norm = normalize_percentiles(&raw);
        assert_eq!(norm, vec![0.0, 0.05, 0.5, 1.0]);
    }

    #[test]
    fn aggregate_rejects_non_finite_path_values() {
        let period = PeriodId::quarter(2025, 1);
        let model = ModelBuilder::new("mc-agg")
            .periods("2025Q1..Q1", None)
            .expect("valid periods")
            .value("revenue", &[(period, AmountOrScalar::scalar(100.0))])
            .build()
            .expect("valid model");
        let config = MonteCarloConfig::new(1, 7);

        let mut path = IndexMap::new();
        path.insert(
            "revenue".to_string(),
            [(period, f64::NAN)].into_iter().collect(),
        );

        let err = aggregate_monte_carlo_paths(&model, &config, &[(path, Vec::new())])
            .expect_err("NaN must fail");
        assert!(err.to_string().contains("non-finite"));
    }

    #[test]
    fn accumulator_preserves_warnings_for_valid_paths() {
        let period = PeriodId::quarter(2025, 1);
        let model = ModelBuilder::new("mc-agg")
            .periods("2025Q1..Q1", None)
            .expect("valid periods")
            .value("revenue", &[(period, AmountOrScalar::scalar(100.0))])
            .build()
            .expect("valid model");
        let config = MonteCarloConfig::new(1, 7);
        let mut accumulator = MonteCarloAccumulator::new(&model, &config).expect("accumulator");

        let mut path = IndexMap::new();
        path.insert(
            "revenue".to_string(),
            [(period, 100.0)].into_iter().collect(),
        );
        accumulator
            .push_path(
                0,
                path,
                vec![EvalWarning::DivisionByZero {
                    node_id: "revenue".into(),
                    period,
                }],
            )
            .expect("path should be accepted");
        let results = accumulator.finish().expect("results should finish");
        assert_eq!(results.warnings.len(), 1);
    }
}
