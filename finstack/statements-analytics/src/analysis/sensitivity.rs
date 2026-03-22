//! Sensitivity analysis engine.

use super::types::{
    ParameterSpec, SensitivityConfig, SensitivityMode, SensitivityResult, SensitivityScenario,
    TornadoEntry,
};
use finstack_statements::error::{Error, Result};
use finstack_statements::evaluator::{Evaluator, StatementResult};
use finstack_statements::types::{AmountOrScalar, FinancialModelSpec};
use finstack_core::dates::PeriodId;
use indexmap::IndexMap;

/// Sensitivity analyzer for financial models.
///
/// # Examples
///
/// ```rust
/// use finstack_statements::prelude::*;
/// use finstack_statements::analysis::{SensitivityAnalyzer, SensitivityConfig, SensitivityMode, ParameterSpec};
///
/// # fn main() -> Result<()> {
/// let model = ModelBuilder::new("sensitivity_test")
///     .periods("2025Q1..Q2", None)?
///     .value("revenue", &[
///         (PeriodId::quarter(2025, 1), AmountOrScalar::scalar(100_000.0)),
///         (PeriodId::quarter(2025, 2), AmountOrScalar::scalar(110_000.0)),
///     ])
///     .compute("cogs", "revenue * 0.6")?
///     .compute("gross_profit", "revenue - cogs")?
///     .build()?;
///
/// let analyzer = SensitivityAnalyzer::new(&model);
/// let mut config = SensitivityConfig::new(SensitivityMode::Diagonal);
///
/// // Define parameter to vary
/// config.add_parameter(ParameterSpec::with_percentages(
///     "revenue",
///     PeriodId::quarter(2025, 1),
///     100_000.0,
///     vec![-10.0, 0.0, 10.0],
/// ));
///
/// // Define target metric to observe
/// config.add_target_metric("gross_profit");
///
/// // Run sensitivity analysis
/// let result = analyzer.run(&config)?;
/// assert_eq!(result.scenarios.len(), 3); // One for each perturbation
/// # Ok(())
/// # }
/// ```
pub struct SensitivityAnalyzer<'a> {
    model: &'a FinancialModelSpec,
}

impl<'a> SensitivityAnalyzer<'a> {
    /// Create a new sensitivity analyzer.
    pub fn new(model: &'a FinancialModelSpec) -> Self {
        Self { model }
    }

    /// Run sensitivity analysis.
    pub fn run(&self, config: &SensitivityConfig) -> Result<SensitivityResult> {
        match config.mode {
            SensitivityMode::Diagonal => self.run_diagonal(config),
            SensitivityMode::FullGrid => self.run_full_grid(config),
            SensitivityMode::Tornado => self.run_tornado(config),
        }
    }

    /// Run diagonal sensitivity (one-at-a-time).
    fn run_diagonal(&self, config: &SensitivityConfig) -> Result<SensitivityResult> {
        let mut scenarios = Vec::new();

        for param in &config.parameters {
            for perturbation in &param.perturbations {
                // Clone model and override this parameter
                let mut model_clone = self.model.clone();
                self.apply_parameter_override(
                    &mut model_clone,
                    &param.node_id,
                    param.period_id,
                    *perturbation,
                )?;

                // Evaluate
                let mut evaluator = Evaluator::new();
                let results = evaluator.evaluate(&model_clone)?;

                // Store scenario
                let mut parameter_values = IndexMap::new();
                parameter_values.insert(
                    scenario_parameter_key(&param.node_id, param.period_id),
                    *perturbation,
                );

                scenarios.push(SensitivityScenario {
                    parameter_values,
                    results,
                });
            }
        }

        Ok(SensitivityResult {
            config: config.clone(),
            scenarios,
        })
    }

    /// Run full grid sensitivity (factorial).
    fn run_full_grid(&self, config: &SensitivityConfig) -> Result<SensitivityResult> {
        if config.parameters.is_empty() {
            return Err(Error::invalid_input(
                "Full grid sensitivity requires at least one parameter",
            ));
        }

        let mut combinations = Vec::new();
        let mut current = Vec::new();
        build_parameter_grid(&config.parameters, 0, &mut current, &mut combinations);

        let mut scenarios = Vec::with_capacity(combinations.len());
        for combination in combinations {
            let mut model_clone = self.model.clone();
            let mut parameter_values = IndexMap::new();

            for (param_idx, perturbation) in combination.into_iter().enumerate() {
                let param = &config.parameters[param_idx];
                self.apply_parameter_override(
                    &mut model_clone,
                    &param.node_id,
                    param.period_id,
                    perturbation,
                )?;
                parameter_values.insert(
                    scenario_parameter_key(&param.node_id, param.period_id),
                    perturbation,
                );
            }

            let mut evaluator = Evaluator::new();
            let results = evaluator.evaluate(&model_clone)?;
            scenarios.push(SensitivityScenario {
                parameter_values,
                results,
            });
        }

        Ok(SensitivityResult {
            config: config.clone(),
            scenarios,
        })
    }

    /// Run tornado sensitivity.
    fn run_tornado(&self, config: &SensitivityConfig) -> Result<SensitivityResult> {
        let mut result = self.run_diagonal(config)?;
        if config.target_metrics.is_empty() {
            return Ok(result);
        }

        let mut baseline_evaluator = Evaluator::new();
        let baseline = baseline_evaluator.evaluate(self.model)?;
        result.scenarios.sort_by(|lhs, rhs| {
            let rhs_impact = max_target_impact(&baseline, &rhs.results, &config.target_metrics);
            let lhs_impact = max_target_impact(&baseline, &lhs.results, &config.target_metrics);
            rhs_impact
                .partial_cmp(&lhs_impact)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(result)
    }

    // Helper to override a parameter value in the model
    fn apply_parameter_override(
        &self,
        model: &mut FinancialModelSpec,
        node_id: &str,
        period_id: finstack_core::dates::PeriodId,
        value: f64,
    ) -> Result<()> {
        if let Some(node) = model.nodes.get_mut(node_id) {
            // Override the value for this period
            let mut values = node.values.clone().unwrap_or_default();
            values.insert(period_id, AmountOrScalar::scalar(value));
            node.values = Some(values);
            Ok(())
        } else {
            Err(Error::invalid_input(format!(
                "Node '{}' not found",
                node_id
            )))
        }
    }
}

fn scenario_parameter_key(node_id: &str, period_id: finstack_core::dates::PeriodId) -> String {
    format!("{}@{}", node_id, period_id)
}

fn build_parameter_grid(
    parameters: &[super::types::ParameterSpec],
    idx: usize,
    current: &mut Vec<f64>,
    combinations: &mut Vec<Vec<f64>>,
) {
    if idx == parameters.len() {
        combinations.push(current.clone());
        return;
    }

    for perturbation in &parameters[idx].perturbations {
        current.push(*perturbation);
        build_parameter_grid(parameters, idx + 1, current, combinations);
        current.pop();
    }
}

fn max_target_impact(
    baseline: &finstack_statements::evaluator::StatementResult,
    scenario: &finstack_statements::evaluator::StatementResult,
    target_metrics: &[String],
) -> f64 {
    target_metrics
        .iter()
        .flat_map(|metric| {
            baseline
                .nodes
                .get(metric)
                .into_iter()
                .flat_map(move |baseline_periods| {
                    baseline_periods
                        .iter()
                        .filter_map(move |(period_id, baseline_value)| {
                            scenario
                                .get(metric, period_id)
                                .map(|scenario_value| (scenario_value - baseline_value).abs())
                        })
                })
        })
        .fold(0.0, f64::max)
}

// ── Tornado chart generation ──

/// Generate tornado chart entries for a specific metric from sensitivity results.
///
/// Each entry represents one parameter's downside and upside impact on the
/// target metric relative to its baseline value. Entries are sorted by
/// descending absolute swing magnitude.
///
/// # Arguments
///
/// * `result`      - Completed sensitivity analysis result.
/// * `metric_node` - Node identifier for the metric to inspect.
/// * `period_hint` - Optional period to look up; if `None`, uses the first
///   available period for the node.
pub fn generate_tornado_entries(
    result: &SensitivityResult,
    metric_node: &str,
    period_hint: Option<PeriodId>,
) -> Vec<TornadoEntry> {
    let mut entries = Vec::new();

    for param in &result.config.parameters {
        if let Some(entry) = build_tornado_entry(result, param, metric_node, period_hint) {
            entries.push(entry);
        }
    }

    entries.sort_by(|a, b| {
        b.swing()
            .abs()
            .partial_cmp(&a.swing().abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    entries
}

fn build_tornado_entry(
    result: &SensitivityResult,
    param: &ParameterSpec,
    metric_node: &str,
    period_hint: Option<PeriodId>,
) -> Option<TornadoEntry> {
    let parameter_key = scenario_parameter_key(&param.node_id, param.period_id);
    let mut min_record: Option<(f64, f64)> = None;
    let mut max_record: Option<(f64, f64)> = None;
    let mut baseline_metric = None;

    for scenario in &result.scenarios {
        let Some(param_value) = scenario.parameter_values.get(&parameter_key) else {
            continue;
        };
        let metric_value = extract_metric_value(&scenario.results, metric_node, period_hint)?;

        if approx_equal(*param_value, param.base_value) {
            baseline_metric = Some(metric_value);
        }

        match &mut min_record {
            Some((current_value, current_metric)) => {
                if *param_value < *current_value {
                    *current_value = *param_value;
                    *current_metric = metric_value;
                }
            }
            None => {
                min_record = Some((*param_value, metric_value));
            }
        }

        match &mut max_record {
            Some((current_value, current_metric)) => {
                if *param_value > *current_value {
                    *current_value = *param_value;
                    *current_metric = metric_value;
                }
            }
            None => {
                max_record = Some((*param_value, metric_value));
            }
        }
    }

    let base = baseline_metric
        .or_else(|| min_record.map(|(_, value)| value))
        .or_else(|| max_record.map(|(_, value)| value))?;

    let downside = min_record.map(|(_, value)| value - base).unwrap_or(0.0);
    let upside = max_record.map(|(_, value)| value - base).unwrap_or(0.0);

    Some(TornadoEntry {
        parameter_id: param.node_id.clone(),
        downside,
        upside,
    })
}

fn extract_metric_value(
    results: &StatementResult,
    node_id: &str,
    period_hint: Option<PeriodId>,
) -> Option<f64> {
    if let Some(period) = period_hint {
        results.get(node_id, &period)
    } else {
        results
            .nodes
            .get(node_id)
            .and_then(|periods| periods.values().next().copied())
    }
}

fn approx_equal(lhs: f64, rhs: f64) -> bool {
    let scale = lhs.abs().max(rhs.abs()).max(1.0);
    (lhs - rhs).abs() <= 1e-9 * scale
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::analysis::types::ParameterSpec;
    use finstack_statements::builder::ModelBuilder;
    use finstack_core::dates::PeriodId;

    #[test]
    fn test_diagonal_sensitivity() {
        let period = PeriodId::quarter(2025, 1);
        let period2 = PeriodId::quarter(2025, 2);
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .expect("valid period range")
            .value(
                "revenue",
                &[
                    (period, AmountOrScalar::scalar(100_000.0)),
                    (period2, AmountOrScalar::scalar(100_000.0)),
                ],
            )
            .compute("cogs", "revenue * 0.4")
            .expect("valid formula")
            .compute("gross_profit", "revenue - cogs")
            .expect("valid formula")
            .build()
            .expect("valid model");

        let analyzer = SensitivityAnalyzer::new(&model);

        let mut config = SensitivityConfig::new(SensitivityMode::Diagonal);
        config.add_parameter(ParameterSpec::with_percentages(
            "revenue",
            period,
            100_000.0,
            vec![-10.0, 0.0, 10.0],
        ));
        config.add_target_metric("gross_profit");

        let result = analyzer
            .run(&config)
            .expect("sensitivity analysis should succeed");
        assert_eq!(result.scenarios.len(), 3); // 3 perturbations
    }

    #[test]
    fn test_full_grid_sensitivity_builds_cartesian_product() {
        let period = PeriodId::quarter(2025, 1);
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q1", None)
            .expect("valid period range")
            .value("revenue", &[(period, AmountOrScalar::scalar(100_000.0))])
            .value("cogs", &[(period, AmountOrScalar::scalar(40_000.0))])
            .compute("gross_profit", "revenue - cogs")
            .expect("valid formula")
            .build()
            .expect("valid model");

        let analyzer = SensitivityAnalyzer::new(&model);
        let mut config = SensitivityConfig::new(SensitivityMode::FullGrid);
        config.add_parameter(ParameterSpec::new(
            "revenue",
            period,
            100_000.0,
            vec![90_000.0, 110_000.0],
        ));
        config.add_parameter(ParameterSpec::new(
            "cogs",
            period,
            40_000.0,
            vec![35_000.0, 45_000.0],
        ));
        config.add_target_metric("gross_profit");

        let result = analyzer.run(&config).expect("full grid should succeed");
        assert_eq!(result.scenarios.len(), 4);
        assert!(result
            .scenarios
            .iter()
            .all(|scenario| scenario.parameter_values.len() == 2));
    }

    #[test]
    fn test_tornado_orders_scenarios_by_target_metric_impact() {
        let period = PeriodId::quarter(2025, 1);
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q1", None)
            .expect("valid period range")
            .value("revenue", &[(period, AmountOrScalar::scalar(100_000.0))])
            .value("cogs", &[(period, AmountOrScalar::scalar(40_000.0))])
            .compute("gross_profit", "revenue - cogs")
            .expect("valid formula")
            .build()
            .expect("valid model");

        let analyzer = SensitivityAnalyzer::new(&model);
        let mut config = SensitivityConfig::new(SensitivityMode::Tornado);
        config.add_parameter(ParameterSpec::new(
            "revenue",
            period,
            100_000.0,
            vec![80_000.0, 120_000.0],
        ));
        config.add_parameter(ParameterSpec::new(
            "cogs",
            period,
            40_000.0,
            vec![30_000.0, 50_000.0],
        ));
        config.add_target_metric("gross_profit");

        let result = analyzer.run(&config).expect("tornado should succeed");
        assert_eq!(result.scenarios.len(), 4);
        assert_eq!(
            result.scenarios[0].parameter_values.keys().next(),
            Some(&"revenue@2025Q1".to_string())
        );
        assert_eq!(
            result.scenarios[1].parameter_values.keys().next(),
            Some(&"revenue@2025Q1".to_string())
        );
    }

    #[test]
    fn test_full_grid_scenario_metadata_distinguishes_period_overrides() {
        let period1 = PeriodId::quarter(2025, 1);
        let period2 = PeriodId::quarter(2025, 2);
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .expect("valid period range")
            .value(
                "revenue",
                &[
                    (period1, AmountOrScalar::scalar(100_000.0)),
                    (period2, AmountOrScalar::scalar(110_000.0)),
                ],
            )
            .build()
            .expect("valid model");

        let analyzer = SensitivityAnalyzer::new(&model);
        let mut config = SensitivityConfig::new(SensitivityMode::FullGrid);
        config.add_parameter(ParameterSpec::new(
            "revenue",
            period1,
            100_000.0,
            vec![90_000.0, 110_000.0],
        ));
        config.add_parameter(ParameterSpec::new(
            "revenue",
            period2,
            110_000.0,
            vec![100_000.0, 120_000.0],
        ));

        let result = analyzer.run(&config).expect("full grid should succeed");
        assert_eq!(result.scenarios.len(), 4);
        assert!(result.scenarios.iter().all(|scenario| {
            scenario.parameter_values.contains_key("revenue@2025Q1")
                && scenario.parameter_values.contains_key("revenue@2025Q2")
        }));
    }
}
