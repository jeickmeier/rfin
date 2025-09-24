//! Backward compatibility re-exports for the `instruments` module.
//!
//! These re-exports will be removed in a future major version.
//! Please use the direct module paths instead:
//! - `common::discountable::Discountable`
//! - `common::traits::{Attributable, Attributes, Instrument}`
//! - `common::parameters::*`

#[deprecated(
    since = "1.0.0",
    note = "Use common::discountable::Discountable directly"
)]
pub use super::common::discountable::Discountable;

#[deprecated(
    since = "1.0.0",
    note = "Use common::traits::{Attributable, Attributes, Instrument} directly"
)]
pub use super::common::traits::{Attributable, Attributes, Instrument};

// Parameter type re-exports for backward compatibility
#[deprecated(since = "1.0.0", note = "Use common::parameters::* directly")]
pub use super::common::parameters::{
    // Leg specifications
    BasisSwapLeg,
    CdsSettlementType,
    // Contract specifications
    ContractSpec,
    // Market parameters
    CreditParams,
    EquityOptionParams,
    // Underlying parameters
    EquityUnderlyingParams,
    // Option types (clean versions from parameters, not models)
    ExerciseStyle,
    FinancingLegSpec,
    FixedLegSpec,
    FloatLegSpec,
    FxOptionParams,
    FxUnderlyingParams,
    IndexUnderlyingParams,
    InterestRateOptionParams,
    OptionMarketParams,
    OptionType,
    ParRateMethod,
    PayReceive,
    PremiumLegSpec,
    ProtectionLegSpec,
    ScheduleSpec,
    SettlementType,
    TotalReturnLegSpec,
    UnderlyingParams,
};

// Direct module access for compatibility (kept without deprecation for now)
pub use super::common::discountable;
pub use super::common::macros;
pub use super::common::parameters;
pub use super::common::traits;
