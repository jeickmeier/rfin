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
pub(crate) mod traits;

// Unified dependency representation
pub(crate) mod dependencies;

// NPV calculation interface
pub(crate) mod discountable;

// Shared utilities and helper functions
pub(crate) mod helpers;

// Common parameter types shared across instruments
pub(crate) mod fx_dates;
pub(crate) mod parameters;

// Option pricing models and frameworks (includes closed-form, volatility, and tree models)
#[doc(hidden)]
pub mod models;

// Monte Carlo pricing engine (requires mc feature)
#[cfg(feature = "mc")]
#[doc(hidden)]
pub mod mc;

// Common pricing patterns and infrastructure
pub(crate) mod pricing;

// Periodized present value calculations
pub(crate) mod period_pv;

// Re-export commonly used types for convenience
#[doc(hidden)]
pub use dependencies::{FxPair, InstrumentDependencies};
#[doc(hidden)]
pub use discountable::Discountable;
/// Re-export resolve_calendar for backward compatibility with tests.
#[doc(hidden)]
pub use finstack_core::dates::fx::resolve_calendar;
#[doc(hidden)]
pub use fx_dates::{
    add_joint_business_days, add_joint_business_days_with_calendars, adjust_joint_calendar,
    adjust_joint_calendar_with_calendars, roll_spot_date, ResolvedCalendarPair,
};
#[doc(hidden)]
pub use helpers::validate_currency_consistency;
#[doc(hidden)]
pub use models::{
    d1, d1_d2, d1_d2_black76, d2, norm_cdf, norm_pdf, short_rate_keys, single_factor_equity_state,
    state_keys, two_factor_equity_rates_state, BinomialTree, EvolutionParams, NodeState,
    SABRCalibrator, SABRModel, SABRParameters, SABRSmile, ShortRateModel, ShortRateTree,
    ShortRateTreeConfig, StateVariables, TreeBranching, TreeGreeks, TreeModel, TreeParameters,
    TreeType, TreeValuator, TrinomialTree, TrinomialTreeType,
};
pub use parameters::{
    BasisSwapLeg, BondConvention, ContractSpec, CreditParams, EquityOptionParams,
    EquityUnderlyingParams, ExerciseStyle, FinancingLegSpec, FixedLegSpec, FloatLegSpec,
    FxOptionParams, FxUnderlyingParams, IRSConvention, IndexUnderlyingParams,
    InterestRateOptionParams, OptionMarketParams, OptionType, ParRateMethod, PayReceive,
    PremiumLegSpec, ProtectionLegSpec, ScheduleSpec, SettlementType, TotalReturnLegSpec,
    TrsScheduleSpec, TrsSide, UnderlyingParams,
};
#[doc(hidden)]
pub use period_pv::PeriodizedPvExt;
#[doc(hidden)]
pub use pricing::{
    GenericInstrumentPricer, HasDiscountCurve, HasForwardCurves, TotalReturnLegParams, TrsEngine,
    TrsReturnModel,
};
pub use traits::{
    Attributes, CurveIdVec, EquityDependencies, EquityInstrumentDeps, Instrument, PricingOptions,
};
