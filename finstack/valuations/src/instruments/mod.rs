//! Financial instruments module: imports and re-exports only.

// Common functionality (traits, macros, models, helpers)
#[macro_use]
pub mod common;

// Flattened instrument modules
/// asian option module.
pub mod asian_option;
/// autocallable module.
pub mod autocallable;
/// barrier option module.
pub mod barrier_option;
/// basis swap module.
pub mod basis_swap;
/// basket module.
pub mod basket;
/// bond module.
pub mod bond;
/// bond future module.
pub mod bond_future;
/// cap floor module.
pub mod cap_floor;
/// cds module.
pub mod cds;
/// cds index module.
pub mod cds_index;
/// cds option module.
pub mod cds_option;
/// cds tranche module.
pub mod cds_tranche;
/// cliquet option module.
pub mod cliquet_option;
/// cms option module.
pub mod cms_option;
/// commodity forward module.
pub mod commodity_forward;
/// commodity swap module.
pub mod commodity_swap;
/// convertible module.
pub mod convertible;
/// dcf module.
pub mod dcf;
/// deposit module.
pub mod deposit;
/// equity module.
pub mod equity;
/// equity index future module.
pub mod equity_index_future;
/// equity option module.
pub mod equity_option;
/// equity total return swap module.
pub mod equity_trs;
/// fi_trs module (fixed income total return swap).
pub mod fi_trs;
/// fra module.
pub mod fra;
/// fx barrier option module.
pub mod fx_barrier_option;
/// fx option module.
pub mod fx_option;
/// fx spot module.
pub mod fx_spot;
/// fx swap module.
pub mod fx_swap;
/// inflation linked bond module.
pub mod inflation_linked_bond;
/// inflation swap module.
pub mod inflation_swap;
/// ir future module.
pub mod ir_future;
/// irs module.
pub mod irs;
/// lookback option module.
pub mod lookback_option;
/// pricing overrides module.
pub mod pricing_overrides;
/// private markets fund module.
pub mod private_markets_fund;
/// quanto option module.
pub mod quanto_option;
/// range accrual module.
pub mod range_accrual;
/// repo module.
pub mod repo;
/// revolving credit module.
pub mod revolving_credit;
/// structured credit module.
pub mod structured_credit;
/// swaption module.
pub mod swaption;
/// term loan module.
pub mod term_loan;
/// variance swap module.
pub mod variance_swap;
/// volatility index future module.
pub mod vol_index_future;
/// volatility index option module.
pub mod vol_index_option;
/// cross-currency swap module.
pub mod xccy_swap;

// Preserve public path for equity metrics after move
pub use equity::metrics as equity_metrics;

// === Core Instrument Types ===
pub use asian_option::{AsianOption, AveragingMethod};
pub use autocallable::{Autocallable, FinalPayoffType};
pub use barrier_option::{BarrierOption, BarrierType};
pub use basis_swap::BasisSwap;
pub use basket::Basket;
pub use bond::Bond;
pub use bond_future::{BondFuture, BondFutureBuilder, BondFutureSpecs, DeliverableBond};
pub use cap_floor::RateOptionType;
pub use cds::CreditDefaultSwap;
pub use cds_index::CDSIndex;
pub use cds_option::CdsOption;
pub use cds_tranche::CdsTranche;
pub use cliquet_option::CliquetOption;
pub use cms_option::CmsOption;
pub use commodity_forward::CommodityForward;
pub use commodity_swap::CommoditySwap;
pub use convertible::ConvertibleBond;
pub use dcf::{DiscountedCashFlow, TerminalValueSpec};
pub use deposit::Deposit;
pub use equity::Equity;
pub use equity_index_future::{EquityFutureSpecs, EquityIndexFuture};
pub use equity_option::EquityOption;
pub use fra::ForwardRateAgreement;
pub use fx_barrier_option::FxBarrierOption;
pub use fx_option::FxOption;
pub use fx_spot::FxSpot;
pub use fx_swap::FxSwap;
pub use inflation_linked_bond::InflationLinkedBond;
pub use inflation_swap::{InflationSwap, YoYInflationSwap};
pub use ir_future::InterestRateFuture;
pub use irs::InterestRateSwap;
pub use lookback_option::{LookbackOption, LookbackType};
pub use pricing_overrides::PricingOverrides;
pub use private_markets_fund::PrivateMarketsFund;
pub use quanto_option::QuantoOption;
pub use range_accrual::RangeAccrual;
pub use repo::{CollateralSpec, CollateralType, Repo, RepoType};
pub use revolving_credit::RevolvingCredit;
pub use structured_credit::StructuredCredit;
pub use swaption::Swaption;
pub use term_loan::TermLoan;
pub use equity_trs::EquityTotalReturnSwap;
pub use fi_trs::FIIndexTotalReturnSwap;
// Re-export TRS common types
pub use common::parameters::trs_common::{TrsScheduleSpec, TrsSide};
pub use variance_swap::VarianceSwap;
pub use vol_index_future::{VolatilityIndexFuture, VolIndexContractSpecs};
pub use vol_index_option::{VolatilityIndexOption, VolIndexOptionSpecs};
pub use xccy_swap::XccySwap;

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

// === JSON Import/Export ===
#[cfg(feature = "serde")]
pub mod json_loader;

#[cfg(feature = "serde")]
pub use json_loader::{InstrumentEnvelope, InstrumentJson};
