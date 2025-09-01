//! Option pricing models (pure model code shared across instruments).

pub mod binomial_tree;
pub mod black;
pub mod sabr;
pub mod short_rate_tree;
pub mod tree_framework;
pub mod trinomial_tree;

pub use binomial_tree::{BinomialTree, TreeType};
pub use black::{d1, d2, norm_cdf, norm_pdf};
pub use sabr::{SABRCalibrator, SABRModel, SABRParameters, SABRSmile};
pub use short_rate_tree::{short_rate_keys, ShortRateModel, ShortRateTree, ShortRateTreeConfig};
pub use tree_framework::{
    single_factor_equity_state, state_keys, two_factor_equity_rates_state, EvolutionParams,
    NodeState, StateVariables, TreeBranching, TreeGreeks, TreeModel, TreeParameters, TreeValuator,
};
pub use trinomial_tree::{TrinomialTree, TrinomialTreeType};
