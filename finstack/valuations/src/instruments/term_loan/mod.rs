//! Term loan instruments (including Delayed Draw Term Loans).
//!
//! This module defines the `TermLoan` instrument, specs, cashflow engine, and pricing.

pub mod cashflows;
pub mod metrics;
pub mod pricing;
pub mod spec;
pub mod types;

// Re-export main type
pub use types::TermLoan;
pub use spec::{
    TermLoanSpec,
    DdtlSpec,
    OidPolicy,
    PikSpec,
    OidEirSpec,
    CovenantSpec,
    CommitmentFeeBase,
    DrawEvent,
    CommitmentStepDown,
    AmortizationSpec,
    PikToggle,
    CashSweepEvent,
};


