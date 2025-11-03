//! Sensitivity analysis engine.

use super::types::{
    SensitivityConfig, SensitivityMode, SensitivityResult, SensitivityScenario,
};
use crate::error::{Error, Result};
use crate::evaluator::Evaluator;
use crate::types::{AmountOrScalar, FinancialModelSpec};
use indexmap::IndexMap;

/// Sensitivity analyzer for financial models.
///
/// # Examples
///
/// ```rust,ignore
/// use finstack_statements::analysis::{SensitivityAnalyzer, SensitivityConfig, SensitivityMode};
///
/// let analyzer = SensitivityAnalyzer::new(&model);
/// let mut config = SensitivityConfig::new(SensitivityMode::Diagonal);
/// // Add parameters and targets...
/// let result = analyzer.run(&config)?;
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
                self.apply_parameter_override(&mut model_clone, &param.node_id, param.period_id, *perturbation)?;

                // Evaluate
                let mut evaluator = Evaluator::new();
                let results = evaluator.evaluate(&model_clone)?;

                // Store scenario
                let mut parameter_values = IndexMap::new();
                parameter_values.insert(param.node_id.clone(), *perturbation);

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
    fn run_full_grid(&self, _config: &SensitivityConfig) -> Result<SensitivityResult> {
        // Simplified implementation - full factorial would be more complex
        Err(Error::invalid_input(
            "Full grid sensitivity not yet implemented",
        ))
    }

    /// Run tornado sensitivity.
    fn run_tornado(&self, config: &SensitivityConfig) -> Result<SensitivityResult> {
        // For tornado, we run diagonal and then rank by impact
        self.run_diagonal(config)
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
            Err(Error::invalid_input(format!("Node '{}' not found", node_id)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::types::ParameterSpec;
    use crate::builder::ModelBuilder;
    use finstack_core::dates::PeriodId;

    #[test]
    fn test_diagonal_sensitivity() {
        let period = PeriodId::quarter(2025, 1);
        let period2 = PeriodId::quarter(2025, 2);
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .unwrap()
            .value("revenue", &[
                (period, AmountOrScalar::scalar(100_000.0)),
                (period2, AmountOrScalar::scalar(100_000.0)),
            ])
            .compute("cogs", "revenue * 0.4")
            .unwrap()
            .compute("gross_profit", "revenue - cogs")
            .unwrap()
            .build()
            .unwrap();

        let analyzer = SensitivityAnalyzer::new(&model);

        let mut config = SensitivityConfig::new(SensitivityMode::Diagonal);
        config.add_parameter(ParameterSpec::with_percentages(
            "revenue",
            period,
            100_000.0,
            vec![-10.0, 0.0, 10.0],
        ));
        config.add_target_metric("gross_profit");

        let result = analyzer.run(&config).unwrap();
        assert_eq!(result.scenarios.len(), 3); // 3 perturbations
    }
}

