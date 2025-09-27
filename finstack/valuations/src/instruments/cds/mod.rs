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

// Note: previously re-exported `pricer` as `cds_pricer` for backward compatibility.
// This alias has been removed to simplify the public surface; dependents should import from `cds::pricer`.
