mod wrapper;

mod basis_swap;
mod bond;
mod cap_floor;
mod cds;
mod cds_index;
mod cds_option;
mod cds_tranche;
mod convertible;
mod deposit;
mod equity;
mod equity_option;
mod fra;
mod fx;
mod inflation_linked_bond;
mod inflation_swap;
mod ir_future;
mod irs;
mod private_markets_fund;
mod repo;
mod structured;
mod swaption;
mod trs;
mod variance_swap;

// Re-export wrapper trait for internal use
pub(crate) use wrapper::InstrumentWrapper;

pub use basis_swap::JsBasisSwap as BasisSwap;
pub use bond::JsBond as Bond;
pub use cap_floor::JsInterestRateOption as InterestRateOption;
pub use cds::JsCreditDefaultSwap as CreditDefaultSwap;
pub use cds_index::JsCDSIndex as CDSIndex;
pub use cds_option::JsCdsOption as CdsOption;
pub use cds_tranche::JsCdsTranche as CdsTranche;
pub use convertible::JsConvertibleBond as ConvertibleBond;
pub use deposit::JsDeposit as Deposit;
pub use equity::JsEquity as Equity;
pub use equity_option::JsEquityOption as EquityOption;
pub use fra::JsForwardRateAgreement as ForwardRateAgreement;
pub use fx::{JsFxOption as FxOption, JsFxSpot as FxSpot, JsFxSwap as FxSwap};
pub use inflation_linked_bond::JsInflationLinkedBond as InflationLinkedBond;
pub use inflation_swap::JsInflationSwap as InflationSwap;
pub use ir_future::JsInterestRateFuture as InterestRateFuture;
pub use irs::JsInterestRateSwap as InterestRateSwap;
pub use private_markets_fund::JsPrivateMarketsFund as PrivateMarketsFund;
pub use repo::JsRepo as Repo;
pub use structured::{
    JsBasket as Basket, JsStructuredCredit as StructuredCredit,
};
pub use swaption::JsSwaption as Swaption;
pub use trs::{
    JsEquityTotalReturnSwap as EquityTotalReturnSwap,
    JsFiIndexTotalReturnSwap as FiIndexTotalReturnSwap,
};
pub use variance_swap::JsVarianceSwap as VarianceSwap;
