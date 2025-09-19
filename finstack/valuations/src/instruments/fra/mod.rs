//! Forward Rate Agreement (FRA) instrument module.
//!
//! Provides a modern, deposit-like layout with clear separation between
//! instrument types, pricing engine, and metrics. FRAs are key short-end
//! instruments that quote forward rates between a start and end date with
//! settlement at the start of the accrual period.

pub mod metrics;
pub mod pricing;
mod types;

pub use types::ForwardRateAgreement;
pub use pricing::engine::FraEngine;
