//! Interest rate swap module: submodules and type re-export.

pub mod metrics;
mod types;

pub use types::{FixedLegSpec, FloatLegSpec, InterestRateSwap, PayReceive};

// Builder provided by FinancialBuilder derive
