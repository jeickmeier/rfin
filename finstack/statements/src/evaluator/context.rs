//! Evaluation context that backs formula execution during model evaluation.
//!
//! The context owns per-period state such as evaluated node values,
//! historical lookups, and optional capital-structure cashflows. Evaluator
//! components update and query this structure while traversing the dependency
//! graph.

use crate::error::{Error, Result};
use crate::evaluator::results::EvalWarning;
use crate::types::NodeValueType;
use finstack_core::dates::{PeriodId, PeriodKind};
use indexmap::IndexMap;

/// Evaluation context for a single period.
///
/// Tracks node values for the current period and provides access to historical values.
#[derive(Debug, Clone)]
pub struct EvaluationContext {
    /// Current period being evaluated
    pub period_id: PeriodId,

    /// Period frequency (quarterly, monthly, etc.)
    pub period_kind: PeriodKind,

    /// Map of node_id → column index for the current period
    pub node_to_column: IndexMap<String, usize>,

    /// Historical results: period_id → (node_id → value)
    pub historical_results: IndexMap<PeriodId, IndexMap<String, f64>>,

    /// Current period results being built.
    /// Uses `Option<f64>` to distinguish between "not yet evaluated" (`None`) and "evaluated to NaN" (`Some(NaN)`).
    pub current_values: Vec<Option<f64>>,

    /// Track value types for each node (monetary vs scalar)
    pub node_value_types: IndexMap<String, NodeValueType>,

    /// Capital structure cashflows (optional)
    pub capital_structure_cashflows: Option<crate::capital_structure::CapitalStructureCashflows>,

    /// Warnings collected while evaluating this period
    pub warnings: Vec<EvalWarning>,
}

impl EvaluationContext {
    /// Create a new evaluation context for a period.
    ///
    /// # Arguments
    /// * `period_id` - Period currently being evaluated
    /// * `node_to_column` - Mapping from node identifiers to their column index
    /// * `historical_results` - Prior period results available for lag/lead lookups
    ///
    /// # Example
    ///
    /// ```rust
    /// # use finstack_statements::evaluator::EvaluationContext;
    /// # use finstack_core::dates::PeriodId;
    /// # use indexmap::IndexMap;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let period = PeriodId::quarter(2025, 1);
    /// let mut columns = IndexMap::new();
    /// columns.insert("revenue".to_string(), 0);
    /// let ctx = EvaluationContext::new(period, columns, IndexMap::new());
    /// assert_eq!(ctx.period_id, period);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(
        period_id: PeriodId,
        node_to_column: IndexMap<String, usize>,
        historical_results: IndexMap<PeriodId, IndexMap<String, f64>>,
    ) -> Self {
        let num_nodes = node_to_column.len();
        let period_kind = period_id.kind(); // Extract period frequency from period_id
        Self {
            period_id,
            period_kind,
            node_to_column,
            historical_results,
            current_values: vec![None; num_nodes],
            node_value_types: IndexMap::new(),
            capital_structure_cashflows: None,
            warnings: Vec::new(),
        }
    }

    /// Set capital structure cashflows for this context.
    ///
    /// Callers typically invoke this when the evaluator loads supplementary
    /// data from the capital structure module.
    pub fn with_capital_structure(
        mut self,
        cashflows: crate::capital_structure::CapitalStructureCashflows,
    ) -> Self {
        self.capital_structure_cashflows = Some(cashflows);
        self
    }

    /// Set the value for a node in the current period.
    ///
    /// Accepts any `f64` value, including `NaN`. Values are stored even when they
    /// originate from forecasts or capital-structure flows so that precedence
    /// resolution can make informed decisions later.
    ///
    /// Non-finite values (`NaN`, `Inf`, `-Inf`) are accepted but emit a warning
    /// so downstream consumers can detect propagation of bad values.
    ///
    /// # Arguments
    /// * `node_id` - Identifier of the node being updated
    /// * `value` - Numeric result to store for the current period
    pub fn set_value(&mut self, node_id: &str, value: f64) -> Result<()> {
        let idx = self
            .node_to_column
            .get(node_id)
            .ok_or_else(|| Error::node_not_found(node_id))?;

        // Finiteness validation: detect NaN / Inf early and emit a warning.
        // We still store the value so that downstream formulas can decide how
        // to handle it (e.g., coalesce, if-then-else guards), but the warning
        // makes it visible in results metadata.
        if !value.is_finite() {
            self.warnings.push(EvalWarning::NonFiniteValue {
                node_id: node_id.to_string(),
                period: self.period_id,
                value,
            });
        }

        self.current_values[*idx] = Some(value);
        Ok(())
    }

    /// Get the value for a node in the current period.
    ///
    /// Returns an error if the node has not been evaluated yet (value is `None`).
    /// Returns `Ok(NaN)` if the node was evaluated but resulted in `NaN`.
    ///
    /// # Arguments
    /// * `node_id` - Identifier to query
    pub fn get_value(&self, node_id: &str) -> Result<f64> {
        let idx = self
            .node_to_column
            .get(node_id)
            .ok_or_else(|| Error::node_not_found(node_id))?;

        match self.current_values[*idx] {
            Some(value) => Ok(value),
            None => Err(Error::eval(format!(
                "Node '{}' has not been evaluated yet in period {}. \
                 This usually indicates a circular dependency or missing value.",
                node_id, self.period_id
            ))),
        }
    }

    /// Get historical value for a node at a specific period.
    ///
    /// # Arguments
    /// * `node_id` - Identifier to query
    /// * `period_id` - Historical period to look up
    pub fn get_historical_value(&self, node_id: &str, period_id: &PeriodId) -> Option<f64> {
        self.historical_results
            .get(period_id)
            .and_then(|period_results| period_results.get(node_id).copied())
    }

    /// Get capital structure value for the current period.
    ///
    /// # Arguments
    /// * `component` - Component type: "interest_expense", "interest_expense_cash", "interest_expense_pik",
    ///   "principal_payment", or "debt_balance"
    /// * `instrument_or_total` - Instrument ID or "total" for aggregate
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use finstack_statements::evaluator::EvaluationContext;
    ///
    /// # fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    /// # let context: EvaluationContext = unimplemented!("obtained during evaluation");
    /// // Total interest (cash + PIK)
    /// let total_interest = context.get_cs_value("interest_expense", "total")?;
    ///
    /// // Cash interest only
    /// let cash_interest = context.get_cs_value("interest_expense_cash", "total")?;
    ///
    /// // PIK interest only
    /// let pik_interest = context.get_cs_value("interest_expense_pik", "total")?;
    /// # let _ = (total_interest, cash_interest, pik_interest);
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_cs_value(&self, component: &str, instrument_or_total: &str) -> Result<f64> {
        let cs_cashflows = self
            .capital_structure_cashflows
            .as_ref()
            .ok_or_else(|| Error::capital_structure("No capital structure defined in model"))?;

        let value = if instrument_or_total == "total" {
            // Get total for all instruments
            match component {
                "interest_expense" => cs_cashflows.get_total_interest(&self.period_id),
                "interest_expense_cash" => cs_cashflows.get_total_interest_cash(&self.period_id),
                "interest_expense_pik" => cs_cashflows.get_total_interest_pik(&self.period_id),
                "principal_payment" => cs_cashflows.get_total_principal(&self.period_id),
                "debt_balance" => cs_cashflows.get_total_debt_balance(&self.period_id),
                _ => return Err(Error::capital_structure(format!(
                    "Unknown capital structure component: {}. Expected: interest_expense, interest_expense_cash, interest_expense_pik, principal_payment, or debt_balance",
                    component
                ))),
            }
        } else {
            // Get value for specific instrument
            match component {
                "interest_expense" => cs_cashflows.get_interest(instrument_or_total, &self.period_id),
                "interest_expense_cash" => cs_cashflows.get_interest_cash(instrument_or_total, &self.period_id),
                "interest_expense_pik" => cs_cashflows.get_interest_pik(instrument_or_total, &self.period_id),
                "principal_payment" => cs_cashflows.get_principal(instrument_or_total, &self.period_id),
                "debt_balance" => cs_cashflows.get_debt_balance(instrument_or_total, &self.period_id),
                _ => return Err(Error::capital_structure(format!(
                    "Unknown capital structure component: {}. Expected: interest_expense, interest_expense_cash, interest_expense_pik, principal_payment, or debt_balance",
                    component
                ))),
            }
        };

        value
    }

    /// Get all results as a map.
    ///
    /// Only includes nodes that have been evaluated (Some value).
    /// Nodes with None are skipped (should not happen in valid evaluation).
    pub fn into_results(self) -> (IndexMap<String, f64>, Vec<EvalWarning>) {
        let mut results = IndexMap::new();
        for (node_id, idx) in &self.node_to_column {
            if let Some(value) = self.current_values[*idx] {
                results.insert(node_id.clone(), value);
            }
        }
        (results, self.warnings)
    }

    /// Record a warning for the current period.
    pub fn push_warning(&mut self, warning: EvalWarning) {
        self.warnings.push(warning);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let mut node_to_column = IndexMap::new();
        node_to_column.insert("revenue".to_string(), 0);
        node_to_column.insert("cogs".to_string(), 1);

        let ctx =
            EvaluationContext::new(PeriodId::quarter(2025, 1), node_to_column, IndexMap::new());

        assert_eq!(ctx.current_values.len(), 2);
    }

    #[test]
    fn test_set_and_get_value() {
        let mut node_to_column = IndexMap::new();
        node_to_column.insert("revenue".to_string(), 0);

        let mut ctx =
            EvaluationContext::new(PeriodId::quarter(2025, 1), node_to_column, IndexMap::new());

        ctx.set_value("revenue", 100_000.0)
            .expect("test should succeed");
        assert_eq!(
            ctx.get_value("revenue").expect("test should succeed"),
            100_000.0
        );
    }

    #[test]
    fn test_get_historical_value() {
        let mut node_to_column = IndexMap::new();
        node_to_column.insert("revenue".to_string(), 0);

        let mut historical = IndexMap::new();
        let mut q1_results = IndexMap::new();
        q1_results.insert("revenue".to_string(), 100_000.0);
        historical.insert(PeriodId::quarter(2025, 1), q1_results);

        let ctx = EvaluationContext::new(PeriodId::quarter(2025, 2), node_to_column, historical);

        let value = ctx.get_historical_value("revenue", &PeriodId::quarter(2025, 1));
        assert_eq!(value, Some(100_000.0));
    }
}
