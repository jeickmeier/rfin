//! CDS Index module: submodules and type re-export.

pub mod metrics;
pub mod builder;
mod types;

pub use types::CDSIndex;

// Provide a distinct path for types.rs to reference this builder (parity with CDS/IRS)
pub(crate) mod mod_cds_index {
    pub use super::builder::CDSIndexBuilder;
}
