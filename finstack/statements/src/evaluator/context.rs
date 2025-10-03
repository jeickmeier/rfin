//! Evaluation context for statement formulas.

use crate::error::{Error, Result};
use finstack_core::dates::PeriodId;
use indexmap::IndexMap;

/// Evaluation context for a single period.
///
/// Tracks node values for the current period and provides access to historical values.
#[derive(Debug)]
pub struct StatementContext {
    /// Current period being evaluated
    pub period_id: PeriodId,

    /// Map of node_id → column index for the current period
    pub node_to_column: IndexMap<String, usize>,

    /// Historical results: period_id → (node_id → value)
    pub historical_results: IndexMap<PeriodId, IndexMap<String, f64>>,

    /// Current period results being built
    pub current_values: Vec<f64>,
}

impl StatementContext {
    /// Create a new evaluation context for a period.
    pub fn new(
        period_id: PeriodId,
        node_to_column: IndexMap<String, usize>,
        historical_results: IndexMap<PeriodId, IndexMap<String, f64>>,
    ) -> Self {
        let num_nodes = node_to_column.len();
        Self {
            period_id,
            node_to_column,
            historical_results,
            current_values: vec![f64::NAN; num_nodes],
        }
    }

    /// Set the value for a node in the current period.
    pub fn set_value(&mut self, node_id: &str, value: f64) -> Result<()> {
        let idx = self
            .node_to_column
            .get(node_id)
            .ok_or_else(|| Error::node_not_found(node_id))?;
        self.current_values[*idx] = value;
        Ok(())
    }

    /// Get the value for a node in the current period.
    pub fn get_value(&self, node_id: &str) -> Result<f64> {
        let idx = self
            .node_to_column
            .get(node_id)
            .ok_or_else(|| Error::node_not_found(node_id))?;
        let value = self.current_values[*idx];

        if value.is_nan() {
            Err(Error::eval(format!(
                "Node '{}' has not been evaluated yet in period {}",
                node_id, self.period_id
            )))
        } else {
            Ok(value)
        }
    }

    /// Get historical value for a node at a specific period.
    pub fn get_historical_value(&self, node_id: &str, period_id: &PeriodId) -> Option<f64> {
        self.historical_results
            .get(period_id)
            .and_then(|period_results| period_results.get(node_id).copied())
    }

    /// Get all results as a map.
    pub fn into_results(self) -> IndexMap<String, f64> {
        let mut results = IndexMap::new();
        for (node_id, idx) in &self.node_to_column {
            results.insert(node_id.clone(), self.current_values[*idx]);
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let mut node_to_column = IndexMap::new();
        node_to_column.insert("revenue".to_string(), 0);
        node_to_column.insert("cogs".to_string(), 1);

        let ctx =
            StatementContext::new(PeriodId::quarter(2025, 1), node_to_column, IndexMap::new());

        assert_eq!(ctx.current_values.len(), 2);
    }

    #[test]
    fn test_set_and_get_value() {
        let mut node_to_column = IndexMap::new();
        node_to_column.insert("revenue".to_string(), 0);

        let mut ctx =
            StatementContext::new(PeriodId::quarter(2025, 1), node_to_column, IndexMap::new());

        ctx.set_value("revenue", 100_000.0).unwrap();
        assert_eq!(ctx.get_value("revenue").unwrap(), 100_000.0);
    }

    #[test]
    fn test_get_historical_value() {
        let mut node_to_column = IndexMap::new();
        node_to_column.insert("revenue".to_string(), 0);

        let mut historical = IndexMap::new();
        let mut q1_results = IndexMap::new();
        q1_results.insert("revenue".to_string(), 100_000.0);
        historical.insert(PeriodId::quarter(2025, 1), q1_results);

        let ctx = StatementContext::new(PeriodId::quarter(2025, 2), node_to_column, historical);

        let value = ctx.get_historical_value("revenue", &PeriodId::quarter(2025, 1));
        assert_eq!(value, Some(100_000.0));
    }
}
