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
pub mod fx_dates;
pub mod parameters;

// Option pricing models and frameworks (includes closed-form, volatility, and tree models)
pub mod models;

// Monte Carlo pricing engine (requires mc feature)
#[cfg(feature = "mc")]
pub mod mc;

// Common pricing patterns and infrastructure
pub mod pricing;

// Periodized present value calculations
pub mod period_pv;

// Re-export commonly used types for convenience
pub use discountable::Discountable;
pub use fx_dates::{
    add_joint_business_days_with_calendars, adjust_joint_calendar,
    adjust_joint_calendar_with_calendars, roll_spot_date, ResolvedCalendarPair,
};
pub use helpers::{build_with_metrics_dyn, instrument_to_arc, validate_currency_consistency};
pub use models::{
    d1, d1_d2, d1_d2_black76, d2, norm_cdf, norm_pdf, short_rate_keys, single_factor_equity_state,
    state_keys, two_factor_equity_rates_state, BinomialTree, EvolutionParams, NodeState,
    SABRCalibrator, SABRModel, SABRParameters, SABRSmile, ShortRateModel, ShortRateTree,
    ShortRateTreeConfig, StateVariables, TreeBranching, TreeGreeks, TreeModel, TreeParameters,
    TreeType, TreeValuator, TrinomialTree, TrinomialTreeType,
};
pub use parameters::{
    BasisSwapLeg, ContractSpec, CreditParams, EquityOptionParams, EquityUnderlyingParams,
    ExerciseStyle, FinancingLegSpec, FixedLegSpec, FloatLegSpec, FxOptionParams,
    FxUnderlyingParams, IndexUnderlyingParams, InterestRateOptionParams, OptionMarketParams,
    OptionType, ParRateMethod, PayReceive, PremiumLegSpec, ProtectionLegSpec, ScheduleSpec,
    SettlementType, TotalReturnLegSpec, TrsScheduleSpec, TrsSide, UnderlyingParams,
};
pub use period_pv::PeriodizedPvExt;
pub use pricing::{
    GenericDiscountingPricer, GenericInstrumentPricer, TotalReturnLegParams, TrsEngine,
    TrsReturnModel,
};
pub use traits::{Attributes, CurveIdVec, EquityDependencies, EquityInstrumentDeps, Instrument};
