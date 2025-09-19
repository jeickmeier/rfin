//! CDS instrument module: submodules and type re-export.

pub mod cds_pricer;
pub mod credit;
pub mod metrics;
pub mod parameters;
mod types;

pub use credit::CreditParams;
pub use types::CDSConvention;
pub use types::CreditDefaultSwap;
pub use types::PayReceive;
pub use types::PremiumLegSpec;
pub use types::ProtectionLegSpec;
pub use types::SettlementType;
