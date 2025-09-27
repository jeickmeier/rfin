//! Forward Rate Agreement (FRA) instrument module.
//!
//! Provides a modern layout with clear separation between instrument types,
//! pricing implementation, and metrics. FRAs are key short-end instruments
//! that quote forward rates between a start and end date with settlement
//! at the start of the accrual period.

pub mod metrics;
pub mod pricer;
mod types;

pub use types::ForwardRateAgreement;
