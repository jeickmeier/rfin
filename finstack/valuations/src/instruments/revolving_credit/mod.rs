//! Revolving credit facility instrument.
//!
//! This module provides a comprehensive revolving credit implementation supporting:
//! - Deterministic draw/repayment schedules
//! - Stochastic utilization modeling via Monte Carlo
//! - Standard fee structures (upfront, commitment, usage, facility)
//! - Fixed and floating rate bases
//! - Full metrics (PV, DV01, Theta, BucketedDV01, CS01, plus facility-specific)

pub mod cashflows;
pub mod metrics;
pub mod pricer;
pub mod types;

// Re-export main types
pub use types::{
    BaseRateSpec, DrawRepayEvent, DrawRepaySpec, RevolvingCredit, RevolvingCreditFees,
    StochasticUtilizationSpec, UtilizationProcess,
};
