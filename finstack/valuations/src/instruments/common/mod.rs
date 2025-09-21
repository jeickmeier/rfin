//! Common functionality shared across multiple instruments.
//!
//! This module contains utilities, models, and types that are used
//! by multiple instrument implementations, including:
//! - Core instrument traits (Instrument, Priceable, Attributable)
//! - Implementation macros for reducing boilerplate
//! - NPV calculation interfaces (Discountable)
//! - Option pricing models (Black-Scholes, binomial/trinomial trees, SABR)
//! - Common helper functions
//! - Shared data structures and enums

// Macro infrastructure for reducing boilerplate
#[macro_use]
pub mod macros;

// Core instrument traits and metadata
pub mod traits;

// NPV calculation interface
pub mod discountable;

// Shared utilities and helper functions
pub mod helpers;

// Option pricing models and frameworks
pub mod models;

// Re-export commonly used types for convenience
pub use discountable::Discountable;
pub use helpers::{build_with_metrics_dyn, validate_currency_consistency};
pub use models::{
    BinomialTree, ExerciseStyle, OptionType, SettlementType, TreeType,
    SABRCalibrator, SABRModel, SABRParameters, SABRSmile,
    d1, d2, norm_cdf, norm_pdf,
    short_rate_keys, ShortRateModel, ShortRateTree, ShortRateTreeConfig,
    single_factor_equity_state, state_keys, two_factor_equity_rates_state, 
    EvolutionParams, NodeState, StateVariables, TreeBranching, TreeGreeks, 
    TreeModel, TreeParameters, TreeValuator,
    TrinomialTree, TrinomialTreeType, OptionMarketParams
};
pub use traits::{Attributable, Attributes, Instrument, Priceable};
