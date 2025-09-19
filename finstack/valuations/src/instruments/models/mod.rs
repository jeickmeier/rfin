//! Option pricing models (pure model code shared across instruments).

pub mod binomial_tree;
pub mod black;
pub mod sabr;
pub mod short_rate_tree;
pub mod tree_framework;
pub mod trinomial_tree;
pub mod option_market_params;
pub mod option_types;

pub use binomial_tree::{BinomialTree, TreeType};
pub use black::{d1, d2};
pub use finstack_core::math::{norm_cdf, norm_pdf};
pub use sabr::{SABRCalibrator, SABRModel, SABRParameters, SABRSmile};
pub use short_rate_tree::{short_rate_keys, ShortRateModel, ShortRateTree, ShortRateTreeConfig};
pub use tree_framework::{
    single_factor_equity_state, state_keys, two_factor_equity_rates_state, EvolutionParams,
    NodeState, StateVariables, TreeBranching, TreeGreeks, TreeModel, TreeParameters, TreeValuator,
};
pub use trinomial_tree::{TrinomialTree, TrinomialTreeType};
pub use option_market_params::OptionMarketParams;
pub use option_types::{ExerciseStyle, OptionType, SettlementType};
