//! Fixed income instruments for valuation and risk analysis.
//!
//! This module provides implementations of fixed income instruments including
//! bonds, interest rate swaps, deposits, loans, credit default swaps, and
//! inflation-linked bonds.

pub mod bond;
pub mod irs;
pub mod deposit;
pub mod loan;
pub mod cds;
pub mod ilb;

// Re-export all fixed income instrument types
pub use bond::Bond;
pub use irs::InterestRateSwap;
pub use deposit::Deposit;
pub use loan::Loan;
pub use cds::CreditDefaultSwap;
pub use ilb::InflationLinkedBond;
