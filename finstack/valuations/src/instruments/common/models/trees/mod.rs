//! Tree-based pricing models for American, Bermudan, and path-dependent options.
//!
//! Provides implementations of binomial, trinomial, and multi-factor tree
//! methods for pricing options with early exercise and complex payoffs.

pub mod binomial_tree;
pub mod multi_factor_tree;
pub mod short_rate_tree;
pub mod tree_framework;
pub mod trinomial_tree;
pub mod two_factor_binomial;
pub mod two_factor_rates_credit;

pub use binomial_tree::{BinomialTree, TreeType};
pub use short_rate_tree::{short_rate_keys, ShortRateModel, ShortRateTree, ShortRateTreeConfig};
pub use tree_framework::{
    single_factor_equity_state, state_keys, two_factor_equity_rates_state, BarrierSpec,
    BarrierStyle, EvolutionParams, NodeState, StateVariables, TreeBranching, TreeGreeks, TreeModel,
    TreeParameters, TreeValuator,
};
pub use trinomial_tree::{TrinomialTree, TrinomialTreeType};
