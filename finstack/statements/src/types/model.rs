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

    /// Capital structure specification (optional)
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

    /// Compute capital structure cashflows from instrument specifications.
    ///
    /// This method builds instruments from the model's capital structure specifications
    /// and aggregates their cashflows by period.
    ///
    /// # Arguments
    /// * `market_ctx` - Market context with discount/forward curves
    /// * `as_of` - Valuation date for pricing
    ///
    /// # Returns
    /// Aggregated cashflows by instrument and period, or None if no capital structure is defined
    ///
    /// # Example
    /// ```ignore
    /// use finstack_core::market_data::MarketContext;
    /// use finstack_core::dates::Date;
    ///
    /// let market_ctx = MarketContext::default();
    /// let as_of = Date::from_calendar_date(2025, time::Month::January, 1)?;
    /// let cashflows = model.compute_capital_structure_cashflows(&market_ctx, as_of)?;
    /// ```
    pub fn compute_capital_structure_cashflows(
        &self,
        market_ctx: &finstack_core::market_data::MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> crate::error::Result<Option<crate::capital_structure::CapitalStructureCashflows>> {
        // Return None if no capital structure is defined
        let cs_spec = match &self.capital_structure {
            Some(cs) => cs,
            None => return Ok(None),
        };

        // Build instruments from specifications
        use finstack_valuations::cashflow::traits::CashflowProvider;
        use std::sync::Arc;
        let mut instruments: IndexMap<String, Arc<dyn CashflowProvider + Send + Sync>> =
            IndexMap::new();

        for debt_spec in &cs_spec.debt_instruments {
            match debt_spec {
                DebtInstrumentSpec::Bond { id, .. } => {
                    let bond =
                        crate::capital_structure::integration::build_bond_from_spec(debt_spec)?;
                    instruments.insert(id.clone(), Arc::new(bond));
                }
                DebtInstrumentSpec::Swap { id, .. } => {
                    let swap =
                        crate::capital_structure::integration::build_swap_from_spec(debt_spec)?;
                    instruments.insert(id.clone(), Arc::new(swap));
                }
                DebtInstrumentSpec::Generic { id, .. } => {
                    // For generic instruments, we can't build them automatically yet
                    // This would need custom deserialization logic
                    return Err(crate::error::Error::capital_structure(format!(
                        "Cannot automatically compute cashflows for generic debt instrument '{}'. \
                         Generic instruments require manual cashflow specification.",
                        id
                    )));
                }
            }
        }

        // Aggregate cashflows by period
        let cashflows = crate::capital_structure::integration::aggregate_instrument_cashflows(
            &instruments,
            &self.periods,
            market_ctx,
            as_of,
        )?;

        Ok(Some(cashflows))
    }
}

fn default_schema_version() -> u32 {
    1
}

/// Capital structure specification.
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

/// Debt instrument specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DebtInstrumentSpec {
    /// Fixed-rate bond
    Bond {
        /// Instrument identifier
        id: String,
        /// Instrument specification (JSON)
        spec: serde_json::Value,
    },
    /// Interest rate swap
    Swap {
        /// Instrument identifier
        id: String,
        /// Instrument specification (JSON)
        spec: serde_json::Value,
    },
    /// Generic debt instrument (custom JSON spec)
    Generic {
        /// Instrument identifier
        id: String,
        /// Instrument specification (JSON)
        spec: serde_json::Value,
    },
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
