//! Financial model specification types.

use crate::types::NodeSpec;
use finstack_core::dates::Period;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Top-level financial model specification.
///
/// This is the wire format for a complete financial statement model.
/// It can be serialized to/from JSON for storage and interchange.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinancialModelSpec {
    /// Unique model identifier
    pub id: String,

    /// Ordered list of periods (quarters, months, etc.)
    pub periods: Vec<Period>,

    /// Map of node_id → NodeSpec
    pub nodes: IndexMap<String, NodeSpec>,

    /// Capital structure specification (optional, feature-gated)
    #[cfg(feature = "capital_structure")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capital_structure: Option<CapitalStructureSpec>,

    /// Additional metadata
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub meta: IndexMap<String, serde_json::Value>,

    /// Schema version for forward compatibility
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
}

impl FinancialModelSpec {
    /// Create a new model specification.
    pub fn new(id: impl Into<String>, periods: Vec<Period>) -> Self {
        Self {
            id: id.into(),
            periods,
            nodes: IndexMap::new(),
            #[cfg(feature = "capital_structure")]
            capital_structure: None,
            meta: IndexMap::new(),
            schema_version: 1,
        }
    }

    /// Add a node to the model.
    pub fn add_node(&mut self, node: NodeSpec) {
        self.nodes.insert(node.node_id.clone(), node);
    }

    /// Get a node by ID.
    pub fn get_node(&self, node_id: &str) -> Option<&NodeSpec> {
        self.nodes.get(node_id)
    }

    /// Get a mutable reference to a node by ID.
    pub fn get_node_mut(&mut self, node_id: &str) -> Option<&mut NodeSpec> {
        self.nodes.get_mut(node_id)
    }

    /// Check if a node exists.
    pub fn has_node(&self, node_id: &str) -> bool {
        self.nodes.contains_key(node_id)
    }
}

fn default_schema_version() -> u32 {
    1
}

/// Capital structure specification (feature-gated).
#[cfg(feature = "capital_structure")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapitalStructureSpec {
    /// Debt instruments (bonds, loans, swaps)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub debt_instruments: Vec<DebtInstrumentSpec>,

    /// Equity instruments (optional, future expansion)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub equity_instruments: Vec<serde_json::Value>,

    /// Additional metadata
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub meta: IndexMap<String, serde_json::Value>,
}

/// Debt instrument specification (feature-gated).
#[cfg(feature = "capital_structure")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DebtInstrumentSpec {
    /// Fixed-rate bond
    Bond { id: String, spec: serde_json::Value },
    /// Interest rate swap
    Swap { id: String, spec: serde_json::Value },
    /// Generic debt instrument (custom JSON spec)
    Generic { id: String, spec: serde_json::Value },
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::build_periods;

    #[test]
    fn test_model_spec_creation() {
        let periods = build_periods("2025Q1..Q4", None).unwrap().periods;
        let model = FinancialModelSpec::new("test_model", periods.clone());

        assert_eq!(model.id, "test_model");
        assert_eq!(model.periods.len(), 4);
        assert_eq!(model.schema_version, 1);
        assert!(model.nodes.is_empty());
    }

    #[test]
    fn test_add_and_get_node() {
        let periods = build_periods("2025Q1..Q2", None).unwrap().periods;
        let mut model = FinancialModelSpec::new("test", periods);

        let node = NodeSpec::new("revenue", crate::types::NodeType::Value);
        model.add_node(node);

        assert!(model.has_node("revenue"));
        assert_eq!(model.get_node("revenue").unwrap().node_id, "revenue");
    }

    #[test]
    fn test_serialization_roundtrip() {
        let periods = build_periods("2025Q1..Q2", None).unwrap().periods;
        let model = FinancialModelSpec::new("test", periods);

        let json = serde_json::to_string(&model).unwrap();
        let deserialized: FinancialModelSpec = serde_json::from_str(&json).unwrap();

        assert_eq!(model, deserialized);
    }
}
