//! Multi-scenario management and comparison for statement models.
//!
//! This module provides a lightweight registry for named scenarios built on
//! top of a single [`FinancialModelSpec`].
//!
//! A `ScenarioSet` stores a map of scenario name → [`ScenarioDefinition`],
//! supports simple parent chaining with override merging, and can:
//! - Evaluate all scenarios into a [`ScenarioResults`] envelope.
//! - Compute variance-style diffs between two scenarios using
//!   [`VarianceAnalyzer`].
//! - Export wide comparison tables as Polars DataFrames when the
//!   `dataframes` feature is enabled.
//!
//! The wire format is intentionally simple and mirrors the design docs:
//!
//! ```json
//! {
//!   "scenario_set": {
//!     "base": { "model_id": "acme-2025", "overrides": {} },
//!     "downside": {
//!       "parent": "base",
//!       "overrides": { "revenue_growth": -0.05, "margin": -0.02 }
//!     },
//!     "stress": {
//!       "parent": "downside",
//!       "overrides": { "revenue_growth": -0.15 }
//!     }
//!   }
//! }
//! ```
//!
//! In the first implementation, `overrides` is interpreted as a map of
//! `node_id → scalar`, where the scalar is broadcast as an explicit value
//! for **all periods** of the given node. This leverages the existing
//! precedence rules (`Value > Forecast > Formula`) to override model drivers
//! in a deterministic way without introducing new forecast semantics.

use crate::analysis::{VarianceAnalyzer, VarianceConfig, VarianceReport};
use crate::error::{Error, Result};
use crate::evaluator::StatementResult;
use crate::types::{AmountOrScalar, FinancialModelSpec};
use finstack_core::dates::PeriodId;
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

#[cfg(feature = "dataframes")]
use crate::utils::constants::EPSILON;
#[cfg(feature = "dataframes")]
use polars::prelude::*;

/// Definition for a single named scenario.
///
/// Scenarios are attached to a base [`FinancialModelSpec`] and specify:
/// - An optional `model_id` hint (for multi-model workflows).
/// - An optional `parent` scenario to inherit overrides from.
/// - A set of scalar overrides applied as explicit values for all periods
///   of the referenced nodes.
///
/// The JSON representation mirrors the design docs example:
///
/// ```json
/// "downside": {
///   "parent": "base",
///   "overrides": { "revenue_growth": -0.05, "margin": -0.02 }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioDefinition {
    /// Optional identifier of the underlying financial model.
    ///
    /// This is a **hint only** in the first implementation and is not
    /// enforced at runtime. It enables future multi-model workflows while
    /// remaining backwards compatible with the current single-model design.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,

    /// Optional parent scenario to inherit overrides from.
    ///
    /// Parent chains can be arbitrarily deep but must be acyclic. Later
    /// scenarios in the chain override earlier ones for the same `node_id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,

    /// Scalar overrides for model nodes (`node_id -> scalar value`).
    ///
    /// During evaluation these are applied as explicit `AmountOrScalar::scalar`
    /// values for **all periods** of the model, leveraging the existing
    /// `Value > Forecast > Formula` precedence to override forecasts and
    /// formulas when present.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub overrides: IndexMap<String, f64>,
}

/// Registry of named scenarios built on top of a base model.
///
/// The `scenarios` map preserves insertion order and is the primary surface
/// for JSON (de)serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioSet {
    /// Map of scenario name → definition.
    pub scenarios: IndexMap<String, ScenarioDefinition>,
}

/// Evaluated results for all scenarios in a [`ScenarioSet`].
#[derive(Debug, Clone)]
pub struct ScenarioResults {
    /// Map of scenario name → evaluated [`StatementResult`] for that scenario.
    pub scenarios: IndexMap<String, StatementResult>,
}

/// Variance-style diff between two evaluated scenarios.
///
/// This is a thin wrapper around [`VarianceReport`] that keeps track of the
/// baseline and comparison scenario names.
#[derive(Debug, Clone)]
pub struct ScenarioDiff {
    /// Baseline scenario name.
    pub baseline: String,
    /// Comparison scenario name.
    pub comparison: String,
    /// Underlying variance report.
    pub variance: VarianceReport,
}

impl ScenarioSet {
    /// Evaluate all scenarios against a base financial model.
    ///
    /// Parent scenarios are resolved first, with child overrides applied
    /// last. Each scenario is evaluated independently starting from the
    /// provided `base_model`.
    pub fn evaluate_all(&self, base_model: &FinancialModelSpec) -> Result<ScenarioResults> {
        if self.scenarios.is_empty() {
            return Err(Error::invalid_input(
                "ScenarioSet.scenarios cannot be empty",
            ));
        }

        let mut out = IndexMap::new();

        for (name, _) in &self.scenarios {
            let merged_overrides = self.resolve_overrides(name)?;
            let mut model = base_model.clone();
            apply_overrides(&mut model, &merged_overrides)?;

            let mut evaluator = crate::evaluator::Evaluator::new();
            let results = evaluator.evaluate(&model)?;
            out.insert(name.clone(), results);
        }

        Ok(ScenarioResults { scenarios: out })
    }

    /// Compute a variance-style diff between two evaluated scenarios.
    ///
    /// This delegates to [`VarianceAnalyzer`] under the hood so that
    /// scenario diffs share the same semantics as other variance reports.
    pub fn diff(
        &self,
        results: &ScenarioResults,
        baseline: &str,
        comparison: &str,
        metrics: &[String],
        periods: &[PeriodId],
    ) -> Result<ScenarioDiff> {
        if metrics.is_empty() {
            return Err(Error::invalid_input("metrics cannot be empty"));
        }

        if periods.is_empty() {
            return Err(Error::invalid_input("periods cannot be empty"));
        }

        let baseline_results = results.scenarios.get(baseline).ok_or_else(|| {
            Error::invalid_input(format!("Unknown baseline scenario '{baseline}'"))
        })?;

        let comparison_results = results.scenarios.get(comparison).ok_or_else(|| {
            Error::invalid_input(format!("Unknown comparison scenario '{comparison}'"))
        })?;

        let analyzer = VarianceAnalyzer::new(baseline_results, comparison_results);
        let config = VarianceConfig::new(
            baseline.to_string(),
            comparison.to_string(),
            metrics.to_vec(),
            periods.to_vec(),
        );

        let variance = analyzer.compute(&config)?;

        Ok(ScenarioDiff {
            baseline: baseline.to_string(),
            comparison: comparison.to_string(),
            variance,
        })
    }

    /// Return the lineage of a scenario from root ancestor to the given name.
    ///
    /// This is useful for explainability and debugging of nested overrides.
    pub fn trace(&self, scenario: &str) -> Result<Vec<String>> {
        let mut lineage = Vec::new();
        let mut seen = IndexSet::new();
        let mut current = Some(scenario);

        while let Some(name) = current {
            if !seen.insert(name.to_string()) {
                return Err(Error::invalid_input(format!(
                    "Cycle detected in scenario parents at '{name}'"
                )));
            }

            let def = self.scenarios.get(name).ok_or_else(|| {
                Error::invalid_input(format!("Unknown scenario '{name}' in trace()"))
            })?;

            lineage.push(name.to_string());
            current = def.parent.as_deref();
        }

        lineage.reverse();
        Ok(lineage)
    }

    /// Resolve the full override map for a given scenario by walking the
    /// parent chain from root to leaf.
    ///
    /// Later scenarios in the chain override earlier ones for the same
    /// `node_id`.
    fn resolve_overrides(&self, name: &str) -> Result<IndexMap<String, f64>> {
        let mut merged = IndexMap::new();
        let mut stack = Vec::new();
        let mut seen = IndexSet::new();
        let mut current = Some(name);

        while let Some(scenario_name) = current {
            if !seen.insert(scenario_name.to_string()) {
                return Err(Error::invalid_input(format!(
                    "Cycle detected in scenario parents at '{scenario_name}'"
                )));
            }

            let def = self.scenarios.get(scenario_name).ok_or_else(|| {
                Error::invalid_input(format!("Unknown scenario '{scenario_name}'"))
            })?;
            stack.push(scenario_name);
            current = def.parent.as_deref();
        }

        // Merge from oldest ancestor → target scenario so that later overrides win.
        while let Some(scenario_name) = stack.pop() {
            // Scenario was verified to exist when pushed onto the stack
            if let Some(def) = self.scenarios.get(scenario_name) {
                for (node_id, value) in &def.overrides {
                    merged.insert(node_id.clone(), *value);
                }
            }
        }

        Ok(merged)
    }
}

impl ScenarioResults {
    /// Return the number of scenarios.
    pub fn len(&self) -> usize {
        self.scenarios.len()
    }

    /// Check if there are no scenarios.
    pub fn is_empty(&self) -> bool {
        self.scenarios.is_empty()
    }
}

#[cfg(feature = "dataframes")]
impl ScenarioResults {
    /// Export a wide comparison table to a Polars DataFrame.
    ///
    /// Columns:
    /// - `period` (Utf8)
    /// - `metric` (Utf8)
    /// - One Float64 column per scenario (named by scenario).
    /// - For each non-baseline scenario, an additional Float64 column
    ///   `<scenario>_vs_<baseline>_pct` computed as:
    ///   `(scenario - baseline) / baseline`, with `0.0` used when the
    ///   baseline is effectively zero to avoid infinities/NaNs.
    ///
    /// The baseline scenario is chosen as:
    /// - `"base"` if present, otherwise
    /// - the first scenario in insertion order.
    pub fn to_comparison_df(&self, metrics: &[&str]) -> Result<DataFrame> {
        if self.scenarios.is_empty() {
            return Err(Error::invalid_input(
                "ScenarioResults.scenarios cannot be empty",
            ));
        }

        if metrics.is_empty() {
            return Err(Error::invalid_input("metrics cannot be empty"));
        }

        let mut scenario_iter = self.scenarios.iter();
        let baseline_name = if self.scenarios.contains_key("base") {
            "base"
        } else {
            scenario_iter
                .next()
                .map(|(name, _)| name.as_str())
                .ok_or_else(|| Error::invalid_input("ScenarioResults.scenarios cannot be empty"))?
        };

        let baseline_results = self.scenarios.get(baseline_name).ok_or_else(|| {
            Error::invalid_input(format!(
                "Baseline scenario '{}' not found in ScenarioResults",
                baseline_name
            ))
        })?;

        // Pre-allocate output columns.
        let scenario_names: Vec<&str> = self.scenarios.keys().map(|k| k.as_str()).collect();
        let non_baseline_names: Vec<&str> = scenario_names
            .iter()
            .copied()
            .filter(|name| *name != baseline_name)
            .collect();

        let mut periods_col: Vec<String> = Vec::new();
        let mut metrics_col: Vec<String> = Vec::new();

        let mut scenario_values: Vec<Vec<Option<f64>>> = vec![Vec::new(); scenario_names.len()];
        let mut pct_values: Vec<Vec<Option<f64>>> = vec![Vec::new(); non_baseline_names.len()];

        for metric in metrics {
            let metric_nodes = baseline_results.nodes.get(*metric).ok_or_else(|| {
                Error::missing_data(format!(
                    "Metric '{}' not found in baseline scenario '{}'",
                    metric, baseline_name
                ))
            })?;

            for (&period, _) in metric_nodes {
                periods_col.push(period.to_string());
                metrics_col.push((*metric).to_string());

                // Compute scenario values and percent deltas vs baseline.
                let baseline_value = baseline_results.get(metric, &period);

                for (idx, scenario_name) in scenario_names.iter().enumerate() {
                    if let Some(results) = self.scenarios.get(*scenario_name) {
                        let value = results.get(metric, &period);
                        scenario_values[idx].push(value);
                    } else {
                        scenario_values[idx].push(None);
                    }
                }

                for (pct_idx, scenario_name) in non_baseline_names.iter().enumerate() {
                    if let Some(results) = self.scenarios.get(*scenario_name) {
                        let value = results.get(metric, &period);

                        let pct = match (baseline_value, value) {
                            (Some(base), Some(v)) => {
                                if base.abs() < EPSILON {
                                    Some(0.0)
                                } else {
                                    Some((v - base) / base)
                                }
                            }
                            _ => None,
                        };

                        pct_values[pct_idx].push(pct);
                    } else {
                        pct_values[pct_idx].push(None);
                    }
                }
            }
        }

        let mut columns: Vec<Column> = Vec::new();
        columns.push(Series::new("period".into(), periods_col).into());
        columns.push(Series::new("metric".into(), metrics_col).into());

        for (idx, scenario_name) in scenario_names.iter().enumerate() {
            columns.push(Series::new((*scenario_name).into(), scenario_values[idx].clone()).into());

            if *scenario_name != baseline_name {
                // Find index in pct_values for this scenario.
                if let Some(pct_idx) = non_baseline_names
                    .iter()
                    .position(|name| *name == *scenario_name)
                {
                    let pct_col_name = format!("{}_vs_{}_pct", scenario_name, baseline_name);
                    columns.push(
                        Series::new(pct_col_name.as_str().into(), pct_values[pct_idx].clone())
                            .into(),
                    );
                }
            }
        }

        let df = DataFrame::new_infer_height(columns).map_err(|e| {
            Error::invalid_input(format!(
                "Failed to create scenario comparison DataFrame: {}",
                e
            ))
        })?;

        Ok(df)
    }
}

/// Apply a resolved override map to a financial model.
///
/// For each `(node_id, value)` pair, this function:
/// - Looks up the corresponding [`NodeSpec`](crate::types::NodeSpec).
/// - Clones or creates the `values` map.
/// - Inserts `AmountOrScalar::scalar(value)` for **all** model periods.
fn apply_overrides(
    model: &mut FinancialModelSpec,
    overrides: &IndexMap<String, f64>,
) -> Result<()> {
    if overrides.is_empty() {
        return Ok(());
    }

    let period_ids: Vec<PeriodId> = model.periods.iter().map(|p| p.id).collect();

    for (node_id, value) in overrides {
        let node = model.get_node_mut(node_id).ok_or_else(|| {
            Error::invalid_input(format!("Node '{}' not found in model", node_id))
        })?;

        let mut values = node.values.clone().unwrap_or_default();

        for period_id in &period_ids {
            values.insert(*period_id, AmountOrScalar::scalar(*value));
        }

        node.values = Some(values);
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn resolve_overrides_respects_parent_chain() {
        let mut scenarios = IndexMap::new();

        scenarios.insert(
            "base".to_string(),
            ScenarioDefinition {
                model_id: None,
                parent: None,
                overrides: IndexMap::new(),
            },
        );

        let mut downside_overrides = IndexMap::new();
        downside_overrides.insert("revenue".to_string(), 90_000.0);
        scenarios.insert(
            "downside".to_string(),
            ScenarioDefinition {
                model_id: None,
                parent: Some("base".to_string()),
                overrides: downside_overrides,
            },
        );

        let mut stress_overrides = IndexMap::new();
        stress_overrides.insert("revenue".to_string(), 80_000.0);
        scenarios.insert(
            "stress".to_string(),
            ScenarioDefinition {
                model_id: None,
                parent: Some("downside".to_string()),
                overrides: stress_overrides,
            },
        );

        let set = ScenarioSet { scenarios };

        let base = set
            .resolve_overrides("base")
            .expect("base overrides should resolve");
        assert!(base.is_empty());

        let downside = set
            .resolve_overrides("downside")
            .expect("downside overrides should resolve");
        assert_eq!(downside.get("revenue"), Some(&90_000.0));

        let stress = set
            .resolve_overrides("stress")
            .expect("stress overrides should resolve");
        assert_eq!(stress.get("revenue"), Some(&80_000.0));
    }
}
