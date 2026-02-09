//! Results types for statement evaluation.

use crate::types::NodeValueType;
use finstack_core::dates::PeriodId;
use finstack_core::money::Money;
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
/// Results now support dual storage:
/// - `nodes`: f64 values for backward compatibility
/// - `monetary_nodes`: Money values for currency-aware monetary nodes
/// - `node_value_types`: Track which nodes are monetary vs scalar
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
/// let result = evaluator.evaluate(&model)?;
/// assert!(result.get("gross_profit", &PeriodId::quarter(2025, 1)).is_some());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatementResult {
    /// Map of node_id → (period_id → value) [f64 for backward compatibility]
    pub nodes: IndexMap<String, IndexMap<PeriodId, f64>>,

    /// Map of node_id → (period_id → Money) for monetary nodes
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub monetary_nodes: IndexMap<String, IndexMap<PeriodId, Money>>,

    /// Track value types for each node
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub node_value_types: IndexMap<String, NodeValueType>,

    /// Metadata about the evaluation
    pub meta: ResultsMeta,
}

/// Metadata about evaluation results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultsMeta {
    /// Evaluation time in milliseconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_time_ms: Option<u64>,

    /// Number of nodes evaluated
    pub num_nodes: usize,

    /// Number of periods evaluated
    pub num_periods: usize,

    /// Numeric mode used for evaluation
    #[serde(default)]
    pub numeric_mode: NumericMode,

    /// Rounding context (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rounding_context: Option<finstack_core::config::RoundingContext>,

    /// Whether parallel evaluation was used
    #[serde(default)]
    pub parallel: bool,

    /// Warnings encountered during evaluation (division by zero, NaN propagation, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<EvalWarning>,
}

impl Default for ResultsMeta {
    fn default() -> Self {
        Self {
            eval_time_ms: None,
            num_nodes: 0,
            num_periods: 0,
            numeric_mode: NumericMode::Float64,
            rounding_context: None,
            parallel: false,
            warnings: Vec::new(),
        }
    }
}

/// Numeric mode used for evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NumericMode {
    /// f64 floating-point mode (current default)
    #[default]
    Float64,
    /// Decimal fixed-point mode (future)
    Decimal,
}

impl StatementResult {
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

    /// Get the Money value for a monetary node at a specific period.
    ///
    /// # Arguments
    /// * `node_id` - Identifier of the monetary node (e.g., `"revenue"`)
    /// * `period_id` - Period key
    ///
    /// # Returns
    /// `Some(Money)` if the node is monetary and has a value for this period, otherwise `None`.
    pub fn get_money(&self, node_id: &str, period_id: &PeriodId) -> Option<Money> {
        self.monetary_nodes
            .get(node_id)
            .and_then(|period_map| period_map.get(period_id).copied())
    }

    /// Get the scalar value for a non-monetary node at a specific period.
    ///
    /// # Arguments
    /// * `node_id` - Identifier of the scalar node (e.g., `"gross_margin_pct"`)
    /// * `period_id` - Period key
    ///
    /// # Returns
    /// `Some(f64)` if the node is scalar and has a value for this period, otherwise `None`.
    pub fn get_scalar(&self, node_id: &str, period_id: &PeriodId) -> Option<f64> {
        // Check if this is a scalar node (not monetary)
        if let Some(NodeValueType::Scalar) = self.node_value_types.get(node_id) {
            self.get(node_id, period_id)
        } else {
            None
        }
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
        super::export::to_polars_long(self)
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
        super::export::to_polars_long_filtered(self, node_filter)
    }

    /// Export to Polars wide format DataFrame.
    ///
    /// Schema: `(period_id: Utf8, <node1>: Float64, <node2>: Float64, ...)`
    #[cfg(feature = "dataframes")]
    pub fn to_polars_wide(&self) -> Result<polars::prelude::DataFrame> {
        super::export::to_polars_wide(self)
    }
}

/// Warning emitted during evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvalWarning {
    /// Division by zero encountered
    DivisionByZero {
        /// Identifier of the node that triggered the warning.
        node_id: String,
        /// Period in which the warning occurred.
        period: PeriodId,
    },
    /// NaN value bubbled up to a node result
    NaNPropagated {
        /// Identifier of the node that produced the NaN value.
        node_id: String,
        /// Period in which the warning occurred.
        period: PeriodId,
    },
    /// Non-finite value (NaN, Inf, -Inf) detected when storing a node result.
    ///
    /// This warning is emitted by the finiteness validation pipeline so that
    /// consumers can identify which node/period introduced bad values.
    NonFiniteValue {
        /// Identifier of the node that produced the non-finite value.
        node_id: String,
        /// Period in which the warning occurred.
        period: PeriodId,
        /// The actual non-finite value (NaN, Inf, or -Inf).
        value: f64,
    },
}
