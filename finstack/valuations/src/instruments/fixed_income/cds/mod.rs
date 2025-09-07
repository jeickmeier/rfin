//! CDS instrument module: submodules and type re-export.

pub mod builder;
pub mod cds_pricer;
pub mod metrics;
mod types;

pub use types::CDSConvention;
pub use types::CreditDefaultSwap;
pub use types::PayReceive;
pub use types::PremiumLegSpec;
pub use types::ProtectionLegSpec;
pub use types::SettlementType;

// Provide a distinct path for types.rs to reference this builder
pub(crate) mod mod_cds {
    pub use super::builder::CDSBuilder;
}
