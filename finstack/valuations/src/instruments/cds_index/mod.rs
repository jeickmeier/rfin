//! CDS Index module: structure, pricing, and metrics.
//!
//! Layout mirrors other instruments (e.g., `cds`, `irs`):
//! - `types`: instrument data structures and trait impls
//! - `pricer`: pricing implementation and engine
//! - `metrics`: metric calculators and registry hook

pub mod metrics;
pub mod parameters;
pub mod pricer;
mod types;

pub use types::CDSIndex;
pub use types::CDSIndexConstituent;
pub use types::IndexPricing;
