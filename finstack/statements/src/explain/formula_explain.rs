//! Formula explanation and calculation breakdown.

use crate::error::{Error, Result};
use crate::evaluator::Results;
use crate::types::{FinancialModelSpec, NodeType};
use finstack_core::dates::PeriodId;
use serde::{Deserialize, Serialize};

/// Explains how formulas are calculated.
///
/// The explainer breaks down formula calculations to show how a node's value
/// was derived from its dependencies.
///
/// # Examples
///
/// ```rust
/// # use finstack_statements::builder::ModelBuilder;
/// # use finstack_statements::evaluator::Evaluator;
/// # use finstack_statements::explain::FormulaExplainer;
/// # use finstack_core::dates::PeriodId;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let model = ModelBuilder::new("demo")
///     .periods("2025Q1..Q2", None)?
///     .compute("revenue", "100000")?
///     .compute("cogs", "revenue * 0.4")?
///     .compute("gross_profit", "revenue - cogs")?
///     .build()?;
///
/// let mut evaluator = Evaluator::new();
/// let results = evaluator.evaluate(&model)?;
///
/// let explainer = FormulaExplainer::new(&model, &results);
/// let period = PeriodId::quarter(2025, 1);
/// let explanation = explainer.explain("gross_profit", &period)?;
///
/// println!("{}", explanation.to_string_detailed());
/// // Output:
/// // gross_profit [2025Q1] = 60,000
/// // Formula: revenue - cogs
/// // Type: Calculated
/// # Ok(())
/// # }
/// ```
pub struct FormulaExplainer<'a> {
    model: &'a FinancialModelSpec,
    results: &'a Results,
}

impl<'a> FormulaExplainer<'a> {
    /// Create a new formula explainer.
    ///
    /// # Arguments
    ///
    /// * `model` - Financial model specification
    /// * `results` - Evaluation results
    pub fn new(model: &'a FinancialModelSpec, results: &'a Results) -> Self {
        Self { model, results }
    }

    /// Explain how a node's value was calculated for a specific period.
    ///
    /// # Arguments
    ///
    /// * `node_id` - Node identifier
    /// * `period` - Period to explain
    ///
    /// # Returns
    ///
    /// Detailed explanation of the calculation
    pub fn explain(&self, node_id: &str, period: &PeriodId) -> Result<Explanation> {
        // Get node spec
        let node_spec = self
            .model
            .nodes
            .get(node_id)
            .ok_or_else(|| Error::invalid_input(format!("Node '{}' not found", node_id)))?;

        // Get final value
        let final_value = self.results.get(node_id, period).ok_or_else(|| {
            Error::invalid_input(format!(
                "No result for node '{}' in period '{}'",
                node_id, period
            ))
        })?;

        // Build breakdown
        let breakdown = self.build_breakdown(node_id, period, &node_spec.formula_text)?;

        Ok(Explanation {
            node_id: node_id.to_string(),
            period_id: *period,
            final_value,
            node_type: node_spec.node_type,
            formula_text: node_spec.formula_text.clone(),
            breakdown,
        })
    }

    // Build calculation breakdown
    fn build_breakdown(
        &self,
        _node_id: &str,
        period: &PeriodId,
        formula: &Option<String>,
    ) -> Result<Vec<ExplanationStep>> {
        let mut breakdown = Vec::new();

        if let Some(formula_text) = formula {
            // Extract all identifiers from formula
            let identifiers = crate::utils::formula::extract_all_identifiers(formula_text)?;

            // Add value for each component
            for identifier in identifiers {
                // Skip cs.* references for now (capital structure)
                if identifier.starts_with("cs.") {
                    continue;
                }

                if let Some(value) = self.results.get(&identifier, period) {
                    breakdown.push(ExplanationStep {
                        component: identifier.clone(),
                        value,
                        operation: None, // We don't parse operations in this simple version
                    });
                }
            }
        }

        Ok(breakdown)
    }
}

/// Detailed explanation of a node's calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Explanation {
    /// Node identifier
    pub node_id: String,

    /// Period being explained
    pub period_id: PeriodId,

    /// Final calculated value
    pub final_value: f64,

    /// Type of node (Value, Calculated, etc.)
    pub node_type: NodeType,

    /// Formula text (if calculated)
    pub formula_text: Option<String>,

    /// Breakdown of calculation components
    pub breakdown: Vec<ExplanationStep>,
}

impl Explanation {
    /// Convert explanation to detailed string format.
    ///
    /// # Returns
    ///
    /// Human-readable explanation of the calculation
    pub fn to_string_detailed(&self) -> String {
        let mut output = String::new();

        // Header
        output.push_str(&format!(
            "{} [{}] = {:.2}\n",
            self.node_id, self.period_id, self.final_value
        ));

        // Formula
        if let Some(formula) = &self.formula_text {
            output.push_str(&format!("Formula: {}\n", formula));
        }

        // Type
        output.push_str(&format!("Type: {:?}\n", self.node_type));

        // Breakdown
        if !self.breakdown.is_empty() {
            output.push_str("\nComponents:\n");
            for step in &self.breakdown {
                output.push_str(&format!("  {} = {:.2}\n", step.component, step.value));
            }
        }

        output
    }

    /// Convert explanation to compact string format.
    ///
    /// # Returns
    ///
    /// Compact single-line summary
    pub fn to_string_compact(&self) -> String {
        if let Some(formula) = &self.formula_text {
            format!(
                "{} [{}] = {:.2} ({})",
                self.node_id, self.period_id, self.final_value, formula
            )
        } else {
            format!(
                "{} [{}] = {:.2}",
                self.node_id, self.period_id, self.final_value
            )
        }
    }
}

/// Step in a calculation breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplanationStep {
    /// Component identifier (e.g., "revenue")
    pub component: String,

    /// Value of the component
    pub value: f64,

    /// Operation applied (e.g., "+", "-", "*", "/")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::ModelBuilder;
    use crate::evaluator::Evaluator;
    use crate::types::AmountOrScalar;

    #[test]
    fn test_explain_value_node() {
        let period = PeriodId::quarter(2025, 1);
        let period2 = PeriodId::quarter(2025, 2);
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .expect("test should succeed")
            .value(
                "revenue",
                &[
                    (period, AmountOrScalar::scalar(100_000.0)),
                    (period2, AmountOrScalar::scalar(110_000.0)),
                ],
            )
            .build()
            .expect("test should succeed");

        let mut evaluator = Evaluator::new();
        let results = evaluator.evaluate(&model).expect("test should succeed");

        let explainer = FormulaExplainer::new(&model, &results);
        let explanation = explainer.explain("revenue", &period).expect("test should succeed");

        assert_eq!(explanation.node_id, "revenue");
        assert_eq!(explanation.final_value, 100_000.0);
        assert!(matches!(explanation.node_type, NodeType::Value));
        assert!(explanation.formula_text.is_none());
        assert!(explanation.breakdown.is_empty());
    }

    #[test]
    fn test_explain_calculated_node() {
        let period = PeriodId::quarter(2025, 1);
        let period2 = PeriodId::quarter(2025, 2);
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .expect("test should succeed")
            .value(
                "revenue",
                &[
                    (period, AmountOrScalar::scalar(100_000.0)),
                    (period2, AmountOrScalar::scalar(110_000.0)),
                ],
            )
            .compute("cogs", "revenue * 0.4")
            .expect("test should succeed")
            .compute("gross_profit", "revenue - cogs")
            .expect("test should succeed")
            .build()
            .expect("test should succeed");

        let mut evaluator = Evaluator::new();
        let results = evaluator.evaluate(&model).expect("test should succeed");

        let explainer = FormulaExplainer::new(&model, &results);
        let explanation = explainer.explain("gross_profit", &period).expect("test should succeed");

        assert_eq!(explanation.node_id, "gross_profit");
        assert_eq!(explanation.final_value, 60_000.0);
        assert!(matches!(explanation.node_type, NodeType::Calculated));
        assert_eq!(explanation.formula_text, Some("revenue - cogs".to_string()));
        assert_eq!(explanation.breakdown.len(), 2); // revenue and cogs
    }

    #[test]
    fn test_explain_to_string_detailed() {
        let period = PeriodId::quarter(2025, 1);
        let period2 = PeriodId::quarter(2025, 2);
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .expect("test should succeed")
            .value(
                "revenue",
                &[
                    (period, AmountOrScalar::scalar(100_000.0)),
                    (period2, AmountOrScalar::scalar(110_000.0)),
                ],
            )
            .compute("cogs", "revenue * 0.4")
            .expect("test should succeed")
            .build()
            .expect("test should succeed");

        let mut evaluator = Evaluator::new();
        let results = evaluator.evaluate(&model).expect("test should succeed");

        let explainer = FormulaExplainer::new(&model, &results);
        let explanation = explainer.explain("cogs", &period).expect("test should succeed");

        let detailed = explanation.to_string_detailed();
        assert!(detailed.contains("cogs [2025Q1]"));
        assert!(detailed.contains("Formula: revenue * 0.4"));
        assert!(detailed.contains("revenue = 100000.00"));
    }

    #[test]
    fn test_explain_nonexistent_node() {
        let period = PeriodId::quarter(2025, 1);
        let period2 = PeriodId::quarter(2025, 2);
        let model = ModelBuilder::new("test")
            .periods("2025Q1..Q2", None)
            .expect("test should succeed")
            .value(
                "revenue",
                &[
                    (period, AmountOrScalar::scalar(100_000.0)),
                    (period2, AmountOrScalar::scalar(110_000.0)),
                ],
            )
            .build()
            .expect("test should succeed");

        let mut evaluator = Evaluator::new();
        let results = evaluator.evaluate(&model).expect("test should succeed");

        let explainer = FormulaExplainer::new(&model, &results);
        let result = explainer.explain("nonexistent", &period);

        assert!(result.is_err());
    }
}
