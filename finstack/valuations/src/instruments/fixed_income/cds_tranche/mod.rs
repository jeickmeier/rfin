//! CDS Tranche instrument (boilerplate implementation).
//!
//! A CDS tranche references a standardized credit index (e.g., CDX IG/HY, iTraxx)
//! and a loss layer defined by attachment/detachment points. This module provides
//! a minimal scaffold for the instrument type and wiring to the pricing/metrics
//! framework. Valuation logic is intentionally minimal and returns zero PV in the
//! instrument currency until tranche pricing models are implemented.

pub mod metrics;
pub mod model;
mod types;

pub use types::{CdsTranche, TrancheSide};