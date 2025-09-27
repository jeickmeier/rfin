//! Financial instruments module: imports and re-exports only.

// Common functionality (traits, macros, models, helpers)
#[macro_use]
pub mod common;

// Flattened instrument modules
pub mod abs;
pub mod basis_swap;
pub mod basket;
pub mod bond;
pub mod cap_floor;
pub mod cds;
pub mod cds_index;
pub mod cds_option;
pub mod cds_tranche;
pub mod clo;
pub mod cmbs;
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
pub mod rmbs;
pub mod swaption;
pub mod trs;
pub mod variance_swap;

// Preserve public path for equity metrics after move
pub use equity::metrics as equity_metrics;

// === Core Instrument Types ===
pub use abs::Abs;
pub use basis_swap::BasisSwap;
pub use basket::Basket;
pub use bond::Bond;
pub use cap_floor::RateOptionType;
pub use cds::CreditDefaultSwap;
pub use cds_index::CDSIndex;
pub use cds_option::CdsOption;
pub use cds_tranche::CdsTranche;
pub use clo::Clo;
pub use cmbs::Cmbs;
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
pub use rmbs::Rmbs;
pub use swaption::Swaption;
pub use trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap};
pub use variance_swap::VarianceSwap;

// === Common Functionality ===
pub use common::build_with_metrics_dyn;
pub use common::discountable::Discountable;
pub use common::traits::{Attributes, Instrument};
pub use common::{BinomialTree, TreeType};

// === Parameter Types ===
pub use common::parameters::{
    BasisSwapLeg, ContractSpec, CreditParams, EquityOptionParams, EquityUnderlyingParams,
    ExerciseStyle, FinancingLegSpec, FixedLegSpec, FloatLegSpec, FxOptionParams,
    FxUnderlyingParams, IndexUnderlyingParams, InterestRateOptionParams, OptionMarketParams,
    OptionType, ParRateMethod, PayReceive, PremiumLegSpec, ProtectionLegSpec, ScheduleSpec,
    SettlementType, TotalReturnLegSpec, UnderlyingParams,
};
