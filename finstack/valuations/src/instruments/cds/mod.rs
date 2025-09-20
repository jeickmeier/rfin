//! CDS instrument module: structure, pricing, and metrics.
//!
//! Follows the standard instrument layout used across valuations:
//! - `types`: instrument data structures and trait impls
//! - `pricing`: pricing facade and engine implementation
//! - `metrics`: metric calculators and registry hook

pub mod pricing;
pub mod parameters;
pub mod metrics;
mod types;

pub use parameters::CreditParams;
pub use types::CDSConvention;
pub use types::CreditDefaultSwap;
pub use types::PayReceive;
pub use types::PremiumLegSpec;
pub use types::ProtectionLegSpec;
pub use types::SettlementType;

// Back-compat: re-export pricing engine types at the same path names used previously.
// External code that referenced `cds_pricer::CDSPricer` will continue to work via this alias.
pub use pricing::engine as cds_pricer;
