//! Node specification and types.

use crate::types::AmountOrScalar;
use finstack_core::dates::PeriodId;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Specification for a single node (metric/line item) in the financial model.
///
/// A node can be:
/// - **Value**: Explicit values only
/// - **Calculated**: Formula-derived only
/// - **Mixed**: Value OR Forecast OR Formula (precedence: Value > Forecast > Formula)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeSpec {
    /// Unique identifier for this node
    pub node_id: String,

    /// Human-readable name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Node computation type
    pub node_type: NodeType,

    /// Explicit values per period (for Value and Mixed nodes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<IndexMap<PeriodId, AmountOrScalar>>,

    /// Forecast specifications (for Mixed nodes)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forecasts: Vec<ForecastSpec>,

    /// Formula text (for Calculated and Mixed nodes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formula_text: Option<String>,

    /// Where clause for conditional evaluation (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub where_text: Option<String>,

    /// Tags for grouping/filtering
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Additional metadata
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub meta: IndexMap<String, serde_json::Value>,
}

impl NodeSpec {
    /// Create a new node specification.
    pub fn new(node_id: impl Into<String>, node_type: NodeType) -> Self {
        Self {
            node_id: node_id.into(),
            name: None,
            node_type,
            values: None,
            forecasts: Vec::new(),
            formula_text: None,
            where_text: None,
            tags: Vec::new(),
            meta: IndexMap::new(),
        }
    }

    /// Set the human-readable name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add explicit values.
    pub fn with_values(mut self, values: IndexMap<PeriodId, AmountOrScalar>) -> Self {
        self.values = Some(values);
        self
    }

    /// Set the formula text.
    pub fn with_formula(mut self, formula: impl Into<String>) -> Self {
        self.formula_text = Some(formula.into());
        self
    }

    /// Add a forecast specification.
    pub fn with_forecast(mut self, forecast: ForecastSpec) -> Self {
        self.forecasts.push(forecast);
        self
    }

    /// Add tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

/// Node computation type.
///
/// Determines how a node's value is computed:
/// - **Value**: Only explicit values (actuals, assumptions)
/// - **Calculated**: Only formula-derived
/// - **Mixed**: Value OR Forecast OR Formula (precedence: Value > Forecast > Formula)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    /// Only explicit values
    Value,
    /// Only formula-derived
    Calculated,
    /// Value OR Forecast OR Formula (precedence: Value > Forecast > Formula)
    Mixed,
}

/// Forecast method specification.
///
/// Defines how to forecast future values for a node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForecastSpec {
    /// Forecast method
    pub method: ForecastMethod,

    /// Method-specific parameters
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub params: IndexMap<String, serde_json::Value>,
}

/// Available forecast methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForecastMethod {
    /// Carry last value forward
    ForwardFill,

    /// Compound growth: v[t] = v[t-1] * (1 + rate)
    GrowthPct,

    /// Sample from normal distribution (deterministic with seed)
    #[cfg(feature = "stats")]
    Normal,

    /// Sample from log-normal distribution (deterministic with seed)
    #[cfg(feature = "stats")]
    LogNormal,

    /// Explicit period overrides
    Override,

    /// Reference external time series
    TimeSeries,

    /// Seasonal pattern (additive/multiplicative)
    Seasonal,
}

