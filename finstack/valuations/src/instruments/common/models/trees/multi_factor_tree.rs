//! Generic multi-factor tree scaffold to support combinations such as
//! rates/credit, rates/equity, rates/commodity, FX/rates, etc.
//!
//! This is an initial scaffold that integrates with the generic `TreeModel`
//! and `TreeValuator` traits. The concrete factor evolution and correlation
//! handling will be implemented incrementally.
//!
//! NOTE: This is intentionally minimal. Barrier support and performance
//!       optimizations will be added as needed for specific use cases.

use finstack_core::market_data::context::MarketContext;
use finstack_core::{Error, Result};

use super::tree_framework::{NodeState, StateVariables, TreeModel, TreeValuator};

/// Factor type to guide specialized handling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FactorType {
    Equity(String),
    InterestRate,
    CreditSpread,
    Commodity(String),
    FxPair(String),
}

/// Initial placeholder for multi-factor tree configuration.
#[derive(Clone, Debug)]
pub struct MultiFactorConfig {
    /// Number of time steps shared across factors.
    pub steps: usize,
    /// Correlation matrix (row-major N×N). Dimensions must match `factors.len()`.
    pub correlation: Vec<f64>,
    /// Factor descriptors (ordering must match correlation).
    pub factor_types: Vec<FactorType>,
}

impl Default for MultiFactorConfig {
    fn default() -> Self {
        Self {
            steps: 100,
            correlation: Vec::new(),
            factor_types: Vec::new(),
        }
    }
}

/// Multi-factor tree placeholder using a simple product-state recombining policy.
///
/// In the initial version, we do not expand the full N-dimensional lattice.
/// Instead, we provide a single-step hand-off so instruments can consume
/// correlated shocks via user-supplied `TreeValuator` logic.
#[derive(Clone, Debug)]
pub struct MultiFactorTree {
    pub config: MultiFactorConfig,
}

impl MultiFactorTree {
    pub fn new(config: MultiFactorConfig) -> Self {
        Self { config }
    }
}

impl TreeModel for MultiFactorTree {
    fn price<V: TreeValuator>(
        &self,
        initial_vars: StateVariables,
        time_to_maturity: f64,
        market_context: &MarketContext,
        valuator: &V,
    ) -> Result<f64> {
        if self.config.steps == 0 {
            return Err(Error::Internal);
        }

        // Minimal placeholder: evaluate only at maturity with provided vars.
        // Future work: expand a correlated multi-factor lattice.
        let mut vars = initial_vars.clone();
        vars.insert("time", time_to_maturity);
        let terminal_state =
            NodeState::new(self.config.steps, time_to_maturity, vars, market_context);
        valuator.value_at_maturity(&terminal_state)
    }
}
