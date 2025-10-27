//! Results types for statement evaluation.

use finstack_core::dates::PeriodId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[cfg(feature = "dataframes")]
use crate::error::Result;

/// Results from evaluating a financial model.
///
/// Values are stored as an [`IndexMap`] keyed by node identifier so you can
/// preserve declaration order when presenting them. Helper methods make it easy
/// to access per-period values or export to Polars.
///
/// # Example
///
/// ```rust
/// # use finstack_statements::builder::ModelBuilder;
/// # use finstack_statements::evaluator::Evaluator;
/// # use finstack_core::dates::PeriodId;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let model = ModelBuilder::new("demo")
///     .periods("2025Q1..Q2", None)?
///     .value("revenue", &[
///         (PeriodId::quarter(2025, 1), 100_000.0.into()),
///         (PeriodId::quarter(2025, 2), 105_000.0.into()),
///     ])
///     .compute("gross_profit", "revenue * 0.6")?
///     .build()?;
///
/// let mut evaluator = Evaluator::new();
/// let results = evaluator.evaluate(&model)?;
/// assert!(results.get("gross_profit", &PeriodId::quarter(2025, 1)).is_some());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Results {
    /// Map of node_id → (period_id → value)
    pub nodes: IndexMap<String, IndexMap<PeriodId, f64>>,

    /// Metadata about the evaluation
    pub meta: ResultsMeta,
}

/// Metadata about evaluation results.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResultsMeta {
    /// Evaluation time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_time_ms: Option<u64>,

    /// Number of nodes evaluated
    pub num_nodes: usize,

    /// Number of periods evaluated
    pub num_periods: usize,
}

impl Results {
    /// Create empty results.
    ///
    /// Useful in tests or when you need a placeholder structure before running
    /// an evaluation.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the value for a node at a specific period.
    ///
    /// # Arguments
    /// * `node_id` - Identifier of the node (e.g., `"revenue"`)
    /// * `period_id` - Period key returned by the evaluator or builder
    ///
    /// # Returns
    /// `Some(value)` if the datapoint exists, otherwise `None`.
    pub fn get(&self, node_id: &str, period_id: &PeriodId) -> Option<f64> {
        self.nodes
            .get(node_id)
            .and_then(|period_map| period_map.get(period_id).copied())
    }

    /// Get all period values for a specific node.
    ///
    /// # Arguments
    /// * `node_id` - Identifier to look up
    pub fn get_node(&self, node_id: &str) -> Option<&IndexMap<PeriodId, f64>> {
        self.nodes.get(node_id)
    }

    /// Get an iterator over all periods for a node.
    ///
    /// # Arguments
    /// * `node_id` - Identifier to iterate over
    pub fn all_periods(&self, node_id: &str) -> impl Iterator<Item = (&PeriodId, f64)> + '_ {
        self.get_node(node_id)
            .into_iter()
            .flat_map(|map| map.iter().map(|(k, v)| (k, *v)))
    }

    /// Get value or default.
    ///
    /// # Arguments
    /// * `node_id` - Identifier to look up
    /// * `period` - Period identifier
    /// * `default` - Value to return when the datapoint is missing
    pub fn get_or(&self, node_id: &str, period: &PeriodId, default: f64) -> f64 {
        self.get(node_id, period).unwrap_or(default)
    }

    /// Export to Polars long format DataFrame.
    ///
    /// Schema: `(node_id: Utf8, period_id: Utf8, value: Float64)`
    #[cfg(feature = "dataframes")]
    pub fn to_polars_long(&self) -> Result<polars::prelude::DataFrame> {
        crate::results::export::to_polars_long(self)
    }

    /// Export to Polars long format with node filtering.
    ///
    /// If `node_filter` is empty, all nodes are included.
    ///
    /// # Arguments
    /// * `node_filter` - Optional list of node identifiers to keep
    #[cfg(feature = "dataframes")]
    pub fn to_polars_long_filtered(
        &self,
        node_filter: &[&str],
    ) -> Result<polars::prelude::DataFrame> {
        crate::results::export::to_polars_long_filtered(self, node_filter)
    }

    /// Export to Polars wide format DataFrame.
    ///
    /// Schema: `(period_id: Utf8, <node1>: Float64, <node2>: Float64, ...)`
    #[cfg(feature = "dataframes")]
    pub fn to_polars_wide(&self) -> Result<polars::prelude::DataFrame> {
        crate::results::export::to_polars_wide(self)
    }
}
