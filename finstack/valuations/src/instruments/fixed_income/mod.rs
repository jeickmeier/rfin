//! Fixed income instruments: bonds, loans, MBS, and structured products.

/// Bond module - Fixed and floating rate bonds.
pub mod bond;
/// Bond future module - Bond futures contracts.
pub mod bond_future;
/// CMO module - Collateralized Mortgage Obligations (renamed from agency_cmo).
pub mod cmo;
/// Convertible bond module.
pub mod convertible;
/// Dollar roll module - MBS dollar rolls.
pub mod dollar_roll;
/// Fixed income TRS module - Fixed income index total return swaps.
pub mod fi_trs;
/// Inflation-linked bond module.
pub mod inflation_linked_bond;
/// MBS passthrough module - Agency MBS pass-throughs (renamed from agency_mbs_passthrough).
pub mod mbs_passthrough;
/// Revolving credit facility module.
pub mod revolving_credit;
/// Structured credit module - ABS, RMBS, CMBS, CLO.
pub mod structured_credit;
/// TBA module - To Be Announced trades (renamed from agency_tba).
pub mod tba;
/// Term loan module.
pub mod term_loan;

// Re-export primary types
pub use bond::Bond;
pub use bond_future::{BondFuture, BondFutureBuilder, BondFutureSpecs, DeliverableBond};
pub use cmo::{AgencyCmo, CmoTranche, CmoTrancheType, CmoWaterfall};
pub use convertible::ConvertibleBond;
pub use dollar_roll::DollarRoll;
pub use fi_trs::FIIndexTotalReturnSwap;
pub use inflation_linked_bond::InflationLinkedBond;
pub use mbs_passthrough::{AgencyMbsPassthrough, AgencyProgram, PoolType};
pub use revolving_credit::RevolvingCredit;
pub use structured_credit::StructuredCredit;
pub use tba::{AgencyTba, TbaTerm};
pub use term_loan::TermLoan;
