//! Common parameter types organized by purpose.
//!
//! This module provides shared parameter types used across multiple instruments:
//! - **underlying**: Underlying asset parameters (FX, equity, index)
//! - **legs**: Leg specifications for swaps and structured products
//! - **market**: Market-specific parameters for options and derivatives
//! - **contract**: Contract specifications and general types
//! - **conventions**: Standard market conventions for bonds and swaps

pub mod contract;
pub mod conventions;
pub mod legs;
pub mod market;
pub mod option_market;
pub mod underlying;

// Re-export commonly used types for convenience
pub use contract::{ContractSpec, ScheduleSpec};
pub use conventions::{BondConvention, IRSConvention};
pub use legs::{
    BasisSwapLeg, FinancingLegSpec, FixedLegSpec, FloatLegSpec, ParRateMethod, PayReceive,
    PremiumLegSpec, ProtectionLegSpec, TotalReturnLegSpec,
};
pub use market::{
    CreditParams, EquityOptionParams, ExerciseStyle, FxOptionParams, InterestRateOptionParams,
    OptionType, SettlementType,
};
pub use option_market::OptionMarketParams;
pub use underlying::{
    EquityUnderlyingParams, FxUnderlyingParams, IndexUnderlyingParams, UnderlyingParams,
};
