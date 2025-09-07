//! Interest rate swap module: submodules and type re-export.

pub mod metrics;
mod types;
mod builder;

pub use types::{FixedLegSpec, FloatLegSpec, InterestRateSwap, PayReceive};

// Provide a distinct path for types.rs to reference this builder
pub(crate) mod mod_irs {
    pub use super::builder::IRSBuilder;
}
