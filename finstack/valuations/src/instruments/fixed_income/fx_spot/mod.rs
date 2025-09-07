//! FX Spot module: submodules and type re-export.

pub mod metrics;
pub mod builder;
mod types;

pub use types::FxSpot;

// Provide a distinct path for types.rs to reference this builder
pub(crate) mod mod_fx_spot {
    pub use super::builder::FxSpotBuilder;
}
