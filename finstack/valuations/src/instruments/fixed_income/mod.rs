//! Fixed income instruments for valuation and risk analysis.
//!
//! This module provides implementations of fixed income instruments including
//! bonds, interest rate swaps, deposits, loans, credit default swaps, and
//! inflation-linked bonds.

pub mod bond;
pub mod cds;
pub mod deposit;
pub mod ilb;
pub mod irs;
pub mod loan;

// Re-export all fixed income instrument types
pub use bond::Bond;
pub use cds::CreditDefaultSwap;
pub use deposit::Deposit;
pub use ilb::InflationLinkedBond;
pub use irs::InterestRateSwap;
pub use loan::Loan;
