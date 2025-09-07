//! Forward Rate Agreement (FRA) instrument implementation.
//!
//! FRAs are essential for short-end interest rate curve calibration,
//! providing forward rate fixings between deposit maturities and swap start dates.

pub mod metrics;
mod types;
mod builder;

pub use types::ForwardRateAgreement;
