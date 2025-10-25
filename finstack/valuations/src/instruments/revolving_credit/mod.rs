//! Revolving Credit Facility instrument module.
//!
//! Exposes the core `RevolvingCreditFacility` type, pricing helpers, and
//! metric registrations for revolving credit facilities (RCFs). The
//! implementation mirrors the modern instrument layout used across the
//! valuations crate (types + pricer + metrics) and is designed for future
//! Monte Carlo extensions.

pub mod metrics;
mod pricer;
mod types;

pub use types::{
    InterestRateSpec, RcfFeeSpec, RcfTransaction, RevolvingCreditFacility, TransactionType,
    ResetConvention,
};

