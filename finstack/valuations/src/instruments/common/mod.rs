//! Common functionality shared across multiple instruments.
//!
//! This module contains utilities, models, and types that are used
//! by multiple instrument implementations, including:
//! - Core instrument traits (Instrument)
//! - NPV calculation interfaces (Discountable)
//! - Option pricing models (Black-Scholes, binomial/trinomial trees, SABR)
//! - Common helper functions
//! - Shared data structures and enums

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

// Generic metric calculators to reduce duplication
pub mod metrics;

// Common pricing patterns and infrastructure
pub mod pricing;

// Re-export commonly used types for convenience
pub use discountable::Discountable;
pub use helpers::{build_with_metrics_dyn, validate_currency_consistency};
pub use metrics::{GenericBucketedDv01, GenericBucketedDv01WithContext, HasDiscountCurve};
pub use models::{
    d1, d2, norm_cdf, norm_pdf, short_rate_keys, single_factor_equity_state, state_keys,
    two_factor_equity_rates_state, BinomialTree, EvolutionParams, NodeState, SABRCalibrator,
    SABRModel, SABRParameters, SABRSmile, ShortRateModel, ShortRateTree, ShortRateTreeConfig,
    StateVariables, TreeBranching, TreeGreeks, TreeModel, TreeParameters, TreeType, TreeValuator,
    TrinomialTree, TrinomialTreeType,
};
pub use parameters::{
    BasisSwapLeg, ContractSpec, CreditParams, EquityOptionParams, EquityUnderlyingParams,
    ExerciseStyle, FinancingLegSpec, FixedLegSpec, FloatLegSpec, FxOptionParams,
    FxUnderlyingParams, IndexUnderlyingParams, InterestRateOptionParams, OptionMarketParams,
    OptionType, ParRateMethod, PayReceive, PremiumLegSpec, ProtectionLegSpec, ScheduleSpec,
    SettlementType, TotalReturnLegSpec, UnderlyingParams,
};
pub use pricing::{GenericDiscountingPricer, GenericInstrumentPricer};
pub use traits::{Attributes, Instrument};
