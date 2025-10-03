//! Results types for statement evaluation.

use finstack_core::dates::PeriodId;
use indexmap::IndexMap;

#[cfg(feature = "polars_export")]
use crate::error::Result;

/// Results from evaluating a financial model.
#[derive(Debug, Clone, Default)]
pub struct Results {
    /// Map of node_id → (period_id → value)
    pub nodes: IndexMap<String, IndexMap<PeriodId, f64>>,

    /// Metadata about the evaluation
    pub meta: ResultsMeta,
}

/// Metadata about evaluation results.
#[derive(Debug, Clone, Default)]
pub struct ResultsMeta {
    /// Evaluation time in milliseconds
    pub eval_time_ms: Option<u64>,

    /// Number of nodes evaluated
    pub num_nodes: usize,

    /// Number of periods evaluated
    pub num_periods: usize,

    /// Was evaluation parallel?
    pub parallel: bool,
}

impl Results {
    /// Create empty results.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the value for a node at a specific period.
    ///
    /// Returns `None` if the node or period is not found.
    pub fn get(&self, node_id: &str, period_id: &PeriodId) -> Option<f64> {
        self.nodes
            .get(node_id)
            .and_then(|period_map| period_map.get(period_id).copied())
    }

    /// Get all period values for a specific node.
    pub fn get_node(&self, node_id: &str) -> Option<&IndexMap<PeriodId, f64>> {
        self.nodes.get(node_id)
    }

    /// Get an iterator over all periods for a node.
    pub fn all_periods(&self, node_id: &str) -> impl Iterator<Item = (&PeriodId, f64)> + '_ {
        self.get_node(node_id)
            .into_iter()
            .flat_map(|map| map.iter().map(|(k, v)| (k, *v)))
    }

    /// Get value or default.
    pub fn get_or(&self, node_id: &str, period: &PeriodId, default: f64) -> f64 {
        self.get(node_id, period).unwrap_or(default)
    }

    /// Export to Polars long format DataFrame.
    ///
    /// Schema: `(node_id: Utf8, period_id: Utf8, value: Float64)`
    #[cfg(feature = "polars_export")]
    pub fn to_polars_long(&self) -> Result<polars::prelude::DataFrame> {
        crate::results::export::to_polars_long(self)
    }

    /// Export to Polars long format with node filtering.
    ///
    /// If `node_filter` is empty, all nodes are included.
    #[cfg(feature = "polars_export")]
    pub fn to_polars_long_filtered(&self, node_filter: &[&str]) -> Result<polars::prelude::DataFrame> {
        crate::results::export::to_polars_long_filtered(self, node_filter)
    }

    /// Export to Polars wide format DataFrame.
    ///
    /// Schema: `(period_id: Utf8, <node1>: Float64, <node2>: Float64, ...)`
    #[cfg(feature = "polars_export")]
    pub fn to_polars_wide(&self) -> Result<polars::prelude::DataFrame> {
        crate::results::export::to_polars_wide(self)
    }
}

