//! CDS instrument module: structure, pricing, and metrics.
//!
//! Follows the standard instrument layout used across valuations:
//! - `types`: instrument data structures and trait impls
//! - `pricer`: pricing implementation and engine
//! - `metrics`: metric calculators and registry hook

pub mod metrics;
pub mod parameters;
pub mod pricer;
mod types;

pub use types::CDSConvention;
pub use types::CreditDefaultSwap;
pub use types::PayReceive;
pub use types::PremiumLegSpec;
pub use types::ProtectionLegSpec;

// Back-compat: re-export pricing types at the same path names used previously.
// External code that referenced `cds_pricer::CDSPricer` will continue to work via this alias.
pub use pricer as cds_pricer;
