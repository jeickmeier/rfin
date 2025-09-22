//! Common functionality shared across multiple instruments.
//!
//! This module contains utilities, models, and types that are used
//! by multiple instrument implementations, including:
//! - Core instrument traits (Instrument, Attributable)
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

// Common parameter types shared across instruments
pub mod parameters;

// Option pricing models and frameworks
pub mod models;

// Re-export commonly used types for convenience
pub use discountable::Discountable;
pub use helpers::{build_with_metrics_dyn, validate_currency_consistency};
pub use pricer::{
    has_pricer,
    has_pricer_for_key,
    price_with_pricer_or,
    push_pricer,
    push_pricer_for_key,
    register_pricer,
    register_pricer_for_key,
    set_default_pricer,
    with_pricer,
    with_pricer_for_key,
    Pricer,
};
pub use models::{
    d1, d2, norm_cdf, norm_pdf, short_rate_keys, single_factor_equity_state, state_keys,
    two_factor_equity_rates_state, BinomialTree, EvolutionParams, NodeState, SABRCalibrator,
    SABRModel, SABRParameters, SABRSmile, ShortRateModel, ShortRateTree, ShortRateTreeConfig,
    StateVariables, TreeBranching, TreeGreeks, TreeModel, TreeParameters, TreeType, TreeValuator,
    TrinomialTree, TrinomialTreeType,
};
pub use parameters::{
    BasisSwapLeg, CdsSettlementType, ContractSpec, CreditParams, EquityOptionParams,
    EquityUnderlyingParams, ExerciseStyle, FinancingLegSpec, FixedLegSpec, FloatLegSpec,
    FxOptionParams, FxUnderlyingParams, IndexUnderlyingParams, InterestRateOptionParams,
    OptionMarketParams, OptionType, ParRateMethod, PayReceive, PremiumLegSpec, ProtectionLegSpec,
    ScheduleSpec, SettlementType, TotalReturnLegSpec, UnderlyingParams,
};
pub use traits::{Attributable, Attributes, Instrument};

// Centralized pricer trait and registry
mod pricer;
