//! Fixed income instruments for valuation and risk analysis.
//!
//! This module provides implementations of fixed income instruments including
//! bonds, interest rate swaps, deposits, loans, credit default swaps, and
//! inflation-linked bonds.

pub mod bond;
pub mod cds;
pub mod cds_index;
pub mod cds_tranche;
pub mod convertible;
pub mod deposit;
pub mod discountable;
pub mod fra;
pub mod fx_spot;
pub mod fx_swap;
pub mod inflation_linked_bond;
pub mod inflation_swap;
pub mod irs;
pub mod loan;

// Re-export all fixed income instrument types
pub use bond::Bond;
pub use cds::CreditDefaultSwap;
pub use cds_index::CDSIndex;
pub use cds_tranche::CdsTranche;
pub use convertible::ConvertibleBond;
pub use deposit::Deposit;
pub use discountable::Discountable;
pub use fra::{ForwardRateAgreement, InterestRateFuture};
pub use fx_spot::FxSpot;
pub use fx_swap::FxSwap;
pub use inflation_linked_bond::InflationLinkedBond;
pub use inflation_swap::InflationSwap;
pub use irs::InterestRateSwap;
pub use loan::Loan;
