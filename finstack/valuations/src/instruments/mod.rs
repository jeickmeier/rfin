//! Financial instruments module: imports and re-exports only.

// Common functionality (traits, macros, models, helpers)
#[macro_use]
pub mod common;

// === Category Modules ===
/// Commodity derivatives.
pub mod commodity;
/// Credit derivatives: CDS and related instruments.
pub mod credit_derivatives;
/// Equity instruments and equity derivatives.
pub mod equity;
/// Exotic and path-dependent options.
pub mod exotics;
/// Fixed income instruments: bonds, loans, MBS, and structured products.
pub mod fixed_income;
/// FX instruments and FX derivatives.
pub mod fx;
/// Interest rate derivatives and money market instruments.
pub mod rates;

// These allow existing code using `instruments::bond`, `instruments::irs`, etc. to continue working.

// Fixed Income
pub use fixed_income::bond;
pub use fixed_income::bond_future;
pub use fixed_income::cmo;
pub use fixed_income::convertible;
pub use fixed_income::dollar_roll;
pub use fixed_income::fi_trs;
pub use fixed_income::inflation_linked_bond;
pub use fixed_income::mbs_passthrough;
pub use fixed_income::revolving_credit;
pub use fixed_income::structured_credit;
pub use fixed_income::tba;
pub use fixed_income::term_loan;

// Rates
pub use rates::basis_swap;
pub use rates::cap_floor;
pub use rates::cms_option;
pub use rates::deposit;
pub use rates::fra;
pub use rates::inflation_swap;
pub use rates::ir_future;
pub use rates::irs;
pub use rates::range_accrual;
pub use rates::repo;
pub use rates::swaption;
pub use rates::xccy_swap;

// Credit Derivatives
pub use credit_derivatives::cds;
pub use credit_derivatives::cds_index;
pub use credit_derivatives::cds_option;
pub use credit_derivatives::cds_tranche;

// Equity
pub use equity::autocallable;
pub use equity::cliquet_option;
pub use equity::dcf_equity;
pub use equity::equity_index_future;
pub use equity::equity_option;
pub use equity::equity_trs;
pub use equity::pe_fund;
pub use equity::spot as equity_spot;
pub use equity::variance_swap;
pub use equity::vol_index_future;
pub use equity::vol_index_option;

// FX
pub use fx::fx_barrier_option;
pub use fx::fx_forward;
pub use fx::fx_option;
pub use fx::fx_spot;
pub use fx::fx_swap;
pub use fx::ndf;
pub use fx::quanto_option;

// Commodity
pub use commodity::commodity_forward;
pub use commodity::commodity_swap;

// Exotics
pub use exotics::asian_option;
pub use exotics::barrier_option;
pub use exotics::basket;
pub use exotics::lookback_option;

pub use equity::dcf_equity as dcf;
pub use equity::pe_fund as private_markets_fund;
pub use fixed_income::cmo as agency_cmo;
pub use fixed_income::mbs_passthrough as agency_mbs_passthrough;
pub use fixed_income::tba as agency_tba;

pub use equity::equity_trs as trs;

pub use equity::equity_metrics;

// === Core Instrument Types ===
// Fixed Income
pub use fixed_income::{
    AgencyCmo, AgencyMbsPassthrough, AgencyProgram, AgencyTba, Bond, BondFuture, BondFutureBuilder,
    BondFutureSpecs, CmoTranche, CmoTrancheType, CmoWaterfall, ConvertibleBond, DeliverableBond,
    DollarRoll, FIIndexTotalReturnSwap, InflationLinkedBond, PoolType, RevolvingCredit,
    StructuredCredit, TbaTerm, TermLoan,
};

// Rates
pub use rates::{
    BasisSwap, CmsOption, CollateralSpec, CollateralType, Deposit, ForwardRateAgreement,
    InflationSwap, InterestRateFuture, InterestRateSwap, RangeAccrual, RateOptionType, Repo,
    RepoType, Swaption, XccySwap, YoYInflationSwap,
};

// Credit Derivatives
pub use credit_derivatives::{CDSIndex, CdsOption, CdsTranche, CreditDefaultSwap};

// Equity
pub use equity::{
    Autocallable, CliquetOption, DiscountedCashFlow, Equity, EquityFutureSpecs, EquityIndexFuture,
    EquityOption, EquityTotalReturnSwap, FinalPayoffType, PrivateMarketsFund, TerminalValueSpec,
    VarianceSwap, VolIndexContractSpecs, VolIndexOptionSpecs, VolatilityIndexFuture,
    VolatilityIndexOption,
};

// FX
pub use fx::{FxBarrierOption, FxForward, FxOption, FxSpot, FxSwap, Ndf, QuantoOption};

// Commodity
pub use commodity::{CommodityForward, CommoditySwap};

// Exotics
pub use exotics::{
    AsianOption, AveragingMethod, BarrierOption, BarrierType, Basket, LookbackOption, LookbackType,
};

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

// Re-export TRS common types
pub use common::parameters::trs_common::{TrsScheduleSpec, TrsSide};

/// Pricing overrides module.
pub mod pricing_overrides;
pub use pricing_overrides::PricingOverrides;

// === JSON Import/Export ===
#[cfg(feature = "serde")]
pub mod json_loader;

#[cfg(feature = "serde")]
pub use json_loader::{InstrumentEnvelope, InstrumentJson};
