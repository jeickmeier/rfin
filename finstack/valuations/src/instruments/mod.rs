//! Financial instruments module: imports and re-exports only.

// Common functionality (traits, macros, models, helpers)
#[macro_use]
pub mod common;

// Flattened instrument modules
pub mod basis_swap;
pub mod basket;
pub mod bond;
pub mod cap_floor;
pub mod cds;
pub mod cds_index;
pub mod cds_option;
pub mod cds_tranche;
pub mod convertible;
pub mod deposit;
pub mod equity;
pub mod equity_option;
pub mod fra;
pub mod fx_option;
pub mod fx_spot;
pub mod fx_swap;
pub mod inflation_linked_bond;
pub mod inflation_swap;
pub mod ir_future;
pub mod irs;
pub mod pricing_overrides;
pub mod private_markets_fund;
pub mod repo;
// Preserve public path for equity underlying params after move
pub use equity::underlying;
// Preserve public path for equity metrics after move
pub use equity::metrics as equity_metrics;
pub mod structured_credit;
pub mod swaption;
pub mod trs;
pub mod variance_swap;

// Re-export common types for convenience (avoid glob re-exports to keep API unambiguous)
pub use basis_swap::BasisSwap;
pub use basket::Basket;
pub use bond::Bond;
pub use cds::CreditDefaultSwap;
pub use cds_index::CDSIndex;
pub use cds_option::CdsOption;
pub use cds_tranche::CdsTranche;
pub use convertible::ConvertibleBond;
pub use deposit::Deposit;
pub use equity::Equity;
pub use equity_option::EquityOption;
pub use fra::ForwardRateAgreement;
pub use fx_option::FxOption;
pub use fx_spot::FxSpot;
pub use fx_swap::FxSwap;
pub use inflation_linked_bond::InflationLinkedBond;
pub use inflation_swap::InflationSwap;
pub use ir_future::InterestRateFuture;
pub use irs::InterestRateSwap;
pub use pricing_overrides::PricingOverrides;
pub use private_markets_fund::PrivateMarketsFund;
pub use repo::{CollateralSpec, CollateralType, Repo, RepoType};
pub use structured_credit::{Abs, Clo, StructuredCredit};
pub use swaption::Swaption;
pub use trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap};

// Re-export option-related enums and models at top-level after flattening
pub use cap_floor::RateOptionType;
pub use common::{BinomialTree, TreeType};

pub use common::build_with_metrics_dyn;

// ==============================================================================
// DEPRECATED: Backward compatibility re-exports
//
// These re-exports will be removed in a future major version.
// Please use the direct module paths instead:
// - `common::discountable::Discountable`
// - `common::traits::{Attributable, Attributes, Instrument}`
// - `common::parameters::*`
// ==============================================================================

#[deprecated(
    since = "1.0.0",
    note = "Use common::discountable::Discountable directly"
)]
pub use common::discountable::Discountable;

#[deprecated(
    since = "1.0.0",
    note = "Use common::traits::{Attributable, Attributes, Instrument} directly"
)]
pub use common::traits::{Attributable, Attributes, Instrument};

// Parameter type re-exports for backward compatibility
#[deprecated(since = "1.0.0", note = "Use common::parameters::* directly")]
pub use common::parameters::{
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
pub use common::discountable;
pub use common::macros;
pub use common::parameters;
pub use common::traits;

/// Install default pricers for core instruments so that `Instrument::value` delegates
/// to registered pricing engines out of the box.
pub fn install_default_pricers() {
    // Bond discounting pricer
    crate::instruments::bond::pricing::register_default_bond_pricers();
    // IRS discounting pricer
    crate::instruments::irs::pricing::register_default_irs_pricers();
}
