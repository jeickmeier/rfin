//! CDS instrument module: submodules and type re-export.

pub mod cds_pricer;
pub mod metrics;
mod types;
pub mod parameters;
pub mod credit;

pub use types::CDSConvention;
pub use types::CreditDefaultSwap;
pub use types::PayReceive;
pub use types::PremiumLegSpec;
pub use types::ProtectionLegSpec;
pub use types::SettlementType;
pub use credit::CreditParams;
