//! Financial instruments module: imports and re-exports only.

// Macro infrastructure for reducing boilerplate
#[macro_use]
pub mod macros;

// Instrument-level traits and metadata
pub mod traits;

// Flattened instrument modules
pub mod basis_swap;
pub mod basket;
pub mod bond;
pub mod cap_floor;
pub mod cds;
pub mod cds_index;
pub mod cds_tranche;
pub mod convertible;
pub mod credit_option;
pub mod deposit;
pub mod discountable;
pub mod equity_option;
pub mod fra;
pub mod fx;
pub mod fx_option;
pub mod fx_spot;
pub mod fx_swap;
pub mod inflation_linked_bond;
pub mod inflation_swap;
pub mod ir_future;
pub mod irs;
pub mod loan;
pub mod models;
pub mod pricing_overrides;
pub mod private_equity;
pub mod repo;
pub mod equity;
// Preserve public path for equity underlying params after move
pub use equity::underlying;
// Preserve public path for equity metrics after move
pub use equity::equity_metrics;
pub mod structured_credit;
pub mod swaption;
pub mod trs;
pub mod variance_swap;
pub mod helpers;

// Re-export common types for convenience (avoid glob re-exports to keep API unambiguous)
pub use basket::Basket;
pub use bond::Bond;
pub use basis_swap::{BasisSwap, BasisSwapLeg};
pub use pricing_overrides::PricingOverrides;
pub use cds::CreditDefaultSwap;
pub use cds::CreditParams;
pub use cds_index::CDSIndex;
pub use cds_tranche::CdsTranche;
pub use convertible::ConvertibleBond;
pub use credit_option::CreditOption;
pub use deposit::Deposit;
pub use discountable::Discountable;
pub use equity_option::EquityOption;
pub use fra::ForwardRateAgreement;
pub use fx::FxUnderlyingParams;
pub use fx_option::FxOption;
pub use fx_spot::FxSpot;
pub use fx_swap::FxSwap;
pub use inflation_linked_bond::InflationLinkedBond;
pub use inflation_swap::InflationSwap;
pub use ir_future::InterestRateFuture;
pub use irs::InterestRateSwap;
pub use loan::Loan;
pub use private_equity::PrivateEquityInvestment;
pub use repo::{Repo, CollateralSpec, CollateralType, RepoType};
pub use equity::Equity;
pub use structured_credit::{Abs, Clo, StructuredCredit};
pub use swaption::Swaption;
pub use trs::{EquityTotalReturnSwap, FIIndexTotalReturnSwap};

// Re-export option-related enums and models at top-level after flattening
pub use cap_floor::RateOptionType;
pub use models::{BinomialTree, ExerciseStyle, OptionType, SettlementType, TreeType};

pub use crate::metrics::{RiskMeasurable, RiskReport};
pub use traits::{Attributes, Instrument};
pub use helpers::build_with_metrics_dyn;
